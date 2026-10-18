import * as Popover from "@radix-ui/react-popover";
import { Settings } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";

export type OutputMode = "2d" | "3d";

type PromptPanelProps = {
  prompt: string;
  onPromptChange: (value: string) => void;
  negativePrompt: string;
  onNegativePromptChange: (value: string) => void;
  outputMode: OutputMode;
  onOutputModeChange: (mode: OutputMode) => void;
  onGenerate: () => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

export function PromptPanel({
  prompt,
  onPromptChange,
  negativePrompt,
  onNegativePromptChange,
  outputMode,
  onOutputModeChange,
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
          <select disabled aria-label={t("panel.model")} className={fieldClass}>
            <option>{t("panel.noModel")}</option>
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
              <ModelSettings />
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

      <Button className="w-full" onClick={onGenerate}>
        {t("panel.generate")}
      </Button>

      <section className="flex min-h-0 flex-1 flex-col gap-1 border-t border-zinc-200 pt-3 dark:border-zinc-800">
        <h2 className="text-sm font-medium">{t("panel.recent")}</h2>
        <p className="text-sm text-zinc-500">{t("panel.recentEmpty")}</p>
      </section>
    </div>
  );
}

function ModelSettings() {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-2 text-sm">
      <label className="flex flex-col gap-1">
        {t("panel.resolution")}
        <select defaultValue="1024" className={fieldClass}>
          <option value="512">512</option>
          <option value="768">768</option>
          <option value="1024">1024</option>
        </select>
      </label>
      <label className="flex flex-col gap-1">
        {t("panel.steps")}
        <input type="number" defaultValue={25} min={1} className={fieldClass} />
      </label>
      <label className="flex flex-col gap-1">
        {t("panel.cfg")}
        <input type="number" defaultValue={7} min={0} step={0.5} className={fieldClass} />
      </label>
      <label className="flex flex-col gap-1">
        {t("panel.seed")}
        <input type="number" className={fieldClass} />
      </label>
      <label className="flex flex-col gap-1">
        {t("panel.count")}
        <input type="number" defaultValue={1} min={1} className={fieldClass} />
      </label>
    </div>
  );
}
