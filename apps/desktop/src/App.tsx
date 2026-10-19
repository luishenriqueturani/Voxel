import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorModal } from "./components/layout/ErrorModal";
import { ImageViewer } from "./components/layout/ImageViewer";
import {
  type GenerationParams,
  type OutputMode,
  PromptPanel,
} from "./components/layout/PromptPanel";
import { type SettingSection, SettingsView } from "./components/layout/SettingsView";
import { StatusBar } from "./components/layout/StatusBar";
import { WelcomeView } from "./components/layout/WelcomeView";
import { useErrorNotice } from "./hooks/useErrorNotice";
import { useGenerateShortcut } from "./hooks/useGenerateShortcut";
import { useNativeMenu } from "./hooks/useNativeMenu";
import { actionForMenu } from "./lib/menu";
import { applyTheme, LANGUAGE_KEY, resolveTheme, type Theme } from "./lib/preferences";
import { loadImageBackend, restoreImageBackend, saveImageBackend } from "./native/backend";
import type { ErrorNotice } from "./native/errors";
import { openLocalPath } from "./native/files";
import {
  copyPreview,
  type GalleryImage,
  generateImage,
  imageSrc,
  loadGenerationDefaults,
  saveGeneratedImage,
} from "./native/generation";
import {
  downloadModel,
  importLocalModel,
  type ListedModel,
  loadModels,
  modelsDirectory,
  removeModel,
} from "./native/models";
import { loadPowerNotice } from "./native/power";
import { loadStoredSettings, saveStoredSettings } from "./native/settings";
import {
  confirmSetup,
  listenToSetup,
  loadSetupStatus,
  type ProfileCatalog,
  resetSetup,
  runSetup,
  type SetupChoices,
  type SetupStep,
} from "./native/setup";
import { loadMachineStatus, type MachineStatus } from "./native/status";
import { loadHfTokenStatus, saveHfToken } from "./native/token";

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
  const [params, setParams] = useState<GenerationParams>({
    resolution: 512,
    aspect: "1:1",
    width: 512,
    height: 512,
    steps: 25,
    cfg: 7,
    seed: "",
    count: 1,
  });
  const [generating, setGenerating] = useState(false);
  const generatingRef = useRef(false);
  const [modelId, setModelId] = useState("sd15");
  const [slides, setSlides] = useState<GalleryImage[]>([]);
  const [slideIndex, setSlideIndex] = useState(0);
  const [machine, setMachine] = useState<MachineStatus | null>(null);
  const [imageBackend, setImageBackend] = useState("vulkan");
  const [recommendedBackend, setRecommendedBackend] = useState<string | null>(null);
  const [hfTokenConfigured, setHfTokenConfigured] = useState(false);
  const [library, setLibrary] = useState<ListedModel[]>([]);
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

  const reportFailure = useCallback((error: unknown) => {
    setNotice({
      summaryKey: "error.summary.unknown",
      log: error instanceof Error ? error.message : String(error),
      path: "",
    });
  }, []);

  const requestGeneration = useCallback(
    (seedOverride?: number) => {
      if (generatingRef.current) {
        return;
      }
      generatingRef.current = true;
      setGenerating(true);
      setSlides([]);
      setSlideIndex(0);
      setFrame(null);
      const typed = params.seed.trim() === "" ? null : Number(params.seed);
      const seed = seedOverride ?? (typed !== null && Number.isFinite(typed) ? typed : null);
      void generateImage(
        {
          prompt,
          negativePrompt: negativePrompt === "" ? null : negativePrompt,
          modelId,
          width: params.width,
          height: params.height,
          steps: params.steps,
          cfg: params.cfg,
          seed,
          count: params.count,
        },
        (progress) => {
          setFrame((current) => ({
            step: progress.step,
            total: progress.total,
            preview: progress.preview ?? current?.preview ?? null,
          }));
          if (
            progress.ready &&
            progress.preview &&
            progress.path &&
            typeof progress.seed === "number"
          ) {
            const slide = {
              src: imageSrc(progress.preview),
              path: progress.path,
              seed: progress.seed,
            };
            const index = progress.index ?? 0;
            setSlides((current) => {
              const next = current.slice();
              next[index] = slide;
              return next;
            });
            setSlideIndex(index);
          }
        },
      )
        .then((image) => {
          if (image === null || image.images.length === 0) {
            return;
          }
          setSlides(image.images);
          setSlideIndex(image.images.length - 1);
        })
        .catch(reportFailure)
        .finally(() => {
          generatingRef.current = false;
          setGenerating(false);
        });
    },
    [modelId, negativePrompt, params, prompt, reportFailure],
  );
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
    let cancelled = false;
    void loadGenerationDefaults().then((defaults) => {
      if (!cancelled && defaults) {
        setParams((current) => ({
          ...current,
          resolution: defaults.width,
          aspect: "1:1",
          width: defaults.width,
          height: defaults.height,
          steps: defaults.steps,
          cfg: defaults.cfg,
        }));
      }
    });
    void loadModels().then((models) => {
      if (!cancelled) {
        setLibrary(models);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const tick = () => {
      void loadMachineStatus().then((status) => {
        if (!cancelled && status) {
          setMachine(status);
        }
      });
    };
    tick();
    const timer = window.setInterval(tick, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
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
    void loadHfTokenStatus().then((configured) => {
      if (!cancelled) {
        setHfTokenConfigured(configured);
      }
    });
    void loadModels().then((models) => {
      if (!cancelled) {
        setLibrary(models);
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

  const imageModels = library
    .filter((model) => model.installed && model.kind === "image")
    .map((model) => ({ id: model.id, name: model.name }));
  const selected = slides[slideIndex] ?? null;
  const shown = generating && frame?.preview ? imageSrc(frame.preview) : (selected?.src ?? null);

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
              models={imageModels}
              modelId={modelId}
              onModelChange={setModelId}
              params={params}
              onParamsChange={setParams}
              outputMode={outputMode}
              onOutputModeChange={setOutputMode}
              generating={generating}
              onGenerate={() => requestGeneration()}
            />
          </aside>
          <main
            aria-label={t("main.label")}
            className="flex min-h-0 flex-1 items-center justify-center overflow-auto p-6"
          >
            <h1 className="sr-only">Voxel</h1>
            <ImageViewer
              src={shown}
              step={generating ? (frame?.step ?? 0) : 0}
              total={generating ? (frame?.total ?? 0) : 0}
              canSave={selected !== null && !generating}
              canRegenerate={selected !== null && !generating}
              index={slideIndex}
              count={generating ? 0 : slides.length}
              onPrevious={() => setSlideIndex((current) => Math.max(0, current - 1))}
              onNext={() => setSlideIndex((current) => Math.min(slides.length - 1, current + 1))}
              onSave={() => {
                if (selected) {
                  void saveGeneratedImage(selected.path).catch(reportFailure);
                }
              }}
              onCopy={() => {
                if (shown) {
                  void copyPreview(shown).catch(reportFailure);
                }
              }}
              onRegenerate={() => {
                if (selected) {
                  setParams((current) => ({ ...current, seed: String(selected.seed) }));
                  requestGeneration(selected.seed);
                }
              }}
            />
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
          hfTokenConfigured={hfTokenConfigured}
          onHfTokenSave={(token) => {
            void saveHfToken(token).then(setHfTokenConfigured);
          }}
          onHfTokenClear={() => {
            void saveHfToken("").then(setHfTokenConfigured);
          }}
          models={library}
          onDownload={(id) => {
            void downloadModel(id, true).then(() => loadModels().then(setLibrary));
          }}
          onRemove={(filename) => {
            void removeModel(filename).then(() => loadModels().then(setLibrary));
          }}
          onOpenFolder={() => {
            void modelsDirectory().then((path) => {
              if (path !== null) {
                void openLocalPath(path);
              }
            });
          }}
          onImport={() => {
            void importLocalModel().then((imported) => {
              if (imported) {
                void loadModels().then(setLibrary);
              }
            });
          }}
          onBack={() => setView("workspace")}
        />
      )}
      <StatusBar status={machine} onOpenSettings={() => setView("settings")} />
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
