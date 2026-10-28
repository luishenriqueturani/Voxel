import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useCallback,
  useRef,
  useState,
} from "react";
import type { GenerationParams, OutputMode } from "../components/layout/PromptPanel";
import { blankEdit, canStartEdit, type EditPayload, generationRequest } from "../lib/edit";
import {
  type MeshOrigin,
  type MeshPick,
  type MeshView,
  meshViewsOf,
  pickForPath,
} from "../lib/meshViews";
import { type GalleryImage, generateImage, imageSrc, isCancelled } from "../native/generation";
import { meshFromImage, meshOptimize } from "../native/mesh";
import type { PythonIntent } from "../native/pythonEnv";

type Frame = {
  step: number;
  total: number;
  preview: string | null;
};

export type MeshPaths = {
  glb: string;
  obj: string;
  optimizedGlb?: string | null;
  optimizedObj?: string | null;
  optimizedFbx?: string | null;
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
};

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
  const meshPickRef = useRef<MeshPick | null>(null);
  const prepareRef = useRef(false);
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
    const pick = { views, primaryPath: primary };
    meshPickRef.current = pick;
    setMeshPick(pick);
  }, []);

  const cancelMesh = useCallback(() => {
    meshPickRef.current = null;
    setMeshPick(null);
  }, []);

  const showMesh = useCallback(
    async (path: string, seed: number | null) => {
      const mesh = await meshFromImage(path, meshOptimize(params));
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
        showMesh(primary.path, primary.seed)
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
        generatingRef.current = true;
        setWaitingEnvironment(true);
        setMeshFiles(null);
        void (async () => {
          try {
            await prepareEnvironment();
            setWaitingEnvironment(false);
            setGenerating(true);
            setSlides([]);
            setSlideIndex(0);
            setFrame(null);
            const seed = chosenSeed(params.seed, seedOverride);
            const image = await generateImage(
              generationRequest(prompt, negativePrompt, modelId, params, seed, blankEdit),
              (progress) => {
                setFrame((current) => ({
                  step: progress.step,
                  total: progress.total,
                  preview: progress.preview ?? current?.preview ?? null,
                }));
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
            if (!isCancelled(error)) {
              reportFailure(error);
            }
          } finally {
            generatingRef.current = false;
            setGenerating(false);
            setWaitingEnvironment(false);
          }
        })();
        return;
      }
      const edit = editRef.current;
      if (!canStartEdit(edit)) {
        return;
      }
      const editing = edit.mode === "vary" || edit.mode === "inpaint";
      holdEdit();
      generatingRef.current = true;
      setGenerating(true);
      setMeshFiles(null);
      if (edit.mode !== "vary" && edit.mode !== "inpaint") {
        setSlides([]);
        setSlideIndex(0);
      }
      setFrame(null);
      const seed = chosenSeed(params.seed, seedOverride);
      void generateImage(
        generationRequest(prompt, negativePrompt, modelId, params, seed, edit),
        (progress) => {
          setFrame((current) => ({
            step: progress.step,
            total: progress.total,
            preview: progress.preview ?? current?.preview ?? null,
          }));
          if (
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
          }
          setSlides(image.images);
          setSlideIndex(image.images.length - 1);
          setHistoryRevision((current) => current + 1);
        })
        .catch((error: unknown) => {
          if (isCancelled(error)) {
            restoreEdit();
            return;
          }
          reportFailure(error);
        })
        .finally(() => {
          discardEdit();
          generatingRef.current = false;
          setGenerating(false);
        });
    },
    [
      commitEdit,
      discardEdit,
      editRef,
      generatingRef,
      holdEdit,
      modelId,
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
  };
}
