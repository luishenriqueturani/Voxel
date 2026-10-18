import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { type OutputMode, PromptPanel } from "./components/layout/PromptPanel";
import { type SettingSection, SettingsView } from "./components/layout/SettingsView";
import { StatusBar } from "./components/layout/StatusBar";
import { useGenerateShortcut } from "./hooks/useGenerateShortcut";
import { useNativeMenu } from "./hooks/useNativeMenu";
import { actionForMenu } from "./lib/menu";
import { applyTheme, LANGUAGE_KEY, resolveTheme, type Theme } from "./lib/preferences";
import { loadStoredSettings, saveStoredSettings } from "./native/settings";

type View = "workspace" | "settings";

function prefersDark(): boolean {
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ?? false;
}

export default function App() {
  const { t, i18n } = useTranslation();
  const [view, setView] = useState<View>("workspace");
  const [section, setSection] = useState<SettingSection>("general");
  const [theme, setTheme] = useState<Theme>(() =>
    resolveTheme(localStorage.getItem("voxel.theme"), prefersDark()),
  );
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [outputMode, setOutputMode] = useState<OutputMode>("2d");
  const settingsReady = useRef(false);
  const i18nRef = useRef(i18n);
  i18nRef.current = i18n;

  useEffect(() => {
    let cancelled = false;
    void loadStoredSettings().then(async (stored) => {
      if (cancelled) {
        return;
      }
      if (stored === null) {
        settingsReady.current = true;
        return;
      }
      if (stored.language) {
        localStorage.setItem(LANGUAGE_KEY, stored.language);
        await i18nRef.current.changeLanguage(stored.language);
      }
      if (!cancelled && stored.theme) {
        setTheme(stored.theme);
      }
      if (!cancelled) {
        settingsReady.current = true;
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    applyTheme(theme);
    if (settingsReady.current) {
      void saveStoredSettings({ theme });
    }
  }, [theme]);

  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  // A geração de fato entra na fase 2. O botão, o atalho e o menu já convergem aqui.
  const requestGeneration = useCallback(() => undefined, []);
  useGenerateShortcut(requestGeneration);

  const onMenuAction = useCallback(
    (id: string) => {
      const action = actionForMenu(id);
      if (action === "settings") {
        setView("settings");
      } else if (action === "toggle-theme") {
        setTheme((current) => (current === "dark" ? "light" : "dark"));
      } else if (action === "generate") {
        requestGeneration();
      }
    },
    [requestGeneration],
  );
  useNativeMenu(onMenuAction);

  async function onLanguageChange(language: "pt-BR" | "en") {
    localStorage.setItem(LANGUAGE_KEY, language);
    await i18n.changeLanguage(language);
    await saveStoredSettings({ language });
  }

  return (
    <div className="flex h-full flex-col bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      {view === "workspace" ? (
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-80 shrink-0 border-r border-zinc-200 dark:border-zinc-800">
            <PromptPanel
              prompt={prompt}
              onPromptChange={setPrompt}
              negativePrompt={negativePrompt}
              onNegativePromptChange={setNegativePrompt}
              outputMode={outputMode}
              onOutputModeChange={setOutputMode}
              onGenerate={requestGeneration}
            />
          </aside>
          <main
            aria-label={t("main.label")}
            className="flex flex-1 items-center justify-center p-6"
          >
            <h1 className="sr-only">Voxel</h1>
            <p className="text-sm text-zinc-500">{t("main.empty")}</p>
          </main>
        </div>
      ) : (
        <SettingsView
          section={section}
          onSectionChange={setSection}
          theme={theme}
          onThemeChange={setTheme}
          onLanguageChange={(language) => void onLanguageChange(language)}
          onBack={() => setView("workspace")}
        />
      )}
      <StatusBar onOpenSettings={() => setView("settings")} />
    </div>
  );
}
