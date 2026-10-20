export type MenuEffect =
  | "settings"
  | "toggle-theme"
  | "generate"
  | "cancel-generation"
  | "regenerate"
  | "redo-setup"
  | "open-image"
  | "remove-background"
  | "undo-edit"
  | "copy-image"
  | "export-image"
  | "gallery"
  | "viewer"
  | "models"
  | "import-model"
  | "open-models-folder"
  | "diagnostics";

export function actionForMenu(id: string): MenuEffect | null {
  switch (id) {
    case "edit.preferences":
      return "settings";
    case "view.theme":
      return "toggle-theme";
    case "generate.run":
      return "generate";
    case "generate.cancel":
      return "cancel-generation";
    case "generate.regenerate":
      return "regenerate";
    case "help.redoSetup":
      return "redo-setup";
    case "help.diagnostics":
      return "diagnostics";
    case "file.openImage":
      return "open-image";
    case "file.export":
      return "export-image";
    case "edit.removeBackground":
      return "remove-background";
    case "edit.undo":
      return "undo-edit";
    case "edit.copy":
      return "copy-image";
    case "view.gallery":
      return "gallery";
    case "view.viewer":
      return "viewer";
    case "models.manage":
      return "models";
    case "models.import":
      return "import-model";
    case "models.openFolder":
      return "open-models-folder";
    default:
      return null;
  }
}
