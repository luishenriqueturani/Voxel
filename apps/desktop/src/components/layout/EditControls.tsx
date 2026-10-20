import { useTranslation } from "react-i18next";
import type { EditMode } from "../../lib/edit";

const MODES: EditMode[] = ["new", "vary", "inpaint", "tile"];

function modeHint(mode: EditMode, t: (key: string) => string): string | null {
  if (mode === "vary") {
    return t("edit.hint.vary");
  }
  if (mode === "inpaint") {
    return t("edit.hint.inpaint");
  }
  if (mode === "tile") {
    return t("edit.hint.tile");
  }
  return null;
}

type EditControlsProps = {
  mode: EditMode;
  onModeChange: (mode: EditMode) => void;
  strength: number;
  onStrengthChange: (value: number) => void;
  instruction?: string;
  onInstructionChange?: (value: string) => void;
  hasSource: boolean;
  hasMask: boolean;
};

const changeClass =
  "w-full resize-y rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

function ignoreInstruction(_value: string) {
  return undefined;
}

export function EditControls({
  mode,
  onModeChange,
  strength,
  onStrengthChange,
  instruction = "",
  onInstructionChange = ignoreInstruction,
  hasSource,
  hasMask,
}: EditControlsProps) {
  const { t } = useTranslation();
  const adjusts = mode === "vary" || mode === "inpaint";
  const hint = modeHint(mode, t);
  const blocked = (adjusts && !hasSource) || (mode === "inpaint" && !hasMask);

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="text-sm text-zinc-700 dark:text-zinc-300">{t("edit.mode")}</legend>
      <div className="grid grid-cols-2 gap-1">
        {MODES.map((item) => (
          <label key={item} className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name="edit-mode"
              value={item}
              checked={mode === item}
              onChange={() => onModeChange(item)}
            />
            {t(`edit.${item}`)}
          </label>
        ))}
      </div>
      {adjusts ? (
        <label className="flex flex-col gap-1 text-sm">
          {t("edit.change")}
          <textarea
            aria-label={t("edit.change")}
            value={instruction}
            rows={3}
            className={changeClass}
            onChange={(event) => onInstructionChange(event.target.value)}
          />
        </label>
      ) : null}
      {adjusts ? <p className="text-xs text-zinc-500">{t("edit.changeHint")}</p> : null}
      {adjusts ? (
        <label className="flex flex-col gap-1 text-sm">
          {t("edit.strength")}
          <input
            aria-label={t("edit.strength")}
            type="range"
            min={0.05}
            max={1}
            step={0.05}
            value={strength}
            onChange={(event) => onStrengthChange(Number(event.target.value))}
          />
          <span className="text-zinc-500">{Math.round(strength * 100)}%</span>
        </label>
      ) : null}
      {hint !== null ? <p className="text-xs text-zinc-500">{hint}</p> : null}
      {blocked ? (
        <p className="text-xs text-zinc-500">
          {hasSource ? t("edit.needMask") : t("edit.needImage")}
        </p>
      ) : null}
    </fieldset>
  );
}

export function editReady(mode: EditMode, hasSource: boolean, hasMask: boolean): boolean {
  if (mode === "vary") {
    return hasSource;
  }
  if (mode === "inpaint") {
    return hasSource && hasMask;
  }
  return true;
}
