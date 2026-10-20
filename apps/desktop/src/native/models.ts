import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export const MODEL_PROGRESS_EVENT = "model://progress";

export type DownloadProgress = {
  modelId: string;
  received: number;
  total: number;
};

export type ListedReason = {
  code: string;
  requiredBytes?: number | null;
  availableBytes?: number | null;
  provenBytes?: number | null;
};

export type ListedModel = {
  id: string;
  name: string;
  license: string;
  bytes: number;
  filename: string;
  installed: boolean;
  catalog: boolean;
  compatibility: "compatible" | "caveat" | "incompatible";
  reason: ListedReason | null;
  architecture: string | null;
  kind: string;
};

function isModel(value: unknown): value is ListedModel {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const model = value as ListedModel;
  return (
    typeof model.id === "string" &&
    typeof model.name === "string" &&
    typeof model.filename === "string" &&
    typeof model.compatibility === "string"
  );
}

export async function loadModels(): Promise<ListedModel[]> {
  if (!isTauri()) {
    return [];
  }
  const value = await invoke("list_models");
  return Array.isArray(value) ? value.filter(isModel) : [];
}

export function progressFrom(value: unknown): DownloadProgress | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const progress = value as DownloadProgress;
  if (
    typeof progress.modelId !== "string" ||
    typeof progress.received !== "number" ||
    typeof progress.total !== "number"
  ) {
    return null;
  }
  return {
    modelId: progress.modelId,
    received: progress.received,
    total: progress.total,
  };
}

export function listenToDownloadProgress(
  onProgress: (progress: DownloadProgress) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return Promise.resolve(() => undefined);
  }
  return listen(MODEL_PROGRESS_EVENT, (event) => {
    const progress = progressFrom(event.payload);
    if (progress) {
      onProgress(progress);
    }
  });
}

export type HubFile = {
  repo: string;
  filename: string;
  bytes: number;
  sha256: string;
  license: string;
  gated: boolean;
};

function isHubFile(value: unknown): value is HubFile {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const file = value as HubFile;
  return (
    typeof file.repo === "string" &&
    typeof file.filename === "string" &&
    typeof file.bytes === "number" &&
    typeof file.license === "string" &&
    typeof file.gated === "boolean"
  );
}

export async function searchHubModels(query: string): Promise<HubFile[]> {
  if (!isTauri()) {
    return [];
  }
  const value = await invoke("search_hub_models", { query });
  return Array.isArray(value) ? value.filter(isHubFile) : [];
}

export async function downloadHubModel(file: HubFile): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("download_hub_model", {
    choice: { repo: file.repo, filename: file.filename },
  });
}

type ActiveListener = (id: string | null) => void;

let activeDownloadId: string | null = null;
const activeListeners = new Set<ActiveListener>();

function notifyActive(id: string | null): void {
  for (const listener of activeListeners) {
    listener(id);
  }
}

export function claimDownload(id: string): boolean {
  if (activeDownloadId !== null) {
    return false;
  }
  activeDownloadId = id;
  notifyActive(id);
  return true;
}

export function releaseDownload(id: string): void {
  if (activeDownloadId !== id) {
    return;
  }
  activeDownloadId = null;
  notifyActive(null);
}

export function subscribeActiveDownload(listener: ActiveListener): () => void {
  activeListeners.add(listener);
  return () => {
    activeListeners.delete(listener);
  };
}

export async function downloadModel(modelId: string, confirmed: boolean): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("download_model", { modelId, confirmed });
}

export async function removeModel(filename: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("remove_model", { filename });
}

export async function modelsDirectory(): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("models_directory");
  return typeof value === "string" ? value : null;
}

export async function importLocalModel(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  const selected = await open({
    multiple: false,
    filters: [{ name: "Modelo", extensions: ["safetensors", "gguf"] }],
  });
  if (typeof selected !== "string") {
    return false;
  }
  await invoke("import_model", { path: selected });
  return true;
}
