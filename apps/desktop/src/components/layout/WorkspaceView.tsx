import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MeshPaths } from "../../hooks/useGenerationRequest";
import type { EditMode, EditPayload } from "../../lib/edit";
import type { MeshOrigin, MeshPick, MeshView } from "../../lib/meshViews";
import {
  explicitBackend,
  isImageBackend,
  type ModelPreset,
  onPresetRequest,
  presetFor,
  queuePreset,
  savePreset,
} from "../../lib/modelPreset";
import { pickReferenceImage } from "../../native/background";
import type { ImageVersion } from "../../native/generation";
import type { GalleryItem, PromptEntry } from "../../native/history";
import { pickMeshImages } from "../../native/mesh";
import { Button } from "../ui/button";
import { GalleryView } from "./GalleryView";
import { ImageViewer } from "./ImageViewer";
import { MaskEditor } from "./MaskEditor";
import { MeshViewPicker } from "./MeshViewPicker";
import { ModelPresetDialog } from "./ModelPresetDialog";
import { type GenerationParams, type OutputMode, PromptPanel } from "./PromptPanel";

type WorkspaceModel = { id: string; name: string };

function ignoreOrigin(_origin: MeshOrigin) {
  return undefined;
}

function ignoreMeshConfirm(_views: MeshView[], _primaryPath: string) {
  return undefined;
}

function ignoreMeshCancel() {
  return undefined;
}

function ignorePaths(_paths: string[]) {
  return undefined;
}

type WorkspaceViewProps = {
  prompt: string;
  onPromptChange: (value: string) => void;
  negativePrompt: string;
  onNegativePromptChange: (value: string) => void;
  models: WorkspaceModel[];
  modelId: string;
  onModelChange: (id: string) => void;
  params: GenerationParams;
  onParamsChange: (params: GenerationParams) => void;
  imageBackend?: string;
  onImageBackendChange?: (backend: string) => void;
  outputMode: OutputMode;
  onOutputModeChange: (mode: OutputMode) => void;
  meshOrigin?: MeshOrigin;
  onMeshOriginChange?: (origin: MeshOrigin) => void;
  hasMeshViews?: boolean;
  meshPick?: MeshPick | null;
  onConfirmMesh?: (views: MeshView[], primaryPath: string) => void;
  onCancelMesh?: () => void;
  onAddMeshPaths?: (paths: string[]) => void;
  onAddCurrentViews?: () => void;
  canAddCurrent?: boolean;
  generating: boolean;
  locked: boolean;
  onGenerate: () => void;
  onCancel?: () => void;
  sourcePath: string | null;
  editRef: { current: EditPayload };
  historyRevision: number;
  pendingPrompt?: string | null;
  gallerySignal: number;
  viewerSignal: number;
  onHistoryError: (error: unknown) => void;
  onRestoreHistory: (entry: PromptEntry) => void;
  onOpenGeneration: (item: GalleryItem) => void;
  shown: string | null;
  step: number;
  total: number;
  image?: number;
  images?: number;
  canSave: boolean;
  canRegenerate: boolean;
  canRemove: boolean;
  removing: boolean;
  transparent: boolean;
  index: number;
  count: number;
  onPrevious: () => void;
  onNext: () => void;
  onSave: () => void;
  onCopy: () => void;
  onRegenerate: () => void;
  onRemoveBackground: () => void;
  onSendTo3d?: () => void;
  onImageDrop?: (paths: string[]) => void;
  meshFiles?: MeshPaths | null;
  canUndoEdit?: boolean;
  onUndoEdit?: () => void;
  versions?: ImageVersion[];
  versionIndex?: number;
  onSelectVersion?: (index: number) => void;
};

export function WorkspaceView({
  prompt,
  onPromptChange,
  negativePrompt,
  onNegativePromptChange,
  models,
  modelId,
  onModelChange,
  params,
  onParamsChange,
  imageBackend = "vulkan",
  onImageBackendChange,
  outputMode,
  onOutputModeChange,
  meshOrigin = "text",
  onMeshOriginChange = ignoreOrigin,
  hasMeshViews = false,
  meshPick = null,
  onConfirmMesh = ignoreMeshConfirm,
  onCancelMesh = ignoreMeshCancel,
  onAddMeshPaths = ignorePaths,
  onAddCurrentViews = ignoreMeshCancel,
  canAddCurrent = false,
  generating,
  locked,
  onGenerate,
  onCancel,
  sourcePath,
  editRef,
  historyRevision,
  pendingPrompt = null,
  gallerySignal,
  viewerSignal,
  onHistoryError,
  onRestoreHistory,
  onOpenGeneration,
  shown,
  step,
  total,
  image = 0,
  images = 0,
  canSave,
  canRegenerate,
  canRemove,
  removing,
  transparent,
  index,
  count,
  onPrevious,
  onNext,
  onSave,
  onCopy,
  onRegenerate,
  onRemoveBackground,
  onSendTo3d,
  onImageDrop,
  meshFiles = null,
  canUndoEdit = false,
  onUndoEdit,
  versions,
  versionIndex = 0,
  onSelectVersion,
}: WorkspaceViewProps) {
  const { t } = useTranslation();
  const [showGallery, setShowGallery] = useState(false);
  const [editMode, setEditMode] = useState<EditMode>("new");
  const [strength, setStrength] = useState(0.75);
  const [referencePath, setReferencePath] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");
  const [maskPng, setMaskPng] = useState<string | null>(null);
  const [mosaic, setMosaic] = useState(false);
  const [presetMode, setPresetMode] = useState<"edit" | "generate" | null>(null);
  const seenSource = useRef(sourcePath);
  if (seenSource.current !== sourcePath) {
    seenSource.current = sourcePath;
    if (maskPng !== null) {
      setMaskPng(null);
    }
  }
  useEffect(() => {
    editRef.current = {
      mode: editMode,
      strength,
      sourcePath,
      referencePath,
      maskPng,
      instruction,
    };
  }, [editMode, editRef, instruction, maskPng, referencePath, sourcePath, strength]);
  useEffect(() => {
    if (editMode === "tile") {
      setMosaic(true);
    }
  }, [editMode]);
  const painting = editMode === "inpaint" && shown !== null && !showGallery && !generating;
  useEffect(() => {
    if (gallerySignal > 0) {
      setShowGallery(true);
    }
  }, [gallerySignal]);
  useEffect(() => {
    if (viewerSignal > 0) {
      setShowGallery(false);
    }
  }, [viewerSignal]);
  useEffect(() => {
    if (historyRevision > 0) {
      setShowGallery(false);
    }
  }, [historyRevision]);
  useEffect(() => onPresetRequest(() => setPresetMode("generate")), []);
  function startGenerate() {
    const fromImages = outputMode === "3d" && meshOrigin === "images";
    if (!fromImages && presetFor(modelId) === null) {
      setPresetMode("generate");
      return;
    }
    onGenerate();
  }
  function commitPreset(preset: ModelPreset) {
    savePreset(modelId, preset);
    onImageBackendChange?.(preset.backend);
    const generate = presetMode === "generate";
    setPresetMode(null);
    if (generate) {
      queuePreset(preset);
      onGenerate();
    }
  }
  const storedPreset = presetFor(modelId);
  const currentBackend = isImageBackend(imageBackend) ? imageBackend : "vulkan";
  const presetDraft: ModelPreset = {
    family: storedPreset?.family ?? "sd15",
    resolution:
      storedPreset?.resolution ??
      (params.resolution === 768 || params.resolution === 1024 ? params.resolution : 512),
    steps: storedPreset?.steps ?? params.steps,
    cfg: storedPreset?.cfg ?? params.cfg,
    backend: explicitBackend(modelId) ?? currentBackend,
  };
  return (
    <div className="flex min-h-0 flex-1">
      <aside className="flex min-h-0 w-80 shrink-0 flex-col border-r border-zinc-200 dark:border-zinc-800">
        <PromptPanel
          prompt={prompt}
          onPromptChange={onPromptChange}
          negativePrompt={negativePrompt}
          onNegativePromptChange={onNegativePromptChange}
          models={models}
          modelId={modelId}
          onModelChange={onModelChange}
          params={params}
          onParamsChange={onParamsChange}
          outputMode={outputMode}
          onOutputModeChange={onOutputModeChange}
          meshOrigin={meshOrigin}
          onMeshOriginChange={onMeshOriginChange}
          hasMeshViews={hasMeshViews}
          generating={generating}
          locked={locked}
          historyRevision={historyRevision}
          pendingPrompt={pendingPrompt}
          editMode={editMode}
          onEditModeChange={setEditMode}
          strength={strength}
          onStrengthChange={setStrength}
          instruction={instruction}
          onInstructionChange={setInstruction}
          hasSource={sourcePath !== null}
          hasMask={maskPng !== null}
          referencePath={referencePath}
          onPickReference={() => {
            void pickReferenceImage(t("edit.pickReference")).then((path) => {
              if (path !== null) {
                setReferencePath(path);
              }
            });
          }}
          onClearReference={() => setReferencePath(null)}
          onHistoryError={onHistoryError}
          onRestore={(entry) => {
            setShowGallery(false);
            onRestoreHistory(entry);
          }}
          onGenerate={startGenerate}
          onEditPreset={() => setPresetMode("edit")}
          onCancel={onCancel}
          onImageDrop={onImageDrop}
        />
      </aside>
      <main aria-label={t("main.label")} className="relative flex min-h-0 flex-1 flex-col">
        <div className="flex justify-end px-4 pt-3">
          <Button variant="ghost" onClick={() => setShowGallery((open) => !open)}>
            {showGallery ? t("gallery.viewer") : t("gallery.title")}
          </Button>
        </div>
        <div
          className={`flex min-h-0 flex-1 overflow-auto p-6 ${showGallery || painting ? "items-stretch" : "items-center justify-center"}`}
        >
          <h1 className="sr-only">Voxel</h1>
          {showGallery ? (
            <GalleryView
              key={historyRevision}
              onError={onHistoryError}
              onOpen={(item) => {
                setShowGallery(false);
                onOpenGeneration(item);
              }}
            />
          ) : painting && shown !== null ? (
            <MaskEditor
              src={shown}
              onMask={setMaskPng}
              canRevert={canUndoEdit}
              onRevert={onUndoEdit}
            />
          ) : (
            <ImageViewer
              src={shown}
              mosaic={mosaic && !generating}
              onToggleMosaic={() => setMosaic((open) => !open)}
              step={step}
              image={image}
              images={images}
              total={total}
              generating={generating}
              canSave={canSave}
              canRegenerate={canRegenerate}
              canRemove={canRemove}
              removing={removing}
              transparent={transparent}
              index={index}
              count={count}
              onPrevious={onPrevious}
              onNext={onNext}
              onSave={onSave}
              onCopy={onCopy}
              onRegenerate={onRegenerate}
              onRemoveBackground={onRemoveBackground}
              onSendTo3d={onSendTo3d}
              meshFiles={meshFiles}
              canUndo={canUndoEdit}
              onUndo={onUndoEdit}
              versions={versions}
              versionIndex={versionIndex}
              onSelectVersion={onSelectVersion}
            />
          )}
        </div>
        {meshPick !== null ? (
          <MeshViewPicker
            key={meshPick.token}
            views={meshPick.views}
            primaryPath={meshPick.primaryPath}
            canAddCurrent={canAddCurrent}
            onAddPaths={onAddMeshPaths}
            onAddFromDisk={() => {
              void pickMeshImages().then((paths) => {
                if (paths.length > 0) {
                  onAddMeshPaths(paths);
                }
              });
            }}
            onAddCurrent={onAddCurrentViews}
            onConfirm={onConfirmMesh}
            onCancel={onCancelMesh}
          />
        ) : null}
      </main>
      {presetMode !== null ? (
        <ModelPresetDialog
          modelName={models.find((model) => model.id === modelId)?.name ?? modelId}
          initial={presetDraft}
          pending={presetMode === "generate"}
          onCancel={() => setPresetMode(null)}
          onSave={commitPreset}
        />
      ) : null}
    </div>
  );
}
