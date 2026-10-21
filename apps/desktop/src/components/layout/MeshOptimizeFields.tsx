import { useTranslation } from "react-i18next";
import type { GenerationParams } from "./PromptPanel";

export const TEXTURE_SIZES = [512, 1024, 2048] as const;

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

type MeshOptimizeFieldsProps = {
  params: GenerationParams;
  onChange: (params: GenerationParams) => void;
};

export function MeshOptimizeFields({ params, onChange }: MeshOptimizeFieldsProps) {
  const { t } = useTranslation();
  const enabled = params.optimizeMesh === true;
  const set = (patch: Partial<GenerationParams>) => onChange({ ...params, ...patch });

  return (
    <div className="mt-3 flex flex-col gap-2 border-t border-zinc-200 pt-3 dark:border-zinc-800">
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(event) =>
            set({
              optimizeMesh: event.target.checked,
              targetTriangles: params.targetTriangles ?? 10_000,
              textureSize: params.textureSize ?? 1024,
            })
          }
        />
        {t("panel.optimize")}
      </label>
      <p className="text-zinc-500">{t("panel.optimizeHint")}</p>
      {enabled ? (
        <>
          <label className="flex flex-col gap-1">
            {t("panel.triangles")}
            <input
              type="number"
              min={1}
              value={params.targetTriangles ?? 10_000}
              className={fieldClass}
              onChange={(event) => set({ targetTriangles: Number(event.target.value) })}
            />
            <span className="text-zinc-500">{t("panel.trianglesHint")}</span>
          </label>
          <label className="flex flex-col gap-1">
            {t("panel.textureSize")}
            <select
              value={params.textureSize ?? 1024}
              className={fieldClass}
              onChange={(event) => set({ textureSize: Number(event.target.value) })}
            >
              {TEXTURE_SIZES.map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </label>
        </>
      ) : null}
    </div>
  );
}
