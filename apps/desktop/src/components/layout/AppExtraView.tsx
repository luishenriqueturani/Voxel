import type { Dispatch, SetStateAction } from "react";
import type { FlowMenuCommand } from "../../lib/flow";
import type { Theme } from "../../lib/preferences";
import { openLocalPath } from "../../native/files";
import {
  importLocalModel,
  type ListedModel,
  loadModels,
  modelsDirectory,
  removeModel,
} from "../../native/models";
import type { PythonProgress } from "../../native/pythonEnv";
import { saveHfToken } from "../../native/token";
import { FlowsView } from "./FlowsView";
import { type SettingSection, SettingsView } from "./SettingsView";
import { TemplatesView } from "./TemplatesView";

export type ExtraView = "templates" | "flows" | "settings";

type AppExtraViewProps = {
  view: ExtraView;
  onWorkspace: () => void;
  flowCommand: FlowMenuCommand;
  section: SettingSection;
  onSectionChange: (section: SettingSection) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onLanguageChange: (language: "pt-BR" | "en") => void;
  imageBackend: string;
  recommendedBackend: string | null;
  onImageBackendChange: (backend: string) => void;
  hfTokenConfigured: boolean;
  setHfTokenConfigured: (value: boolean) => void;
  library: ListedModel[];
  setLibrary: Dispatch<SetStateAction<ListedModel[]>>;
  reportFailure: (error: unknown) => void;
  environment: PythonProgress | null;
  reinstallEnvironment: () => Promise<void>;
  removeEnvironment: () => Promise<void>;
};

export function AppExtraView({
  view,
  onWorkspace,
  flowCommand,
  section,
  onSectionChange,
  theme,
  onThemeChange,
  onLanguageChange,
  imageBackend,
  recommendedBackend,
  onImageBackendChange,
  hfTokenConfigured,
  setHfTokenConfigured,
  library,
  setLibrary,
  reportFailure,
  environment,
  reinstallEnvironment,
  removeEnvironment,
}: AppExtraViewProps) {
  if (view === "templates") {
    return <TemplatesView onBack={onWorkspace} />;
  }
  if (view === "flows") {
    return <FlowsView onBack={onWorkspace} command={flowCommand} />;
  }
  return (
    <SettingsView
      section={section}
      onSectionChange={onSectionChange}
      theme={theme}
      onThemeChange={onThemeChange}
      onLanguageChange={onLanguageChange}
      imageBackend={imageBackend}
      recommendedBackend={recommendedBackend}
      onImageBackendChange={onImageBackendChange}
      hfTokenConfigured={hfTokenConfigured}
      onHfTokenSave={(token) => {
        void saveHfToken(token).then(setHfTokenConfigured);
      }}
      onHfTokenClear={() => {
        void saveHfToken("").then(setHfTokenConfigured);
      }}
      models={library}
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
      onBack={onWorkspace}
      environment={environment}
      onReinstallEnvironment={() => {
        void reinstallEnvironment();
      }}
      onRemoveEnvironment={() => {
        void removeEnvironment().catch(reportFailure);
      }}
    />
  );
}
