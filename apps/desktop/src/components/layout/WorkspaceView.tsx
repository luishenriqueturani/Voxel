import { useTranslation } from "react-i18next";
import { ImageViewer } from "./ImageViewer";
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
}: WorkspaceViewProps) {
  const { t } = useTranslation();
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
          onGenerate={onGenerate}
        />
      </aside>
      <main
        aria-label={t("main.label")}
        className="flex min-h-0 flex-1 items-center justify-center overflow-auto p-6"
      >
        <h1 className="sr-only">Voxel</h1>
        <ImageViewer
          src={shown}
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
        />
      </main>
    </div>
  );
}
