import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorModal } from "./components/layout/ErrorModal";
import { PowerNotice } from "./components/layout/PowerNotice";
import type { GenerationParams, OutputMode } from "./components/layout/PromptPanel";
import { type SettingSection, SettingsView } from "./components/layout/SettingsView";
import { StatusBar } from "./components/layout/StatusBar";
import { WelcomeView } from "./components/layout/WelcomeView";
import { WorkspaceView } from "./components/layout/WorkspaceView";
import { useAppMenu } from "./hooks/useAppMenu";
import { useErrorNotice } from "./hooks/useErrorNotice";
import { useGenerateShortcut } from "./hooks/useGenerateShortcut";
import { useGenerationRequest } from "./hooks/useGenerationRequest";
import { useImageEdit } from "./hooks/useImageEdit";
import { useMeshDrop } from "./hooks/useMeshDrop";
import { useModelDownload } from "./hooks/useModelDownload";
import { usePythonEnvironment } from "./hooks/usePythonEnvironment";
import { blankEdit } from "./lib/edit";
import { applyTheme, LANGUAGE_KEY, resolveTheme, type Theme } from "./lib/preferences";
import { mergeStep, prefersDark, SUMMARY_KEYS } from "./lib/shell";
import { loadImageBackend, restoreImageBackend, saveImageBackend } from "./native/backend";
import type { ErrorNotice } from "./native/errors";
import { openLocalPath } from "./native/files";
import {
  cancelGeneration,
  copyPreview,
  type GalleryImage,
  imageSrc,
  loadGenerationDefaults,
  saveGeneratedImage,
} from "./native/generation";
import {
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
  runSetup,
  type SetupChoices,
  type SetupStep,
} from "./native/setup";
import { loadMachineStatus, type MachineStatus } from "./native/status";
import { loadHfTokenStatus, saveHfToken } from "./native/token";

type View = "workspace" | "settings" | "welcome";

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
  const [waitingEnvironment, setWaitingEnvironment] = useState(false);
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
  const editRef = useRef(blankEdit);
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
  const [historyRevision, setHistoryRevision] = useState(0);
  const [gallerySignal, setGallerySignal] = useState(0);
  const [viewerSignal, setViewerSignal] = useState(0);
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

  const { modelProgress, downloadCatalogModel } = useModelDownload(setLibrary, reportFailure);

  const {
    removing,
    openImage,
    removeBackground,
    openGeneration,
    openHistory,
    canUndoEdit,
    holdEdit,
    commitEdit,
    discardEdit,
    restoreEdit,
    undoEdit,
  } = useImageEdit({
    slides,
    setSlides,
    slideIndex,
    setSlideIndex,
    setView,
    busy: generating,
    reportFailure,
    openTitle: t("menu.file.openImage"),
  });

  const python = usePythonEnvironment();
  const {
    request: requestGeneration,
    sendImage,
    meshFiles,
  } = useGenerationRequest({
    generatingRef,
    removing,
    editRef,
    holdEdit,
    commitEdit,
    discardEdit,
    restoreEdit,
    setGenerating,
    setSlides,
    setSlideIndex,
    setFrame,
    setHistoryRevision,
    params,
    prompt,
    negativePrompt,
    modelId,
    reportFailure,
    outputMode,
    prepareEnvironment: python.wait,
    noteIntent: python.note,
    setWaitingEnvironment,
  });
  useGenerateShortcut(requestGeneration);
  useMeshDrop(sendImage);

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
  useAppMenu({
    setView,
    setSection,
    setTheme,
    requestGeneration,
    openImage,
    removeBackground,
    undoEdit,
    setGallerySignal,
    setViewerSignal,
    setLibrary,
    reportFailure,
    setParams,
    setSetupSteps,
    setCatalog,
    setPreview,
    setConfirmOutcome,
    setConfirmDetail,
    setSetupRun,
    selectedPath: selected?.path ?? null,
    selectedSeed: selected?.seed ?? null,
    previewSrc: shown,
  });

  const summaryKey =
    SUMMARY_KEYS.find((key) => key === notice?.summaryKey) ?? "error.summary.unknown";

  return (
    <div className="flex h-full flex-col bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      {powerGpu !== null ? (
        <PowerNotice gpu={powerGpu} onDismiss={() => setPowerGpu(null)} />
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
          onConfirm={(choices) => void onConfirm(choices)}
          onContinue={() => setView("workspace")}
        />
      ) : view === "workspace" ? (
        <WorkspaceView
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
          onOutputModeChange={(mode) => {
            setOutputMode(mode);
            if (mode === "3d") {
              void python.note("panel");
            }
          }}
          generating={generating}
          locked={removing || waitingEnvironment}
          onGenerate={() => requestGeneration()}
          onImageDrop={sendImage}
          meshFiles={meshFiles}
          onCancel={() => {
            void cancelGeneration();
          }}
          sourcePath={selected?.path ?? null}
          editRef={editRef}
          historyRevision={historyRevision}
          gallerySignal={gallerySignal}
          viewerSignal={viewerSignal}
          onHistoryError={reportFailure}
          onRestoreHistory={openHistory}
          onOpenGeneration={openGeneration}
          shown={shown}
          step={generating ? (frame?.step ?? 0) : 0}
          total={generating ? (frame?.total ?? 0) : 0}
          canSave={selected !== null && !generating && !removing}
          canRegenerate={selected?.seed != null && !generating && !removing}
          canRemove={selected !== null && !generating && !removing}
          removing={removing}
          transparent={!generating && selected?.transparent === true}
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
            if (selected?.seed == null) {
              return;
            }
            const seed = selected.seed;
            setParams((current) => ({ ...current, seed: String(seed) }));
            requestGeneration(seed);
          }}
          canUndoEdit={canUndoEdit}
          onUndoEdit={undoEdit}
          onRemoveBackground={removeBackground}
          onSendTo3d={() => {
            if (selected?.path) {
              sendImage(selected.path);
            }
          }}
        />
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
          modelProgress={modelProgress}
          onDownload={downloadCatalogModel}
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
          environment={python.progress ?? null}
          onReinstallEnvironment={() => {
            void python.reinstall();
          }}
          onRemoveEnvironment={() => {
            void python.remove().catch(reportFailure);
          }}
        />
      )}
      <StatusBar
        status={machine}
        environment={python.progress}
        onOpenSettings={() => setView("settings")}
      />
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
