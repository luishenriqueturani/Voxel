import { isTauri } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";

export async function openLocalPath(path: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await openPath(path);
}
