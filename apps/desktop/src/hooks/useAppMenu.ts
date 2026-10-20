import { type Dispatch, type SetStateAction, useCallback } from "react";
import type { GenerationParams } from "../components/layout/PromptPanel";
import type { SettingSection } from "../components/layout/SettingsView";
import { actionForMenu } from "../lib/menu";
import type { Theme } from "../lib/preferences";
import { openLocalPath } from "../native/files";
import { cancelGeneration, copyPreview, saveGeneratedImage } from "../native/generation";
import { importLocalModel, type ListedModel, loadModels, modelsDirectory } from "../native/models";
import type { ProfileCatalog, SetupStep } from "../native/setup";
import { resetSetup } from "../native/setup";
import { useNativeMenu } from "./useNativeMenu";

type AppView = "workspace" | "settings" | "welcome";

type AppMenuInput = {
  setView: (view: AppView) => void;
  setSection: (section: SettingSection) => void;
  setTheme: Dispatch<SetStateAction<Theme>>;
  requestGeneration: (seed?: number) => void;
  openImage: () => void;
  removeBackground: () => void;
  undoEdit: () => void;
  setGallerySignal: Dispatch<SetStateAction<number>>;
  setViewerSignal: Dispatch<SetStateAction<number>>;
  setLibrary: (models: ListedModel[]) => void;
  reportFailure: (error: unknown) => void;
  setParams: Dispatch<SetStateAction<GenerationParams>>;
  setSetupSteps: Dispatch<SetStateAction<SetupStep[]>>;
  setCatalog: Dispatch<SetStateAction<ProfileCatalog[]>>;
  setPreview: Dispatch<SetStateAction<string | null>>;
  setConfirmOutcome: Dispatch<SetStateAction<string | null>>;
  setConfirmDetail: Dispatch<SetStateAction<string>>;
  setSetupRun: Dispatch<SetStateAction<number>>;
  selectedPath: string | null;
  selectedSeed: number | null;
  previewSrc: string | null;
};

export function useAppMenu(input: AppMenuInput): void {
  const openSettings = useCallback(
    (section: SettingSection) => {
      input.setSection(section);
      input.setView("settings");
    },
    [input.setSection, input.setView],
  );

  const onMenuAction = useCallback(
    (id: string) => {
      const action = actionForMenu(id);
      if (action === "settings") {
        openSettings("general");
      } else if (action === "models") {
        openSettings("models");
      } else if (action === "diagnostics") {
        openSettings("hardware");
      } else if (action === "toggle-theme") {
        input.setTheme((current) => (current === "dark" ? "light" : "dark"));
      } else if (action === "generate") {
        input.requestGeneration();
      } else if (action === "cancel-generation") {
        void cancelGeneration();
      } else if (action === "regenerate") {
        if (input.selectedSeed !== null) {
          const seed = input.selectedSeed;
          input.setParams((current) => ({ ...current, seed: String(seed) }));
          input.requestGeneration(seed);
        }
      } else if (action === "redo-setup") {
        void resetSetup().then(() => {
          input.setSetupSteps([]);
          input.setCatalog([]);
          input.setPreview(null);
          input.setConfirmOutcome(null);
          input.setConfirmDetail("");
          input.setSetupRun((current) => current + 1);
          input.setView("welcome");
        });
      } else if (action === "open-image") {
        input.openImage();
      } else if (action === "remove-background") {
        input.removeBackground();
      } else if (action === "undo-edit") {
        input.undoEdit();
      } else if (action === "copy-image") {
        if (input.previewSrc) {
          void copyPreview(input.previewSrc).catch(input.reportFailure);
        }
      } else if (action === "export-image") {
        if (input.selectedPath) {
          void saveGeneratedImage(input.selectedPath).catch(input.reportFailure);
        }
      } else if (action === "gallery") {
        input.setView("workspace");
        input.setGallerySignal((current) => current + 1);
      } else if (action === "viewer") {
        input.setView("workspace");
        input.setViewerSignal((current) => current + 1);
      } else if (action === "import-model") {
        void importLocalModel().then((imported) => {
          if (imported) {
            void loadModels().then(input.setLibrary);
          }
        });
      } else if (action === "open-models-folder") {
        void modelsDirectory().then((path) => {
          if (path !== null) {
            void openLocalPath(path);
          }
        });
      }
    },
    [input, openSettings],
  );

  useNativeMenu(onMenuAction);
}
