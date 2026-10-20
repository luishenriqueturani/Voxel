import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { EditMode, EditPayload } from "../../lib/edit";
import type { GalleryItem, PromptEntry } from "../../native/history";
import { Button } from "../ui/button";
import { GalleryView } from "./GalleryView";
import { ImageViewer } from "./ImageViewer";
import { MaskEditor } from "./MaskEditor";
import { type GenerationParams, type OutputMode, PromptPanel } from "./PromptPanel";

type WorkspaceModel = { id: string; name: string };

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
  outputMode: OutputMode;
  onOutputModeChange: (mode: OutputMode) => void;
  generating: boolean;
  locked: boolean;
  onGenerate: () => void;
  onCancel?: () => void;
  sourcePath: string | null;
  editRef: { current: EditPayload };
  historyRevision: number;
  gallerySignal: number;
  viewerSignal: number;
  onHistoryError: (error: unknown) => void;
  onRestoreHistory: (entry: PromptEntry) => void;
  onOpenGeneration: (item: GalleryItem) => void;
  shown: string | null;
  step: number;
  total: number;
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
  canUndoEdit?: boolean;
  onUndoEdit?: () => void;
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
  outputMode,
  onOutputModeChange,
  generating,
  locked,
  onGenerate,
  onCancel,
  sourcePath,
  editRef,
  historyRevision,
  gallerySignal,
  viewerSignal,
  onHistoryError,
  onRestoreHistory,
  onOpenGeneration,
  shown,
  step,
  total,
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
  canUndoEdit = false,
  onUndoEdit,
}: WorkspaceViewProps) {
  const { t } = useTranslation();
  const [showGallery, setShowGallery] = useState(false);
  const [editMode, setEditMode] = useState<EditMode>("new");
  const [strength, setStrength] = useState(0.75);
  const [instruction, setInstruction] = useState("");
  const [maskPng, setMaskPng] = useState<string | null>(null);
  const [mosaic, setMosaic] = useState(false);
  const seenSource = useRef(sourcePath);
  if (seenSource.current !== sourcePath) {
    seenSource.current = sourcePath;
    if (maskPng !== null) {
      setMaskPng(null);
    }
  }
  useEffect(() => {
    editRef.current = { mode: editMode, strength, sourcePath, maskPng, instruction };
  }, [editMode, editRef, instruction, maskPng, sourcePath, strength]);
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
  return (
    <div className="flex min-h-0 flex-1">
      <aside className="flex w-80 shrink-0 border-r border-zinc-200 dark:border-zinc-800">
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
          generating={generating}
          locked={locked}
          historyRevision={historyRevision}
          editMode={editMode}
          onEditModeChange={setEditMode}
          strength={strength}
          onStrengthChange={setStrength}
          instruction={instruction}
          onInstructionChange={setInstruction}
          hasSource={sourcePath !== null}
          hasMask={maskPng !== null}
          onHistoryError={onHistoryError}
          onRestore={(entry) => {
            setShowGallery(false);
            onRestoreHistory(entry);
          }}
          onGenerate={onGenerate}
          onCancel={onCancel}
        />
      </aside>
      <main aria-label={t("main.label")} className="flex min-h-0 flex-1 flex-col">
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
              total={total}
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
              canUndo={canUndoEdit}
              onUndo={onUndoEdit}
            />
          )}
        </div>
      </main>
    </div>
  );
}
