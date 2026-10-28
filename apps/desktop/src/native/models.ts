import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export const MODEL_PROGRESS_EVENT = "model://progress";

export type DownloadProgress = {
  modelId: string;
  received: number;
  total: number;
  phase?: "download" | "verify";
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
  const phase =
    progress.phase === "download" || progress.phase === "verify" ? progress.phase : undefined;
  return {
    modelId: progress.modelId,
    received: progress.received,
    total: progress.total,
    ...(phase === undefined ? {} : { phase }),
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

export type HubPart = {
  repo: string;
  filename: string;
  bytes: number;
  sha256: string;
  role: string;
  shardIndex: number | null;
  shardCount: number | null;
};

export type HubPackage = {
  repo: string;
  name: string;
  folder: string;
  license: string;
  gated: boolean;
  opener: string;
  directory: boolean;
  marker: string;
  parts: HubPart[];
};

function isHubPart(value: unknown): value is HubPart {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const part = value as HubPart;
  return (
    typeof part.repo === "string" &&
    typeof part.filename === "string" &&
    typeof part.bytes === "number" &&
    typeof part.sha256 === "string" &&
    typeof part.role === "string" &&
    (part.shardIndex === null || typeof part.shardIndex === "number") &&
    (part.shardCount === null || typeof part.shardCount === "number")
  );
}

function isHubPackage(value: unknown): value is HubPackage {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const pkg = value as HubPackage;
  return (
    typeof pkg.repo === "string" &&
    typeof pkg.name === "string" &&
    typeof pkg.folder === "string" &&
    typeof pkg.license === "string" &&
    typeof pkg.gated === "boolean" &&
    typeof pkg.opener === "string" &&
    typeof pkg.directory === "boolean" &&
    typeof pkg.marker === "string" &&
    Array.isArray(pkg.parts) &&
    pkg.parts.every(isHubPart)
  );
}

export async function searchHubModels(query: string): Promise<HubPackage[]> {
  if (!isTauri()) {
    return [];
  }
  const value = await invoke("search_hub_models", { query });
  return Array.isArray(value) ? value.filter(isHubPackage) : [];
}

export async function downloadHubModel(part: HubPart, pkg: HubPackage): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("download_hub_model", {
    choice: {
      repo: part.repo,
      filename: part.filename,
      packageName: pkg.name,
      opener: pkg.opener,
      directory: pkg.directory,
      marker: pkg.marker,
      parts: pkg.parts.map((item) => item.filename),
    },
  });
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
