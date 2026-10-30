import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useCallback,
  useRef,
  useState,
} from "react";
import type { GenerationParams, OutputMode } from "../components/layout/PromptPanel";
import { rememberRun } from "../lib/debugReport";
import { blankEdit, canStartEdit, type EditPayload, generationRequest } from "../lib/edit";
import { applyEditResult } from "../lib/imageVersions";
import {
  type MeshOrigin,
  type MeshPick,
  type MeshView,
  mergeViews,
  meshViewsOf,
  pickForPath,
} from "../lib/meshViews";
import { explicitBackend, paramsForRun, presetFor } from "../lib/modelPreset";
import {
  type GalleryImage,
  type GenerationProgress,
  generateImage,
  imageSrc,
  isCancelled,
} from "../native/generation";
import { meshFromImage, meshOptimize } from "../native/mesh";
import type { PythonIntent } from "../native/pythonEnv";

export type Frame = {
  step: number;
  total: number;
  preview: string | null;
  image: number;
  images: number;
};

function frameFrom(progress: GenerationProgress, current: Frame | null): Frame {
  return {
    step: progress.step,
    total: progress.total,
    preview: progress.preview ?? current?.preview ?? null,
    image: (progress.index ?? 0) + 1,
    images: progress.count ?? 1,
  };
}

export type MeshPaths = {
  glb: string;
  obj: string;
  optimizedGlb?: string | null;
  optimizedObj?: string | null;
  optimizedFbx?: string | null;
  references?: string[];
};

type GenerationRequestOptions = {
  generatingRef: MutableRefObject<boolean>;
  removing: boolean;
  editRef: MutableRefObject<EditPayload>;
  holdEdit: () => void;
  commitEdit: () => void;
  discardEdit: () => void;
  restoreEdit: () => void;
  setGenerating: (value: boolean) => void;
  setSlides: Dispatch<SetStateAction<GalleryImage[]>>;
  setSlideIndex: Dispatch<SetStateAction<number>>;
  setFrame: Dispatch<SetStateAction<Frame | null>>;
  setHistoryRevision: Dispatch<SetStateAction<number>>;
  params: GenerationParams;
  prompt: string;
  negativePrompt: string;
  modelId: string;
  reportFailure: (error: unknown) => void;
  outputMode: OutputMode;
  prepareEnvironment: () => Promise<void>;
  noteIntent: (reason: PythonIntent) => Promise<void>;
  setWaitingEnvironment: (value: boolean) => void;
  onImageBackend?: (backend: string) => void;
  imageBackend?: string;
  slides: GalleryImage[];
  slideIndex: number;
};

type GenerationActions = {
  request: (seedOverride?: number) => void;
  sendImage: (path: string) => void;
  meshFiles: MeshPaths | null;
  meshOrigin: MeshOrigin;
  setMeshOrigin: (origin: MeshOrigin) => void;
  meshPick: MeshPick | null;
  confirmMesh: (views: MeshView[], primaryPath: string) => void;
  cancelMesh: () => void;
  addMeshPaths: (paths: string[]) => void;
  addCurrentViews: () => void;
  canAddCurrent: boolean;
  pendingPrompt: string | null;
};

function backendFor(
  modelId: string,
  onImageBackend?: (backend: string) => void,
): string | undefined {
  const backend = explicitBackend(modelId);
  if (backend === null) {
    return undefined;
  }
  onImageBackend?.(backend);
  return backend;
}

function noteRun(
  modelId: string,
  backend: string,
  mode: string,
  edit: string,
  params: GenerationParams,
  seed: number | null,
): void {
  rememberRun({
    modelId,
    family: presetFor(modelId)?.family ?? "desconhecida",
    backend,
    mode,
    edit,
    width: params.width,
    height: params.height,
    steps: params.steps,
    cfg: params.cfg,
    seed: seed === null ? "aleatória" : String(seed),
    count: params.count,
    resolution: params.resolution,
    aspect: params.aspect,
  });
}

function chosenSeed(typed: string, override?: number): number | null {
  if (override !== undefined) {
    return override;
  }
  if (typed.trim() === "") {
    return null;
  }
  const value = Number(typed);
  return Number.isFinite(value) ? value : null;
}

export function useGenerationRequest(options: GenerationRequestOptions): GenerationActions {
  const [meshFiles, setMeshFiles] = useState<MeshPaths | null>(null);
  const [meshOrigin, setMeshOrigin] = useState<MeshOrigin>("text");
  const [meshPick, setMeshPick] = useState<MeshPick | null>(null);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const meshPickRef = useRef<MeshPick | null>(null);
  const prepareRef = useRef(false);
  const pickToken = useRef(0);
  const {
    generatingRef,
    removing,
    editRef,
    holdEdit,
    commitEdit,
    discardEdit,
    restoreEdit,
    setGenerating,
    setSlides,
    setSlideIndex,
    setFrame,
    setHistoryRevision,
    params,
    prompt,
    negativePrompt,
    modelId,
    reportFailure,
    outputMode,
    prepareEnvironment,
    noteIntent,
    setWaitingEnvironment,
    onImageBackend,
    imageBackend = "vulkan",
    slides,
    slideIndex,
  } = options;

  const openPick = useCallback((views: MeshView[], primaryPath: string, prepare: boolean) => {
    const first = views[0];
    if (first === undefined) {
      return;
    }
    const primary = views.some((view) => view.path === primaryPath) ? primaryPath : first.path;
    prepareRef.current = prepare;
    pickToken.current += 1;
    const pick = { token: pickToken.current, views, primaryPath: primary };
    meshPickRef.current = pick;
    setMeshPick(pick);
  }, []);

  const cancelMesh = useCallback(() => {
    meshPickRef.current = null;
    setMeshPick(null);
  }, []);

  const showMesh = useCallback(
    async (views: MeshView[], primaryPath: string, seed: number | null) => {
      const mesh = await meshFromImage(
        views.map((view) => view.path),
        primaryPath,
        meshOptimize(params),
      );
      if (mesh === null) {
        return;
      }
      setSlides([{ src: imageSrc(mesh.preview), path: mesh.imagePath, seed, transparent: true }]);
      setSlideIndex(0);
      setMeshFiles({
        glb: mesh.glbPath,
        obj: mesh.objPath,
        optimizedGlb: mesh.optimizedGlb,
        optimizedObj: mesh.optimizedObj,
        optimizedFbx: mesh.optimizedFbx,
        references: mesh.references,
      });
    },
    [params, setSlideIndex, setSlides],
  );

  const confirmMesh = useCallback(
    (views: MeshView[], primaryPath: string) => {
      const primary = views.find((view) => view.path === primaryPath) ?? views[0];
      if (primary === undefined || generatingRef.current || removing) {
        return;
      }
      const prepare = prepareRef.current;
      meshPickRef.current = null;
      setMeshPick(null);
      generatingRef.current = true;
      setGenerating(true);
      setMeshFiles(null);
      const finish = () => {
        generatingRef.current = false;
        setGenerating(false);
      };
      const run = () =>
        showMesh(views, primary.path, primary.seed)
          .catch((error: unknown) => {
            if (!isCancelled(error)) {
              reportFailure(error);
            }
          })
          .finally(finish);
      if (!prepare) {
        void run();
        return;
      }
      void noteIntent("send-to-3d")
        .then(run)
        .catch((error: unknown) => {
          if (!isCancelled(error)) {
            reportFailure(error);
          }
          finish();
        });
    },
    [generatingRef, noteIntent, removing, reportFailure, setGenerating, showMesh],
  );

  const request = useCallback(
    (seedOverride?: number) => {
      if (generatingRef.current || removing || meshPickRef.current !== null) {
        return;
      }
      if (outputMode === "3d" && meshOrigin === "images") {
        const views = meshViewsOf(slides);
        const current = slides[slideIndex]?.path ?? views[0]?.path ?? "";
        openPick(views, current, true);
        return;
      }
      if (outputMode === "3d") {
        const runParams = paramsForRun(params, modelId);
        if (runParams === null) {
          return;
        }
        const backend = backendFor(modelId, onImageBackend);
        generatingRef.current = true;
        setWaitingEnvironment(true);
        setMeshFiles(null);
        void (async () => {
          try {
            await prepareEnvironment();
            setWaitingEnvironment(false);
            setGenerating(true);
            setPendingPrompt(prompt);
            setSlides([]);
            setSlideIndex(0);
            setFrame(null);
            const seed = chosenSeed(params.seed, seedOverride);
            noteRun(modelId, backend ?? imageBackend, outputMode, "nenhuma", runParams, seed);
            const image = await generateImage(
              generationRequest(
                prompt,
                negativePrompt,
                modelId,
                runParams,
                seed,
                blankEdit,
                backend,
              ),
              (progress) => {
                setFrame((current) => frameFrom(progress, current));
                if (progress.ready) {
                  setHistoryRevision((current) => current + 1);
                }
              },
            );
            if (image === null || image.images.length === 0) {
              return;
            }
            const views = meshViewsOf(image.images);
            const last = views[views.length - 1];
            if (last === undefined) {
              return;
            }
            setSlides(image.images);
            setSlideIndex(image.images.length - 1);
            setHistoryRevision((current) => current + 1);
            openPick(views, last.path, false);
          } catch (error: unknown) {
            if (isCancelled(error)) {
              setHistoryRevision((current) => current + 1);
            } else {
              reportFailure(error);
            }
          } finally {
            generatingRef.current = false;
            setGenerating(false);
            setPendingPrompt(null);
            setWaitingEnvironment(false);
          }
        })();
        return;
      }
      const edit = editRef.current;
      if (!canStartEdit(edit)) {
        return;
      }
      const runParams = paramsForRun(params, modelId);
      if (runParams === null) {
        return;
      }
      const backend = backendFor(modelId, onImageBackend);
      const editing = edit.mode === "vary" || edit.mode === "inpaint";
      const sourceIndex = slideIndex;
      holdEdit();
      generatingRef.current = true;
      setGenerating(true);
      setPendingPrompt(prompt);
      setMeshFiles(null);
      if (edit.mode !== "vary" && edit.mode !== "inpaint") {
        setSlides([]);
        setSlideIndex(0);
      }
      setFrame(null);
      const seed = chosenSeed(params.seed, seedOverride);
      noteRun(modelId, backend ?? imageBackend, outputMode, edit.mode, runParams, seed);
      void generateImage(
        generationRequest(prompt, negativePrompt, modelId, runParams, seed, edit, backend),
        (progress) => {
          setFrame((current) => frameFrom(progress, current));
          if (progress.ready) {
            setHistoryRevision((current) => current + 1);
          }
          if (
            !editing &&
            progress.ready &&
            progress.preview &&
            progress.path &&
            typeof progress.seed === "number"
          ) {
            const slide = {
              src: imageSrc(progress.preview),
              path: progress.path,
              seed: progress.seed,
            };
            const index = progress.index ?? 0;
            setSlides((current) => {
              const next = current.slice();
              next[index] = slide;
              return next;
            });
            setSlideIndex(index);
          }
        },
      )
        .then((image) => {
          if (image === null || image.images.length === 0) {
            return;
          }
          if (editing) {
            commitEdit();
            setSlides((current) => applyEditResult(current, sourceIndex, image.images).slides);
            setSlideIndex(sourceIndex);
          } else {
            setSlides(image.images);
            setSlideIndex(image.images.length - 1);
          }
          setHistoryRevision((current) => current + 1);
        })
        .catch((error: unknown) => {
          if (isCancelled(error)) {
            setHistoryRevision((current) => current + 1);
            restoreEdit();
            return;
          }
          reportFailure(error);
        })
        .finally(() => {
          discardEdit();
          generatingRef.current = false;
          setGenerating(false);
          setPendingPrompt(null);
        });
    },
    [
      commitEdit,
      discardEdit,
      editRef,
      generatingRef,
      holdEdit,
      modelId,
      imageBackend,
      onImageBackend,
      negativePrompt,
      params,
      prompt,
      removing,
      outputMode,
      prepareEnvironment,
      reportFailure,
      restoreEdit,
      setWaitingEnvironment,
      setFrame,
      setGenerating,
      setHistoryRevision,
      setSlideIndex,
      setSlides,
      meshOrigin,
      openPick,
      slideIndex,
      slides,
    ],
  );

  const addMeshPaths = useCallback(
    (paths: string[]) => {
      if (generatingRef.current || removing) {
        return;
      }
      const incoming = paths.flatMap((path) => {
        const trimmed = path.trim();
        if (trimmed === "") {
          return [];
        }
        const known = slides.find((slide) => slide.path === trimmed);
        return [known ?? { path: trimmed, src: "", seed: null }];
      });
      const unique = mergeViews([], incoming);
      const first = unique[0];
      if (first === undefined) {
        return;
      }
      const open = meshPickRef.current;
      if (open) {
        const views = mergeViews(open.views, unique);
        if (views.length === open.views.length) {
          return;
        }
        const pick = { ...open, views };
        meshPickRef.current = pick;
        setMeshPick(pick);
        return;
      }
      openPick(unique, first.path, true);
    },
    [generatingRef, openPick, removing, slides],
  );

  const addCurrentViews = useCallback(() => {
    addMeshPaths(slides.map((slide) => slide.path));
  }, [addMeshPaths, slides]);

  const canAddCurrent = slides.some(
    (slide) => !meshPick?.views.some((view) => view.path === slide.path),
  );

  const sendImage = useCallback(
    (path: string) => {
      if (generatingRef.current || removing || path === "") {
        return;
      }
      const picked = pickForPath(meshViewsOf(slides), path);
      openPick(picked.views, picked.primaryPath, true);
    },
    [generatingRef, openPick, removing, slides],
  );

  return {
    request,
    sendImage,
    meshFiles,
    meshOrigin,
    setMeshOrigin,
    meshPick,
    confirmMesh,
    cancelMesh,
    addMeshPaths,
    addCurrentViews,
    canAddCurrent,
    pendingPrompt,
  };
}
