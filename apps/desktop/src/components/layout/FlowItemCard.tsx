import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type FlowItem,
  type FlowOverrides,
  type MeshMode,
  clampItemCount,
  meshForOutput,
  needsAllConfirmation,
  outputOf,
  resolveItem,
} from "../../lib/flow";
import { extraNegative } from "../../lib/templatePrompt";
import type { PromptTemplate } from "../../native/templates";
import { Button } from "../ui/button";
import { ASPECTS, type AspectRatio, frameSize } from "./PromptPanel";
import { PromptTemplateSelect } from "./PromptTemplateSelect";

type PanelModel = { id: string; name: string };

type FlowItemCardProps = {
  item: FlowItem;
  index: number;
  total: number;
  templates: PromptTemplate[];
  imageModels: PanelModel[];
  meshModels: PanelModel[];
  onChange: (item: FlowItem) => void;
  onMove: (from: number, to: number) => void;
  onRemove: () => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

const HORIZONTAL = new Set(["4:3", "3:2", "16:9", "21:9"]);

export function FlowItemCard({
  item,
  index,
  total,
  templates,
  imageModels,
  meshModels,
  onChange,
  onMove,
  onRemove,
}: FlowItemCardProps) {
  const { t } = useTranslation();
  const [confirmAll, setConfirmAll] = useState(false);
  const template = templates.find((entry) => entry.id === item.templateId) ?? null;
  const resolved = resolveItem(item, template);
  const mode = outputOf(item.mesh);
  const aspect = isAspect(resolved.aspect) ? resolved.aspect : "1:1";
  const size = frameSize(resolved.resolution, aspect);

  function patch(overrides: FlowOverrides) {
    onChange({ ...item, overrides });
  }

  function pickOutput(next: "2d" | "3d") {
    const mesh = meshForOutput(next, item.count);
    if (needsAllConfirmation(item.count, mesh)) {
      setConfirmAll(true);
      return;
    }
    setConfirmAll(false);
    onChange({ ...item, mesh });
  }

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
      <Field label={t("flows.prompt")}>
        <textarea
          aria-label={t("flows.prompt")}
          className={`${fieldClass} min-h-16`}
          value={item.prompt}
          onChange={(event) => onChange({ ...item, prompt: event.target.value })}
        />
      </Field>
      {resolved.prompt === "" || resolved.prompt === item.prompt ? null : (
        <p className="text-sm text-zinc-600 dark:text-zinc-300">
          <span className="font-medium">{t("templates.final")}: </span>
          {resolved.prompt}
        </p>
      )}
      <PromptTemplateSelect
        templates={templates}
        templateId={item.templateId ?? ""}
        prompt={item.prompt}
        negativePrompt={item.overrides.negative ?? ""}
        showPreview={false}
        onChange={(id) => onChange({ ...item, templateId: id === "" ? null : id })}
      />
      <Field label={t("panel.negative")}>
        <textarea
          aria-label={t("panel.negative")}
          className={`${fieldClass} min-h-12`}
          value={resolved.negative}
          onChange={(event) =>
            patch({
              ...item.overrides,
              negative: extraNegative(template?.negative ?? "", event.target.value),
            })
          }
        />
      </Field>
      <fieldset className="flex flex-col gap-1">
        <legend className="text-sm font-medium">{t("panel.output")}</legend>
        <div className="flex flex-wrap gap-4 text-sm">
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name={`output-${item.id}`}
              checked={mode === "2d"}
              onChange={() => pickOutput("2d")}
            />
            {t("panel.output2d")}
          </label>
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name={`output-${item.id}`}
              checked={mode === "3d"}
              onChange={() => pickOutput("3d")}
            />
            {t("panel.output3d")}
          </label>
        </div>
      </fieldset>
      {mode === "2d" ? (
        <ImageFields
          item={item}
          resolved={resolved}
          imageModels={imageModels}
          size={size}
          onItemChange={onChange}
          onOverrides={patch}
        />
      ) : (
        <MeshFields
          item={item}
          resolved={resolved}
          meshModels={meshModels}
          onMesh={pickMesh}
          onOverrides={patch}
        />
      )}
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
    </article>
  );
}

function ImageFields({
  item,
  resolved,
  imageModels,
  size,
  onItemChange,
  onOverrides,
}: {
  item: FlowItem;
  resolved: ReturnType<typeof resolveItem>;
  imageModels: PanelModel[];
  size: { width: number; height: number };
  onItemChange: (item: FlowItem) => void;
  onOverrides: (overrides: FlowOverrides) => void;
}) {
  const { t } = useTranslation();
  const overrides = item.overrides;
  return (
    <>
      <Field label={t("panel.count")}>
        <input
          type="number"
          min={1}
          max={64}
          aria-label={t("panel.count")}
          className={fieldClass}
          value={item.count}
          onChange={(event) =>
            onItemChange({ ...item, count: clampItemCount(Number(event.target.value)) })
          }
        />
      </Field>
      <ModelSelect
        label={t("panel.model")}
        models={imageModels}
        value={resolved.model2d}
        onChange={(model2d) => onOverrides({ ...overrides, model2d })}
      />
      <Field label={t("panel.resolution")}>
        <select
          aria-label={t("panel.resolution")}
          className={fieldClass}
          value={String(resolved.resolution)}
          onChange={(event) =>
            onOverrides({ ...overrides, resolution: Number(event.target.value) })
          }
        >
          <option value="512">512</option>
          <option value="768">768</option>
          <option value="1024">1024</option>
        </select>
      </Field>
      <Field label={t("panel.aspect")}>
        <select
          aria-label={t("panel.aspect")}
          className={fieldClass}
          value={resolved.aspect}
          onChange={(event) => onOverrides({ ...overrides, aspect: event.target.value })}
        >
          <option value="1:1">1:1</option>
          <optgroup label={t("panel.aspectHorizontal")}>
            {ASPECTS.filter((aspect) => HORIZONTAL.has(aspect)).map((aspect) => (
              <option key={aspect} value={aspect}>
                {aspect}
              </option>
            ))}
          </optgroup>
          <optgroup label={t("panel.aspectVertical")}>
            {ASPECTS.filter((aspect) => !HORIZONTAL.has(aspect) && aspect !== "1:1").map(
              (aspect) => (
                <option key={aspect} value={aspect}>
                  {aspect}
                </option>
              ),
            )}
          </optgroup>
        </select>
        <span className="text-xs text-zinc-500">
          {t("panel.size", { width: size.width, height: size.height })}
        </span>
      </Field>
      <Field label={t("panel.steps")}>
        <input
          type="number"
          min={1}
          max={500}
          aria-label={t("panel.steps")}
          className={fieldClass}
          value={resolved.steps}
          onChange={(event) => onOverrides({ ...overrides, steps: Number(event.target.value) })}
        />
      </Field>
      <Field label={t("panel.cfg")}>
        <input
          type="number"
          min={0}
          max={30}
          step={0.5}
          aria-label={t("panel.cfg")}
          className={fieldClass}
          value={resolved.cfg}
          onChange={(event) => onOverrides({ ...overrides, cfg: Number(event.target.value) })}
        />
      </Field>
    </>
  );
}

function MeshFields({
  item,
  resolved,
  meshModels,
  onMesh,
  onOverrides,
}: {
  item: FlowItem;
  resolved: ReturnType<typeof resolveItem>;
  meshModels: PanelModel[];
  onMesh: (mesh: MeshMode) => void;
  onOverrides: (overrides: FlowOverrides) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <ModelSelect
        label={t("templates.model3d")}
        models={meshModels}
        value={resolved.model3d}
        onChange={(model3d) => onOverrides({ ...item.overrides, model3d })}
      />
      <fieldset className="flex flex-col gap-1">
        <legend className="text-sm font-medium">{t("flows.meshLabel")}</legend>
        <div className="flex flex-wrap gap-4 text-sm">
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name={`mesh-${item.id}`}
              checked={item.mesh === "all"}
              onChange={() => onMesh("all")}
            />
            {t("flows.mesh.all")}
          </label>
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name={`mesh-${item.id}`}
              checked={item.mesh === "later"}
              onChange={() => onMesh("later")}
            />
            {t("flows.mesh.later")}
          </label>
        </div>
      </fieldset>
    </>
  );
}

function ModelSelect({
  label,
  models,
  value,
  onChange,
}: {
  label: string;
  models: PanelModel[];
  value: string;
  onChange: (id: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <Field label={label}>
      <select
        aria-label={label}
        className={fieldClass}
        disabled={models.length === 0}
        value={models.some((model) => model.id === value) ? value : ""}
        onChange={(event) => onChange(event.target.value)}
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
    </Field>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1 text-sm font-medium">
      {label}
      {children}
    </div>
  );
}

function isAspect(value: string): value is AspectRatio {
  return (ASPECTS as readonly string[]).includes(value);
}
