import { useTranslation } from "react-i18next";
import type { Theme } from "../../lib/preferences";
import type { DownloadProgress, ListedModel } from "../../native/models";
import { Button } from "../ui/button";
import { ModelsSection } from "./ModelsSection";

export const SETTING_SECTIONS = [
  "general",
  "hardware",
  "models",
  "environment",
  "execution",
  "logs",
] as const;

export type SettingSection = (typeof SETTING_SECTIONS)[number];

type SettingsViewProps = {
  section: SettingSection;
  onSectionChange: (section: SettingSection) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onLanguageChange: (language: "pt-BR" | "en") => void;
  imageBackend: string;
  recommendedBackend: string | null;
  onImageBackendChange: (backend: string) => void;
  hfTokenConfigured: boolean;
  onHfTokenSave: (token: string) => void;
  onHfTokenClear: () => void;
  models: ListedModel[];
  modelProgress?: DownloadProgress | null;
  onDownload: (id: string) => void;
  onRemove: (filename: string) => void;
  onOpenFolder: () => void;
  onImport: () => void;
  onBack: () => void;
};

const IMAGE_BACKENDS = ["vulkan", "rocm", "cuda", "cpu"] as const;

export function SettingsView({
  section,
  onSectionChange,
  theme,
  onThemeChange,
  onLanguageChange,
  imageBackend,
  recommendedBackend,
  onImageBackendChange,
  hfTokenConfigured,
  onHfTokenSave,
  onHfTokenClear,
  models,
  modelProgress = null,
  onDownload,
  onRemove,
  onOpenFolder,
  onImport,
  onBack,
}: SettingsViewProps) {
  const { t, i18n } = useTranslation();

  return (
    <div className="flex min-h-0 flex-1">
      <nav
        aria-label={t("settings.title")}
        className="flex w-56 shrink-0 flex-col gap-1 border-r border-zinc-200 p-3 dark:border-zinc-800"
      >
        {SETTING_SECTIONS.map((item) => (
          <button
            key={item}
            type="button"
            aria-current={item === section ? "page" : undefined}
            onClick={() => onSectionChange(item)}
            className="rounded-md px-2 py-1.5 text-left text-sm hover:bg-zinc-200 aria-[current=page]:bg-zinc-200 dark:hover:bg-zinc-800 dark:aria-[current=page]:bg-zinc-800"
          >
            {t(`settings.sections.${item}`)}
          </button>
        ))}
      </nav>
      <section className="flex min-w-0 flex-1 flex-col gap-4 p-6">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold">{t("settings.title")}</h1>
          <Button variant="ghost" onClick={onBack}>
            {t("settings.back")}
          </Button>
        </div>
        <h2 className="text-base font-medium">{t(`settings.sections.${section}`)}</h2>
        {section === "general" ? (
          <div className="flex max-w-sm flex-col gap-4 text-sm">
            <label className="flex flex-col gap-1">
              {t("settings.language")}
              <select
                value={i18n.language}
                onChange={(event) => onLanguageChange(event.target.value as "pt-BR" | "en")}
                className="rounded-md border border-zinc-300 bg-white px-2 py-1.5 dark:border-zinc-700 dark:bg-zinc-950"
              >
                <option value="pt-BR">Português</option>
                <option value="en">English</option>
              </select>
            </label>
            <fieldset className="flex gap-4">
              <legend className="mb-1">{t("settings.theme")}</legend>
              <label className="flex items-center gap-1.5">
                <input
                  type="radio"
                  name="theme"
                  checked={theme === "light"}
                  onChange={() => onThemeChange("light")}
                />
                {t("settings.themeLight")}
              </label>
              <label className="flex items-center gap-1.5">
                <input
                  type="radio"
                  name="theme"
                  checked={theme === "dark"}
                  onChange={() => onThemeChange("dark")}
                />
                {t("settings.themeDark")}
              </label>
            </fieldset>
          </div>
        ) : section === "hardware" ? (
          <div className="flex max-w-sm flex-col gap-3 text-sm">
            <label className="flex flex-col gap-1">
              {t("settings.backend")}
              <select
                value={imageBackend}
                onChange={(event) => onImageBackendChange(event.target.value)}
                className="rounded-md border border-zinc-300 bg-white px-2 py-1.5 dark:border-zinc-700 dark:bg-zinc-950"
              >
                {IMAGE_BACKENDS.map((id) => (
                  <option key={id} value={id}>
                    {t(`setup.backends.${id}`)}
                  </option>
                ))}
              </select>
            </label>
            <p className="text-zinc-500">
              {t("settings.recommended", {
                backend: t(`setup.backends.${recommendedBackend ?? "vulkan"}`),
              })}
            </p>
          </div>
        ) : section === "models" ? (
          <ModelsSection
            hfTokenConfigured={hfTokenConfigured}
            onHfTokenSave={onHfTokenSave}
            onHfTokenClear={onHfTokenClear}
            models={models}
            progress={modelProgress}
            onDownload={onDownload}
            onRemove={onRemove}
            onOpenFolder={onOpenFolder}
            onImport={onImport}
          />
        ) : (
          <p className="text-sm text-zinc-500">{t("settings.empty")}</p>
        )}
      </section>
    </div>
  );
}
