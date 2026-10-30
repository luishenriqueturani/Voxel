import { invoke, isTauri } from "@tauri-apps/api/core";
import { currentRun, renderDebug } from "../lib/debugReport";

export async function recordDebug(summary: string, log: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const body = renderDebug(summary, log, currentRun(), new Date());
  try {
    await invoke("record_debug", { body });
  } catch {
    // O modal continua mesmo se o arquivo não puder ser gravado.
  }
}
