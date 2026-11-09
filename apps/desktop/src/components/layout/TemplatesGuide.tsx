import { useTranslation } from "react-i18next";

export function TemplatesGuide() {
  const { t } = useTranslation();
  return (
    <aside className="flex w-80 shrink-0 flex-col gap-4 overflow-y-auto border-l border-zinc-200 p-6 text-sm dark:border-zinc-800">
      <h2 className="text-base font-semibold">{t("templates.guide.title")}</h2>
      <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.lead")}</p>

      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t("templates.guide.emphasisTitle")}</h3>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisLead")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisWeight")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisParen")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisLow")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisLimit")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisOrder")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.emphasisFamilies")}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t("templates.guide.structureTitle")}</h3>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.structureBody")}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t("templates.guide.placeTitle")}</h3>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.placeBody")}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t("templates.guide.negativeTitle")}</h3>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.negativeBody")}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t("templates.guide.examplesTitle")}</h3>
        <p className="font-medium">{t("templates.guide.example3dTitle")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.example3d")}</p>
        <p className="font-medium">{t("templates.guide.examplePhotoTitle")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.examplePhoto")}</p>
        <p className="font-medium">{t("templates.guide.exampleEmphasisTitle")}</p>
        <p className="text-zinc-600 dark:text-zinc-400">{t("templates.guide.exampleEmphasis")}</p>
      </section>
    </aside>
  );
}
