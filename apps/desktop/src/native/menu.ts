import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export async function syncNativeMenu(labels: Record<string, string>): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("set_menu_labels", { labels });
}

export async function listenToMenu(onAction: (id: string) => void): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<string>("menu://action", (event) => {
    onAction(event.payload);
  });
}
