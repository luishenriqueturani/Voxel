export type MenuEffect = "settings" | "toggle-theme" | "generate";

export function actionForMenu(id: string): MenuEffect | null {
  switch (id) {
    case "edit.preferences":
      return "settings";
    case "view.theme":
      return "toggle-theme";
    case "generate.run":
      return "generate";
    default:
      return null;
  }
}
