import type { GenerationParams } from "../components/layout/PromptPanel";

export const MODEL_FAMILIES = ["sd15", "sdxl", "flux", "qwen"] as const;
export const IMAGE_BACKENDS = ["vulkan", "rocm", "cuda", "cpu"] as const;

export type ModelFamily = (typeof MODEL_FAMILIES)[number];
export type ImageBackendId = (typeof IMAGE_BACKENDS)[number];

export type ModelPreset = {
  family: ModelFamily;
  resolution: number;
  steps: number;
  cfg: number;
  backend: ImageBackendId;
};

const STORAGE_KEY = "voxel.modelPresets";

const MEASURED: Record<string, ModelPreset> = {
  sd15: { family: "sd15", resolution: 512, steps: 20, cfg: 7, backend: "vulkan" },
};

let ask: (() => void) | null = null;
let queued: ModelPreset | null = null;

export function onPresetRequest(listener: () => void): () => void {
  ask = listener;
  return () => {
    if (ask === listener) {
      ask = null;
    }
  };
}

export function requestPreset(): void {
  ask?.();
}

export function queuePreset(preset: ModelPreset): void {
  queued = preset;
}

export function takeQueuedPreset(): ModelPreset | null {
  const value = queued;
  queued = null;
  return value;
}

export function presetFor(modelId: string): ModelPreset | null {
  return readAll()[modelId] ?? MEASURED[modelId] ?? null;
}

export function isImageBackend(value: string): value is ImageBackendId {
  return IMAGE_BACKENDS.some((backend) => backend === value);
}

// Backend gravado neste modelo. Preset antigo, sem o campo, não troca a escolha global.
export function explicitBackend(modelId: string): ImageBackendId | null {
  const value = rawPresets()[modelId];
  if (typeof value !== "object" || value === null || !("backend" in value)) {
    return null;
  }
  const backend = value.backend;
  return typeof backend === "string" && isImageBackend(backend) ? backend : null;
}

export function savePreset(modelId: string, preset: ModelPreset): void {
  const all = readAll();
  all[modelId] = preset;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
}

// Parâmetros desta execução. Sem preset gravado nem medido, pede o modal e não gera.
// Resolução, passos e CFG ficam no painel.
export function paramsForRun(params: GenerationParams, modelId: string): GenerationParams | null {
  if (takeQueuedPreset() !== null) {
    return params;
  }
  if (presetFor(modelId) === null) {
    requestPreset();
    return null;
  }
  return params;
}

function rawPresets(): Record<string, unknown> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) {
      return {};
    }
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) {
      return {};
    }
    return parsed as Record<string, unknown>;
  } catch {
    return {};
  }
}

function readAll(): Record<string, ModelPreset> {
  const result: Record<string, ModelPreset> = {};
  for (const [id, value] of Object.entries(rawPresets())) {
    const preset = asPreset(value);
    if (preset !== null) {
      result[id] = preset;
    }
  }
  return result;
}

function asPreset(value: unknown): ModelPreset | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const preset = value as ModelPreset;
  if (!MODEL_FAMILIES.includes(preset.family)) {
    return null;
  }
  if (![512, 768, 1024].includes(preset.resolution)) {
    return null;
  }
  if (!Number.isFinite(preset.steps) || preset.steps < 1) {
    return null;
  }
  if (!Number.isFinite(preset.cfg) || preset.cfg <= 0) {
    return null;
  }
  const backend = isImageBackend(preset.backend) ? preset.backend : "vulkan";
  return {
    family: preset.family,
    resolution: preset.resolution,
    steps: preset.steps,
    cfg: preset.cfg,
    backend,
  };
}
