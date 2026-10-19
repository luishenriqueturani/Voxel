import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorModal } from "./components/layout/ErrorModal";
import { type OutputMode, PromptPanel } from "./components/layout/PromptPanel";
import { type SettingSection, SettingsView } from "./components/layout/SettingsView";
import { StatusBar } from "./components/layout/StatusBar";
import { WelcomeView } from "./components/layout/WelcomeView";
import { useErrorNotice } from "./hooks/useErrorNotice";
import { useGenerateShortcut } from "./hooks/useGenerateShortcut";
import { useNativeMenu } from "./hooks/useNativeMenu";
import { actionForMenu } from "./lib/menu";
import { applyTheme, LANGUAGE_KEY, resolveTheme, type Theme } from "./lib/preferences";
import type { ErrorNotice } from "./native/errors";
import { loadImageBackend, restoreImageBackend, saveImageBackend } from "./native/backend";
import { generateImage } from "./native/generation";
import { openLocalPath } from "./native/files";
import { loadPowerNotice } from "./native/power";
import { loadStoredSettings, saveStoredSettings } from "./native/settings";
import {
  type ProfileCatalog,
  type SetupChoices,
  type SetupStep,
  confirmSetup,
  listenToSetup,
  loadSetupStatus,
  resetSetup,
  runSetup,
} from "./native/setup";

const SUMMARY_KEYS = [
  "error.summary.incompatibleKernel",
  "error.summary.outOfMemory",
  "error.summary.nativeCrash",
  "error.summary.network",
  "error.summary.unknown",
] as const;

type View = "workspace" | "settings" | "welcome";

function mergeStep(steps: SetupStep[], step: SetupStep): SetupStep[] {
  const index = steps.findIndex((current) => current.id === step.id);
  if (index === -1) {
    return [...steps, step];
  }
  const next = steps.slice();
  next[index] = step;
  return next;
}

function prefersDark(): boolean {
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ?? false;
}

export default function App() {
  const { t, i18n } = useTranslation();
  const [view, setView] = useState<View>("workspace");
  const [setupSteps, setSetupSteps] = useState<SetupStep[]>([]);
  const [catalog, setCatalog] = useState<ProfileCatalog[]>([]);
  const [setupBackend, setSetupBackend] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [confirmOutcome, setConfirmOutcome] = useState<string | null>(null);
  const [confirmDetail, setConfirmDetail] = useState("");
  const [setupRun, setSetupRun] = useState(0);
  const [section, setSection] = useState<SettingSection>("general");
  const [theme, setTheme] = useState<Theme>(() =>
    resolveTheme(localStorage.getItem("voxel.theme"), prefersDark()),
  );
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [outputMode, setOutputMode] = useState<OutputMode>("2d");
  const [frame, setFrame] = useState<{
    step: number;
    total: number;
    preview: string | null;
  } | null>(null);
  const [imageBackend, setImageBackend] = useState("vulkan");
  const [recommendedBackend, setRecommendedBackend] = useState<string | null>(null);
  const [notice, setNotice] = useState<ErrorNotice | null>(null);
  const [powerGpu, setPowerGpu] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
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
  const requestGeneration = useCallback(() => {
    void generateImage(
      { prompt, negativePrompt: negativePrompt === "" ? null : negativePrompt },
      (progress) => setFrame(progress),
    )
      .then((image) => {
        if (image?.preview) {
          setFrame((current) => ({
            step: current?.step ?? 0,
            total: current?.total ?? 0,
            preview: image.preview,
          }));
        }
      })
      .catch((error: unknown) => {
        setNotice({
          summaryKey: "error.summary.unknown",
          log: error instanceof Error ? error.message : String(error),
          path: "",
        });
      });
  }, [negativePrompt, prompt]);
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
      } else if (action === "redo-setup") {
        void resetSetup().then(() => {
          setSetupSteps([]);
          setCatalog([]);
          setPreview(null);
          setConfirmOutcome(null);
          setConfirmDetail("");
          setSetupRun((current) => current + 1);
          setView("welcome");
        });
      }
    },
    [requestGeneration],
  );
  useEffect(() => {
    let cancelled = false;
    void loadSetupStatus().then((status) => {
      if (!cancelled && status !== null && !status.completed) {
        setView("welcome");
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (view !== "welcome") {
      return;
    }
    const generation = setupRun;
    let cancelled = false;
    const stopPromise = listenToSetup((step) => {
      if (!cancelled) {
        setSetupSteps((current) => mergeStep(current, step));
      }
    });
    void runSetup().then((status) => {
      if (!cancelled && generation === setupRun && status !== null && status.steps.length > 0) {
        setSetupSteps(status.steps);
        setCatalog(status.catalog);
        setSetupBackend(status.backend);
      }
    });
    return () => {
      cancelled = true;
      void stopPromise.then((stop) => stop());
    };
  }, [view, setupRun]);

  useEffect(() => {
    let cancelled = false;
    void loadPowerNotice().then((gpu) => {
      if (!cancelled) {
        setPowerGpu(gpu);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (view !== "settings") {
      return;
    }
    let cancelled = false;
    void loadImageBackend().then((choice) => {
      if (!cancelled && choice !== null) {
        setImageBackend(choice.current);
        setRecommendedBackend(choice.recommended);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [view]);

  useNativeMenu(onMenuAction);
  useErrorNotice((next) => {
    setCopied(false);
    setNotice(next);
  });

  async function onConfirm(choices: SetupChoices) {
    setConfirming(true);
    setConfirmOutcome(null);
    const result = await confirmSetup(choices);
    setConfirming(false);
    if (result === null) {
      return;
    }
    setConfirmOutcome(result.outcome);
    setConfirmDetail(result.detail);
    if (result.image !== null) {
      setPreview(result.image);
    }
  }

  async function onLanguageChange(language: "pt-BR" | "en") {
    localStorage.setItem(LANGUAGE_KEY, language);
    await i18n.changeLanguage(language);
    await saveStoredSettings({ language });
  }

  async function copyLog() {
    if (notice === null) {
      return;
    }
    try {
      await navigator.clipboard.writeText(notice.log);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  function openDiagnosis() {
    setNotice(null);
    setCopied(false);
    setSection("hardware");
    setView("settings");
  }

  const summaryKey =
    SUMMARY_KEYS.find((key) => key === notice?.summaryKey) ?? "error.summary.unknown";

  return (
    <div className="flex h-full flex-col bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      {powerGpu !== null ? (
        <div
          role="status"
          className="flex items-start gap-4 border-b border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-100"
        >
          <p className="flex-1">{t("power.body", { gpu: powerGpu })}</p>
          <button
            type="button"
            className="shrink-0 rounded-md border border-amber-400 px-3 py-1 dark:border-amber-700"
            onClick={() => setPowerGpu(null)}
          >
            {t("power.dismiss")}
          </button>
        </div>
      ) : null}
      {view === "welcome" ? (
        <WelcomeView
          steps={setupSteps}
          catalog={catalog}
          backend={setupBackend}
          confirming={confirming}
          image={preview}
          outcome={confirmOutcome}
          detail={confirmDetail}
          onConfirm={(choices) => {
            void onConfirm(choices);
          }}
          onContinue={() => setView("workspace")}
        />
      ) : view === "workspace" ? (
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
            <div className="flex flex-col items-center gap-3">
              {frame?.preview ? (
                <img
                  alt={t("generation.preview")}
                  className="max-h-full max-w-full"
                  src={`data:image/png;base64,${frame.preview}`}
                />
              ) : (
                <p className="text-sm text-zinc-500">{t("main.empty")}</p>
              )}
              {frame !== null && frame.total > 0 ? (
                <p className="text-sm text-zinc-500">
                  {t("generation.step", { step: frame.step, total: frame.total })}
                </p>
              ) : null}
            </div>
          </main>
        </div>
      ) : (
        <SettingsView
          section={section}
          onSectionChange={setSection}
          theme={theme}
          onThemeChange={setTheme}
          onLanguageChange={(language) => void onLanguageChange(language)}
          imageBackend={imageBackend}
          recommendedBackend={recommendedBackend}
          onImageBackendChange={(backend) => {
            setImageBackend(backend);
            void saveImageBackend(backend);
          }}
          onBack={() => setView("workspace")}
        />
      )}
      <StatusBar onOpenSettings={() => setView("settings")} />
      {notice !== null ? (
        <ErrorModal
          summary={t(summaryKey)}
          log={notice.log}
          copied={copied}
          onCopy={() => {
            void copyLog();
          }}
          onOpenLog={() => {
            void openLocalPath(notice.path);
          }}
          onOpenDiagnosis={openDiagnosis}
          onRestoreRecommended={() => {
            void restoreImageBackend().then((choice) => {
              if (choice !== null) {
                setImageBackend(choice.current);
                setRecommendedBackend(choice.recommended);
              }
              setNotice(null);
            });
          }}
          onClose={() => setNotice(null)}
        />
      ) : null}
    </div>
  );
}
