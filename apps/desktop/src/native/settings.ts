import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Language, Theme } from "../lib/preferences";

export type StoredSettings = {
  language: Language | null;
  theme: Theme | null;
};

export type SettingsUpdate = {
  language?: Language;
  theme?: Theme;
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
