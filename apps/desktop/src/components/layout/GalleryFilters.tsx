import { useTranslation } from "react-i18next";
import type { GalleryFacets, GalleryQuery } from "../../native/history";

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

type GalleryFiltersProps = {
  filter: GalleryQuery;
  facets: GalleryFacets;
  onChange: (filter: GalleryQuery) => void;
};

export function GalleryFilters({ filter, facets, onChange }: GalleryFiltersProps) {
  const { t } = useTranslation();

  return (
    <form
      className="grid w-full grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] gap-2"
      onSubmit={(event) => event.preventDefault()}
    >
      <label className="col-span-full flex min-w-0 flex-col gap-1 text-sm font-medium sm:col-span-2">
        {t("gallery.search")}
        <input
          type="search"
          value={filter.prompt}
          className={fieldClass}
          placeholder={t("gallery.searchPlaceholder")}
          onChange={(event) => onChange({ ...filter, prompt: event.target.value })}
        />
      </label>
      <label className="flex min-w-0 flex-col gap-1 text-sm font-medium">
        {t("gallery.dateFrom")}
        <input
          type="date"
          value={filter.from}
          className={fieldClass}
          onChange={(event) => onChange({ ...filter, from: event.target.value })}
        />
      </label>
      <label className="flex min-w-0 flex-col gap-1 text-sm font-medium">
        {t("gallery.dateTo")}
        <input
          type="date"
          value={filter.to}
          className={fieldClass}
          onChange={(event) => onChange({ ...filter, to: event.target.value })}
        />
      </label>
      <label className="flex min-w-0 flex-col gap-1 text-sm font-medium">
        {t("gallery.model")}
        <select
          aria-label={t("gallery.model")}
          className={fieldClass}
          value={filter.modelId}
          onChange={(event) => onChange({ ...filter, modelId: event.target.value })}
        >
          <option value="">{t("gallery.allModels")}</option>
          {facets.models.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </select>
      </label>
      <label className="flex min-w-0 flex-col gap-1 text-sm font-medium">
        {t("gallery.flow")}
        <select
          aria-label={t("gallery.flow")}
          className={fieldClass}
          value={filter.flowId}
          onChange={(event) => onChange({ ...filter, flowId: event.target.value })}
        >
          <option value="">{t("gallery.allFlows")}</option>
          {facets.flows.map((flow) => (
            <option key={flow.id} value={flow.id}>
              {flow.name}
            </option>
          ))}
        </select>
      </label>
      <label className="flex min-w-0 flex-col gap-1 text-sm font-medium">
        {t("gallery.kind")}
        <select
          aria-label={t("gallery.kind")}
          className={fieldClass}
          value={filter.kind}
          onChange={(event) => onChange({ ...filter, kind: event.target.value })}
        >
          <option value="">{t("gallery.kindAll")}</option>
          <option value="image">{t("gallery.kindImage")}</option>
          <option value="mesh">{t("gallery.kindMesh")}</option>
        </select>
      </label>
    </form>
  );
}
