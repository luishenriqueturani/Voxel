import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

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
