import { useTranslation } from "react-i18next";
import type { EditMode } from "../../lib/edit";
import { viewName } from "../../lib/meshViews";
import { Button } from "../ui/button";

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
  referencePath?: string | null;
  onPickReference?: () => void;
  onClearReference?: () => void;
};

const changeClass =
  "w-full resize-y rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

function ignoreInstruction(_value: string) {
  return undefined;
}

function ignoreReference() {
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
  referencePath = null,
  onPickReference = ignoreReference,
  onClearReference = ignoreReference,
}: EditControlsProps) {
  const { t } = useTranslation();
  const adjusts = mode === "vary" || mode === "inpaint";
  const usesReference = referencePath !== null;
  const hint = modeHint(mode, t);
  const blocked =
    (mode === "vary" && !hasSource && !usesReference) ||
    (mode === "inpaint" && (!hasSource || !hasMask));

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="text-sm text-zinc-700 dark:text-zinc-300">{t("edit.mode")}</legend>
      <div className="grid grid-cols-2 gap-1">
        {MODES.map((item) => (
          <div key={item} className="group relative">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="edit-mode"
                value={item}
                aria-describedby={`edit-tip-${item}`}
                checked={mode === item}
                onChange={() => onModeChange(item)}
              />
              {t(`edit.${item}`)}
            </label>
            <span
              id={`edit-tip-${item}`}
              role="tooltip"
              className="invisible absolute top-full left-0 z-20 w-48 pt-1 text-xs group-hover:visible group-focus-within:visible"
            >
              <span className="block rounded-md border border-zinc-300 bg-white p-2 text-zinc-700 shadow-md dark:border-zinc-600 dark:bg-zinc-950 dark:text-zinc-200">
                {t(`edit.tip.${item}`)}
              </span>
            </span>
          </div>
        ))}
      </div>
      <div className="flex flex-col gap-1 text-sm">
        <span>{t("edit.reference")}</span>
        {referencePath ? (
          <div className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate">{viewName(referencePath)}</span>
            <Button variant="ghost" onClick={onClearReference}>
              {t("edit.clearReference")}
            </Button>
          </div>
        ) : (
          <Button variant="ghost" className="justify-start" onClick={onPickReference}>
            {t("edit.pickReference")}
          </Button>
        )}
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
      {adjusts || usesReference ? (
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
      {usesReference && mode === "new" ? (
        <p className="text-xs text-zinc-500">{t("edit.referenceHint")}</p>
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

export function editReady(
  mode: EditMode,
  hasSource: boolean,
  hasMask: boolean,
  hasReference = false,
): boolean {
  if (mode === "vary") {
    return hasSource || hasReference;
  }
  if (mode === "inpaint") {
    return hasSource && hasMask;
  }
  return true;
}
