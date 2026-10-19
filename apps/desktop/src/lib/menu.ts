export type MenuEffect = "settings" | "toggle-theme" | "generate" | "redo-setup";

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
    default:
      return null;
  }
}
