import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Language, Theme } from "../lib/preferences";

export type QueueFinish = "none" | "notify" | "suspend" | "poweroff";

export type StoredSettings = {
  language: Language | null;
  theme: Theme | null;
  inhibitSleep: boolean;
  queueFinish: QueueFinish;
};

export type SettingsUpdate = {
  language?: Language;
  theme?: Theme;
  inhibitSleep?: boolean;
  queueFinish?: QueueFinish;
};

export async function loadStoredSettings(): Promise<StoredSettings | null> {
  if (!isTauri()) {
    return null;
  }
  return invoke<StoredSettings>("get_settings");
}

export async function saveStoredSettings(update: SettingsUpdate): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("update_settings", { update });
}
