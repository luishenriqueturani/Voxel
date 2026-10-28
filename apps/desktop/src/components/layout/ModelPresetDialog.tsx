import { type FormEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { MODEL_FAMILIES, type ModelFamily, type ModelPreset } from "../../lib/modelPreset";
import { Button } from "../ui/button";

type ModelPresetDialogProps = {
  modelName: string;
  initial: ModelPreset;
  pending: boolean;
  onCancel: () => void;
  onSave: (preset: ModelPreset) => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

export function ModelPresetDialog({
  modelName,
  initial,
  pending,
  onCancel,
  onSave,
}: ModelPresetDialogProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLElement>(null);
  const [family, setFamily] = useState<ModelFamily>(initial.family);
  const [resolution, setResolution] = useState(initial.resolution);
  const [steps, setSteps] = useState(String(initial.steps));
  const [cfg, setCfg] = useState(String(initial.cfg));

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onCancel();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onCancel]);

  function submit(event: FormEvent) {
    event.preventDefault();
    const parsedSteps = Number(steps);
    const parsedCfg = Number(cfg);
    if (!Number.isFinite(parsedSteps) || parsedSteps < 1) {
      return;
    }
    if (!Number.isFinite(parsedCfg) || parsedCfg <= 0) {
      return;
    }
    onSave({ family, resolution, steps: parsedSteps, cfg: parsedCfg });
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label={t("panel.close")}
        className="absolute inset-0 bg-black/50"
        onClick={onCancel}
      />
      <section
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="model-preset-title"
        tabIndex={-1}
        className="relative z-10 flex w-full max-w-md flex-col gap-4 rounded-lg border border-zinc-200 bg-white p-5 shadow-lg outline-none dark:border-zinc-800 dark:bg-zinc-950"
      >
        <div className="flex flex-col gap-2">
          <h2 id="model-preset-title" className="text-lg font-semibold">
            {t("preset.title")}
          </h2>
          <p>{modelName}</p>
          {pending ? <p className="text-sm text-zinc-500">{t("preset.pending")}</p> : null}
        </div>
        <form className="flex flex-col gap-3" onSubmit={submit}>
          <label className="flex flex-col gap-1 text-sm font-medium">
            {t("preset.family")}
            <select
              aria-label={t("preset.family")}
              className={fieldClass}
              value={family}
              onChange={(event) => setFamily(event.target.value as ModelFamily)}
            >
              {MODEL_FAMILIES.map((item) => (
                <option key={item} value={item}>
                  {t(`preset.families.${item}`)}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm font-medium">
            {t("panel.resolution")}
            <select
              aria-label={t("panel.resolution")}
              className={fieldClass}
              value={String(resolution)}
              onChange={(event) => setResolution(Number(event.target.value))}
            >
              <option value="512">512</option>
              <option value="768">768</option>
              <option value="1024">1024</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm font-medium">
            {t("panel.steps")}
            <input
              aria-label={t("panel.steps")}
              className={fieldClass}
              inputMode="numeric"
              value={steps}
              onChange={(event) => setSteps(event.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm font-medium">
            {t("panel.cfg")}
            <input
              aria-label={t("panel.cfg")}
              className={fieldClass}
              inputMode="decimal"
              value={cfg}
              onChange={(event) => setCfg(event.target.value)}
            />
          </label>
          <div className="flex gap-2">
            <Button type="submit">
              {pending ? t("preset.saveAndGenerate") : t("preset.save")}
            </Button>
            <Button type="button" variant="ghost" onClick={onCancel}>
              {t("panel.cancel")}
            </Button>
          </div>
        </form>
      </section>
    </div>
  );
}
