import { invoke, isTauri } from "@tauri-apps/api/core";

export async function loadPowerNotice(): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const gpu = await invoke<string | null>("power_limit_notice");
  return typeof gpu === "string" && gpu.length > 0 ? gpu : null;
}
