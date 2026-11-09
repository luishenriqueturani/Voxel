import { useTranslation } from "react-i18next";
import { composeGeneration } from "../../lib/templatePrompt";
import type { PromptTemplate } from "../../native/templates";

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

type PromptTemplateSelectProps = {
  templates: PromptTemplate[];
  templateId: string;
  prompt: string;
  negativePrompt: string;
  showPreview?: boolean;
  onChange: (id: string) => void;
};

export function PromptTemplateSelect({
  templates,
  templateId,
  prompt,
  negativePrompt,
  showPreview = true,
  onChange,
}: PromptTemplateSelectProps) {
  const { t } = useTranslation();
  const selected = templates.find((entry) => entry.id === templateId) ?? null;
  const preview =
    selected === null ? null : composeGeneration(selected, prompt, negativePrompt).prompt;

  return (
    <div className="flex flex-col gap-1">
      <label className="flex flex-col gap-1 text-sm font-medium">
        {t("panel.template")}
        <select
          aria-label={t("panel.template")}
          className={fieldClass}
          value={templateId}
          onChange={(event) => onChange(event.target.value)}
        >
          <option value="">{t("panel.noTemplate")}</option>
          {templates.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name}
            </option>
          ))}
        </select>
      </label>
      {!showPreview || preview === null || preview === "" ? null : (
        <p className="line-clamp-2 text-xs text-zinc-500">{preview}</p>
      )}
    </div>
  );
}
