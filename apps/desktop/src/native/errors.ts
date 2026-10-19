import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const ERROR_EVENT = "error://show";

export type ErrorNotice = {
  summaryKey: string;
  log: string;
  path: string;
};

export async function listenToErrors(onNotice: (notice: ErrorNotice) => void): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<ErrorNotice>(ERROR_EVENT, (event) => {
    onNotice(event.payload);
  });
}
