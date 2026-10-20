import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const PYTHON_EVENT = "python://progress";

export type PythonIntent = "panel" | "flow-item" | "template" | "model-download" | "send-to-3d";

export type PythonProgress = {
  phase: string;
  percent: number | null;
  receivedBytes: number | null;
  totalBytes: number | null;
  backend: string | null;
  detail: string | null;
};

function isProgress(value: unknown): value is PythonProgress {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const progress = value as PythonProgress;
  return typeof progress.phase === "string";
}

export async function loadPythonEnvironment(): Promise<PythonProgress | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("python_environment_status");
  return isProgress(value) ? value : null;
}

export async function note3dIntent(reason: PythonIntent): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("note_3d_intent", { reason });
}

export async function awaitPythonEnvironment(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("await_python_environment");
}

export async function reinstallPythonEnvironment(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("reinstall_python_environment");
}

export async function removePythonEnvironment(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("remove_python_environment");
}

export async function listenToPythonEnvironment(
  onProgress: (progress: PythonProgress) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<PythonProgress>(PYTHON_EVENT, (event) => {
    if (isProgress(event.payload)) {
      onProgress(event.payload);
    }
  });
}
