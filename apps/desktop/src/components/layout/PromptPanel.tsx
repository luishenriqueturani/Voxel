import * as Popover from "@radix-ui/react-popover";
import { Info, Loader2, Settings } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";

export type OutputMode = "2d" | "3d";

export const ASPECTS = ["1:1", "4:3", "16:9", "3:4", "9:16"] as const;

export type AspectRatio = (typeof ASPECTS)[number];

const RATIOS: Record<AspectRatio, readonly [number, number]> = {
  "1:1": [1, 1],
  "4:3": [4, 3],
  "16:9": [16, 9],
  "3:4": [3, 4],
  "9:16": [9, 16],
};

const SIDE_MIN = 256;
const SIDE_MAX = 1024;
const SIDE_STEP = 64;

export function snapSide(value: number): number {
  const clamped = Math.min(SIDE_MAX, Math.max(SIDE_MIN, value));
  const rounded = Math.round(clamped / SIDE_STEP) * SIDE_STEP;
  return Math.min(SIDE_MAX, Math.max(SIDE_MIN, rounded));
}

export function frameSize(
  resolution: number,
  aspect: AspectRatio,
): { width: number; height: number } {
  const [widthRatio, heightRatio] = RATIOS[aspect];
  const longSide = Math.max(widthRatio, heightRatio);
  return {
    width: snapSide((resolution * widthRatio) / longSide),
    height: snapSide((resolution * heightRatio) / longSide),
  };
}

export type GenerationParams = {
  resolution: number;
  aspect: AspectRatio;
  width: number;
  height: number;
  steps: number;
  cfg: number;
  seed: string;
  count: number;
};

export type PanelModel = {
  id: string;
  name: string;
};

type PromptPanelProps = {
  prompt: string;
  onPromptChange: (value: string) => void;
  negativePrompt: string;
  onNegativePromptChange: (value: string) => void;
  models: PanelModel[];
  modelId: string;
  onModelChange: (id: string) => void;
  params: GenerationParams;
  onParamsChange: (params: GenerationParams) => void;
  outputMode: OutputMode;
  onOutputModeChange: (mode: OutputMode) => void;
  generating: boolean;
  locked?: boolean;
  onGenerate: () => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

export function PromptPanel({
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
  locked = false,
  onGenerate,
}: PromptPanelProps) {
  const { t } = useTranslation();
  const promptRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    promptRef.current?.focus();
  }, []);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 p-3">
      <label className="flex min-h-0 flex-col gap-1 text-sm font-medium">
        {t("panel.prompt")}
        <textarea
          id="prompt"
          ref={promptRef}
          value={prompt}
          onChange={(event) => onPromptChange(event.target.value)}
          className={`${fieldClass} min-h-28 flex-1 resize-y`}
        />
      </label>

      <details className="text-sm">
        <summary className="cursor-pointer font-medium">{t("panel.negative")}</summary>
        <textarea
          aria-label={t("panel.negative")}
          value={negativePrompt}
          onChange={(event) => onNegativePromptChange(event.target.value)}
          className={`${fieldClass} mt-1 min-h-16`}
        />
      </details>

      <div className="flex items-end gap-2">
        <label className="flex min-w-0 flex-1 flex-col gap-1 text-sm font-medium">
          {t("panel.model")}
          <select
            disabled={models.length === 0}
            aria-label={t("panel.model")}
            className={fieldClass}
            value={models.some((model) => model.id === modelId) ? modelId : ""}
            onChange={(event) => onModelChange(event.target.value)}
          >
            {models.length === 0 ? (
              <option value="">{t("panel.noModel")}</option>
            ) : (
              models.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))
            )}
          </select>
        </label>
        <Popover.Root>
          <Popover.Trigger asChild>
            <Button variant="ghost" aria-label={t("panel.modelSettings")} className="px-2">
              <Settings aria-hidden="true" size={18} />
            </Button>
          </Popover.Trigger>
          <Popover.Portal>
            <Popover.Content
              side="right"
              align="start"
              sideOffset={8}
              className="z-10 w-64 rounded-md border border-zinc-300 bg-white p-3 shadow-md dark:border-zinc-700 dark:bg-zinc-900"
            >
              <ModelSettings params={params} onChange={onParamsChange} />
            </Popover.Content>
          </Popover.Portal>
        </Popover.Root>
      </div>

      <fieldset className="flex gap-4 text-sm">
        <legend className="mb-1 font-medium">{t("panel.output")}</legend>
        <label className="flex items-center gap-1.5">
          <input
            type="radio"
            name="output"
            value="2d"
            checked={outputMode === "2d"}
            onChange={() => onOutputModeChange("2d")}
          />
          {t("panel.output2d")}
        </label>
        <label className="flex items-center gap-1.5">
          <input
            type="radio"
            name="output"
            value="3d"
            checked={outputMode === "3d"}
            onChange={() => onOutputModeChange("3d")}
          />
          {t("panel.output3d")}
        </label>
      </fieldset>

      <Button
        className="w-full"
        disabled={generating || locked}
        aria-busy={generating || locked}
        onClick={onGenerate}
      >
        {generating ? <Loader2 aria-hidden="true" className="animate-spin" size={16} /> : null}
        {generating ? t("panel.generating") : t("panel.generate")}
      </Button>

      <section className="flex min-h-0 flex-1 flex-col gap-1 border-t border-zinc-200 pt-3 dark:border-zinc-800">
        <h2 className="text-sm font-medium">{t("panel.recent")}</h2>
        <p className="text-sm text-zinc-500">{t("panel.recentEmpty")}</p>
      </section>
    </div>
  );
}

function withShape(
  params: GenerationParams,
  resolution: number,
  aspect: AspectRatio,
): GenerationParams {
  return { ...params, resolution, aspect, ...frameSize(resolution, aspect) };
}

function ModelSettings({
  params,
  onChange,
}: {
  params: GenerationParams;
  onChange: (params: GenerationParams) => void;
}) {
  const { t } = useTranslation();
  const set = (patch: Partial<GenerationParams>) => onChange({ ...params, ...patch });
  const [pinned, setPinned] = useState<string | null>(null);

  function toggleHint(id: string) {
    setPinned((current) => (current === id ? null : id));
  }

  return (
    <div className="flex flex-col gap-2 text-sm">
      <SettingField
        id="generation-resolution"
        label={t("panel.resolution")}
        hint={t("panel.hint.resolution")}
        pinned={pinned === "resolution"}
        onToggle={() => toggleHint("resolution")}
      >
        <select
          id="generation-resolution"
          value={String(params.resolution)}
          className={fieldClass}
          onChange={(event) =>
            onChange(withShape(params, Number(event.target.value), params.aspect))
          }
        >
          <option value="512">512</option>
          <option value="768">768</option>
          <option value="1024">1024</option>
        </select>
      </SettingField>
      <SettingField
        id="generation-aspect"
        label={t("panel.aspect")}
        hint={t("panel.hint.aspect")}
        pinned={pinned === "aspect"}
        onToggle={() => toggleHint("aspect")}
      >
        <select
          id="generation-aspect"
          aria-label={t("panel.aspect")}
          value={params.aspect}
          className={fieldClass}
          onChange={(event) =>
            onChange(withShape(params, params.resolution, event.target.value as AspectRatio))
          }
        >
          <option value="1:1">1:1</option>
          <optgroup label={t("panel.aspectHorizontal")}>
            <option value="4:3">4:3</option>
            <option value="16:9">16:9</option>
          </optgroup>
          <optgroup label={t("panel.aspectVertical")}>
            <option value="3:4">3:4</option>
            <option value="9:16">9:16</option>
          </optgroup>
        </select>
        <span className="text-zinc-500">
          {t("panel.size", { width: params.width, height: params.height })}
        </span>
      </SettingField>
      <SettingField
        id="generation-steps"
        label={t("panel.steps")}
        hint={t("panel.hint.steps")}
        pinned={pinned === "steps"}
        onToggle={() => toggleHint("steps")}
      >
        <input
          id="generation-steps"
          type="number"
          value={params.steps}
          min={1}
          max={500}
          className={fieldClass}
          onChange={(event) => set({ steps: Number(event.target.value) })}
        />
      </SettingField>
      <SettingField
        id="generation-cfg"
        label={t("panel.cfg")}
        hint={t("panel.hint.cfg")}
        pinned={pinned === "cfg"}
        onToggle={() => toggleHint("cfg")}
      >
        <input
          id="generation-cfg"
          type="number"
          value={params.cfg}
          min={0}
          max={30}
          step={0.5}
          className={fieldClass}
          onChange={(event) => set({ cfg: Number(event.target.value) })}
        />
      </SettingField>
      <SettingField
        id="generation-seed"
        label={t("panel.seed")}
        hint={t("panel.hint.seed")}
        pinned={pinned === "seed"}
        onToggle={() => toggleHint("seed")}
      >
        <input
          id="generation-seed"
          type="number"
          value={params.seed}
          className={fieldClass}
          onChange={(event) => set({ seed: event.target.value })}
        />
      </SettingField>
      <SettingField
        id="generation-count"
        label={t("panel.count")}
        hint={t("panel.hint.count")}
        pinned={pinned === "count"}
        onToggle={() => toggleHint("count")}
      >
        <input
          id="generation-count"
          type="number"
          value={params.count}
          min={1}
          max={4}
          className={fieldClass}
          onChange={(event) => set({ count: Number(event.target.value) })}
        />
      </SettingField>
    </div>
  );
}

function SettingField({
  id,
  label,
  hint,
  pinned,
  onToggle,
  children,
}: {
  id: string;
  label: string;
  hint: string;
  pinned: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const tipClass = pinned ? "visible" : "invisible group-hover:visible group-focus-within:visible";

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <label htmlFor={id}>{label}</label>
        <span className="group relative inline-flex">
          <button
            type="button"
            className="inline-flex rounded-full text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
            aria-label={t("panel.about", { name: label })}
            aria-expanded={pinned}
            onClick={onToggle}
          >
            <Info aria-hidden="true" size={14} />
          </button>
          <span
            role="tooltip"
            className={`absolute top-full left-0 z-20 w-52 pt-1 text-xs font-normal text-zinc-700 dark:text-zinc-200 ${tipClass}`}
          >
            <span className="block rounded-md border border-zinc-300 bg-white p-2 shadow-md dark:border-zinc-600 dark:bg-zinc-950">
              {hint}
            </span>
          </span>
        </span>
      </div>
      {children}
    </div>
  );
}
