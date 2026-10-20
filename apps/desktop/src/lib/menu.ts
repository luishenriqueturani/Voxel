export type MenuEffect =
  | "settings"
  | "toggle-theme"
  | "generate"
  | "redo-setup"
  | "open-image"
  | "remove-background";

export function actionForMenu(id: string): MenuEffect | null {
  switch (id) {
    case "edit.preferences":
      return "settings";
    case "view.theme":
      return "toggle-theme";
    case "generate.run":
      return "generate";
    case "help.redoSetup":
      return "redo-setup";
    case "file.openImage":
      return "open-image";
    case "edit.removeBackground":
      return "remove-background";
    default:
      return null;
  }
}
