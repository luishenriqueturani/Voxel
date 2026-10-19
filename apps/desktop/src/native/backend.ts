import { invoke, isTauri } from "@tauri-apps/api/core";

export type ImageBackendChoice = {
  current: string;
  recommended: string | null;
};

function asChoice(value: unknown): ImageBackendChoice | null {
  if (typeof value !== "object" || value === null || !("current" in value)) {
    return null;
  }
  const choice = value as ImageBackendChoice;
  if (typeof choice.current !== "string") {
    return null;
  }
  return {
    current: choice.current,
    recommended: typeof choice.recommended === "string" ? choice.recommended : null,
  };
}

export async function loadImageBackend(): Promise<ImageBackendChoice | null> {
  if (!isTauri()) {
    return null;
  }
  return asChoice(await invoke("image_backend_choice"));
}

export async function saveImageBackend(backend: string): Promise<ImageBackendChoice | null> {
  if (!isTauri()) {
    return null;
  }
  return asChoice(await invoke("set_image_backend", { backend }));
}

export async function restoreImageBackend(): Promise<ImageBackendChoice | null> {
  if (!isTauri()) {
    return null;
  }
  return asChoice(await invoke("restore_image_backend"));
}
