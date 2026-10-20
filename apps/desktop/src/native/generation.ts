import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";

export const GENERATION_EVENT = "generation://progress";

export type GenerationProgress = {
  step: number;
  total: number;
  preview: string | null;
  index?: number;
  count?: number;
  ready?: boolean;
  path?: string | null;
  seed?: number | null;
};

export type GalleryImage = {
  src: string;
  path: string;
  seed: number | null;
  transparent?: boolean;
};

type SavedFile = {
  path: string;
  seed: number;
};

export function buildSlides(
  files: readonly SavedFile[],
  ready: ReadonlyMap<number, string>,
  lastPreview: string | null,
): GalleryImage[] {
  return files.flatMap((file, index) => {
    const preview = ready.get(index) ?? (index === files.length - 1 ? lastPreview : null);
    if (!preview) {
      return [];
    }
    return [{ src: imageSrc(preview), path: file.path, seed: file.seed }];
  });
}

export function imageSrc(preview: string): string {
  return `data:image/png;base64,${preview}`;
}

export type GenerationRequest = {
  prompt: string;
  negativePrompt: string | null;
  modelId: string;
  width: number;
  height: number;
  steps: number;
  cfg: number;
  seed: number | null;
  count: number;
  mode?: string;
  strength?: number;
  sourcePath?: string | null;
  maskPng?: string | null;
};

export type GenerationDefaults = {
  width: number;
  height: number;
  steps: number;
  cfg: number;
};

export type GeneratedImage = {
  path: string;
  seed: number;
  modelId: string;
  preview: string | null;
  images: GalleryImage[];
};

function isDefaults(value: unknown): value is GenerationDefaults {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const defaults = value as GenerationDefaults;
  return typeof defaults.width === "number" && typeof defaults.steps === "number";
}

function isImage(value: unknown): value is GeneratedImage {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const image = value as GeneratedImage;
  return typeof image.path === "string" && typeof image.seed === "number";
}

export async function loadGenerationDefaults(): Promise<GenerationDefaults | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("generation_defaults");
  return isDefaults(value) ? value : null;
}

export function isCancelled(error: unknown): boolean {
  return errorText(error).toLowerCase().includes("cancelado");
}

function errorText(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message;
    if (typeof message === "string") {
      return message;
    }
  }
  return "";
}

export async function cancelGeneration(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("cancel_generation");
}

export async function generateImage(
  request: GenerationRequest,
  onProgress: (progress: GenerationProgress) => void,
): Promise<GeneratedImage | null> {
  if (!isTauri()) {
    return null;
  }
  let lastPreview: string | null = null;
  const ready = new Map<number, string>();
  const stop = await listen<GenerationProgress>(GENERATION_EVENT, (event) => {
    const progress = event.payload;
    if (progress.preview) {
      lastPreview = progress.preview;
    }
    if (progress.ready && progress.preview && typeof progress.index === "number") {
      ready.set(progress.index, progress.preview);
    }
    onProgress(progress);
  });
  try {
    const value = await invoke("generate_image", { request });
    if (!isImage(value)) {
      return null;
    }
    const files = savedFiles(value);
    return { ...value, preview: lastPreview, images: buildSlides(files, ready, lastPreview) };
  } finally {
    stop();
  }
}

function savedFiles(value: { path: string; seed: number; images?: unknown }): SavedFile[] {
  if (!Array.isArray(value.images)) {
    return [{ path: value.path, seed: value.seed }];
  }
  const files = value.images.flatMap((image) => {
    if (typeof image !== "object" || image === null) {
      return [];
    }
    const file = image as SavedFile;
    return typeof file.path === "string" && typeof file.seed === "number" ? [file] : [];
  });
  return files.length > 0 ? files : [{ path: value.path, seed: value.seed }];
}

export async function saveGeneratedImage(source: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const destination = await save({
    defaultPath: "imagem.png",
    filters: [{ name: "PNG", extensions: ["png"] }],
  });
  if (typeof destination !== "string") {
    return;
  }
  await invoke("copy_generation", { source, destination });
}

export async function copyPreview(src: string): Promise<void> {
  const blob = await (await fetch(src)).blob();
  const type = blob.type === "" ? "image/png" : blob.type;
  await navigator.clipboard.write([new ClipboardItem({ [type]: blob })]);
}
