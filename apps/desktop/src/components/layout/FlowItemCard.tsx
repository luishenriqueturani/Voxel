import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type FlowItem,
  type FlowOverrides,
  type FlowTemplate,
  MESH_MODES,
  type MeshMode,
  needsAllConfirmation,
  resolveItem,
} from "../../lib/flow";
import { Button } from "../ui/button";
import { ASPECTS } from "./PromptPanel";

type FlowItemCardProps = {
  item: FlowItem;
  index: number;
  total: number;
  template: FlowTemplate | null;
  onChange: (item: FlowItem) => void;
  onMove: (from: number, to: number) => void;
  onRemove: () => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

export function FlowItemCard({
  item,
  index,
  total,
  template,
  onChange,
  onMove,
  onRemove,
}: FlowItemCardProps) {
  const { t } = useTranslation();
  const [confirmAll, setConfirmAll] = useState(false);
  const resolved = resolveItem(template, item);

  function pickMesh(mesh: MeshMode) {
    if (needsAllConfirmation(item.count, mesh)) {
      setConfirmAll(true);
      return;
    }
    setConfirmAll(false);
    onChange({ ...item, mesh });
  }

  return (
    <article className="flex flex-col gap-3 rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
      <header className="flex flex-wrap items-center gap-2">
        <h3 className="flex-1 text-sm font-medium">{t("flows.item", { index: index + 1 })}</h3>
        <Button variant="ghost" disabled={index === 0} onClick={() => onMove(index, index - 1)}>
          {t("flows.moveUp")}
        </Button>
        <Button
          variant="ghost"
          disabled={index >= total - 1}
          onClick={() => onMove(index, index + 1)}
        >
          {t("flows.moveDown")}
        </Button>
        <Button variant="ghost" onClick={onRemove}>
          {t("flows.removeItem")}
        </Button>
      </header>
      <label className="flex flex-col gap-1 text-sm font-medium">
        {t("flows.prompt")}
        <textarea
          aria-label={t("flows.prompt")}
          className={`${fieldClass} min-h-16`}
          value={item.prompt}
          onChange={(event) => onChange({ ...item, prompt: event.target.value })}
        />
      </label>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        {t("templates.final")}: {resolved.prompt || t("templates.noNegative")}
      </p>
      <label className="flex flex-col gap-1 text-sm font-medium">
        {t("panel.count")}
        <input
          type="number"
          min={1}
          max={64}
          aria-label={t("panel.count")}
          className={fieldClass}
          value={item.count}
          onChange={(event) => onChange({ ...item, count: Number(event.target.value) })}
        />
      </label>
      <fieldset className="flex flex-col gap-1">
        <legend className="text-sm font-medium">{t("flows.meshLabel")}</legend>
        <div className="flex flex-wrap gap-3">
          {MESH_MODES.map((mesh) => (
            <label key={mesh} className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name={`mesh-${item.id}`}
                checked={item.mesh === mesh}
                onChange={() => pickMesh(mesh)}
              />
              {t(`flows.mesh.${mesh}`)}
            </label>
          ))}
        </div>
      </fieldset>
      {confirmAll ? (
        <div className="flex flex-col gap-2 rounded-md bg-zinc-100 p-3 dark:bg-zinc-900">
          <p role="status">{t("flows.confirmAllBody", { count: item.count })}</p>
          <div className="flex gap-2">
            <Button
              onClick={() => {
                setConfirmAll(false);
                onChange({ ...item, mesh: "all" });
              }}
            >
              {t("flows.confirmAll")}
            </Button>
            <Button variant="ghost" onClick={() => setConfirmAll(false)}>
              {t("flows.cancel")}
            </Button>
          </div>
        </div>
      ) : null}
      <OverridesFields
        item={item}
        template={template}
        onChange={(overrides) => onChange({ ...item, overrides })}
      />
    </article>
  );
}

function OverridesFields({
  item,
  template,
  onChange,
}: {
  item: FlowItem;
  template: FlowTemplate | null;
  onChange: (overrides: FlowOverrides) => void;
}) {
  const { t } = useTranslation();
  const overrides = item.overrides;
  const resolved = resolveItem(template, item);

  function toggle<Key extends keyof FlowOverrides>(
    key: Key,
    enabled: boolean,
    fallback: NonNullable<FlowOverrides[Key]>,
  ) {
    const next = { ...overrides };
    if (enabled) {
      next[key] = fallback;
    } else {
      delete next[key];
    }
    onChange(next);
  }

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="text-sm font-medium">{t("flows.overrides")}</legend>
      <OverrideToggle
        label={t("templates.negative")}
        enabled={overrides.negative !== undefined}
        onToggle={(enabled) => toggle("negative", enabled, resolved.negative)}
      >
        <textarea
          aria-label={t("templates.negative")}
          className={`${fieldClass} min-h-12`}
          value={overrides.negative ?? ""}
          onChange={(event) => onChange({ ...overrides, negative: event.target.value })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("templates.model2d")}
        enabled={overrides.model2d !== undefined}
        onToggle={(enabled) => toggle("model2d", enabled, resolved.model2d)}
      >
        <input
          aria-label={t("templates.model2d")}
          className={fieldClass}
          value={overrides.model2d ?? ""}
          onChange={(event) => onChange({ ...overrides, model2d: event.target.value })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("templates.model3d")}
        enabled={overrides.model3d !== undefined}
        onToggle={(enabled) => toggle("model3d", enabled, resolved.model3d)}
      >
        <input
          aria-label={t("templates.model3d")}
          className={fieldClass}
          value={overrides.model3d ?? ""}
          onChange={(event) => onChange({ ...overrides, model3d: event.target.value })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("panel.resolution")}
        enabled={overrides.resolution !== undefined}
        onToggle={(enabled) => toggle("resolution", enabled, resolved.resolution)}
      >
        <select
          aria-label={t("panel.resolution")}
          className={fieldClass}
          value={String(overrides.resolution ?? resolved.resolution)}
          onChange={(event) => onChange({ ...overrides, resolution: Number(event.target.value) })}
        >
          <option value="512">512</option>
          <option value="768">768</option>
          <option value="1024">1024</option>
        </select>
      </OverrideToggle>
      <OverrideToggle
        label={t("panel.aspect")}
        enabled={overrides.aspect !== undefined}
        onToggle={(enabled) => toggle("aspect", enabled, resolved.aspect)}
      >
        <select
          aria-label={t("panel.aspect")}
          className={fieldClass}
          value={overrides.aspect ?? resolved.aspect}
          onChange={(event) => onChange({ ...overrides, aspect: event.target.value })}
        >
          {ASPECTS.map((aspect) => (
            <option key={aspect} value={aspect}>
              {aspect}
            </option>
          ))}
        </select>
      </OverrideToggle>
      <OverrideToggle
        label={t("panel.steps")}
        enabled={overrides.steps !== undefined}
        onToggle={(enabled) => toggle("steps", enabled, resolved.steps)}
      >
        <input
          aria-label={t("panel.steps")}
          className={fieldClass}
          inputMode="numeric"
          value={String(overrides.steps ?? "")}
          onChange={(event) => onChange({ ...overrides, steps: Number(event.target.value) })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("panel.cfg")}
        enabled={overrides.cfg !== undefined}
        onToggle={(enabled) => toggle("cfg", enabled, resolved.cfg)}
      >
        <input
          aria-label={t("panel.cfg")}
          className={fieldClass}
          inputMode="decimal"
          value={String(overrides.cfg ?? "")}
          onChange={(event) => onChange({ ...overrides, cfg: Number(event.target.value) })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("panel.seed")}
        enabled={overrides.seed !== undefined}
        onToggle={(enabled) => toggle("seed", enabled, resolved.seed ?? "")}
      >
        <input
          aria-label={t("panel.seed")}
          className={fieldClass}
          value={overrides.seed ?? ""}
          onChange={(event) => onChange({ ...overrides, seed: event.target.value })}
        />
      </OverrideToggle>
      <OverrideToggle
        label={t("templates.postprocess")}
        enabled={overrides.postprocess !== undefined}
        onToggle={(enabled) => toggle("postprocess", enabled, resolved.postprocess)}
      >
        <label className="flex items-center gap-2 text-sm font-medium">
          <input
            type="checkbox"
            checked={overrides.postprocess === true}
            onChange={(event) => onChange({ ...overrides, postprocess: event.target.checked })}
          />
          {t("templates.postprocess")}
        </label>
      </OverrideToggle>
    </fieldset>
  );
}

function OverrideToggle({
  label,
  enabled,
  onToggle,
  children,
}: {
  label: string;
  enabled: boolean;
  onToggle: (enabled: boolean) => void;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1">
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={enabled}
          aria-label={t("flows.override", { field: label })}
          onChange={(event) => onToggle(event.target.checked)}
        />
        {label}
      </label>
      {enabled ? children : <p className="text-xs text-zinc-500">{t("flows.inherit")}</p>}
    </div>
  );
}
