import { invoke, isTauri } from "@tauri-apps/api/core";

export type PromptEntry = {
  prompt: string;
  negativePrompt: string;
  modelId: string;
  steps: number;
  width: number;
  height: number;
  cfg: number;
  seed: number;
  pinned: boolean;
};

export type GalleryItem = PromptEntry & {
  path: string;
  createdAt: string;
  preview: string;
};

function isEntry(value: unknown): value is PromptEntry {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const entry = value as PromptEntry;
  return (
    typeof entry.prompt === "string" &&
    typeof entry.negativePrompt === "string" &&
    typeof entry.modelId === "string" &&
    typeof entry.steps === "number" &&
    typeof entry.width === "number" &&
    typeof entry.height === "number" &&
    typeof entry.cfg === "number" &&
    typeof entry.seed === "number" &&
    typeof entry.pinned === "boolean"
  );
}

export type PromptImage = {
  path: string;
  seed: number;
};

function isPromptImage(value: unknown): value is PromptImage {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const image = value as PromptImage;
  return typeof image.path === "string" && typeof image.seed === "number";
}

export function promptImagesFrom(value: unknown): PromptImage[] {
  return Array.isArray(value) ? value.filter(isPromptImage) : [];
}

export function promptsFrom(value: unknown): PromptEntry[] {
  return Array.isArray(value) ? value.filter(isEntry) : [];
}

export function galleryFrom(value: unknown): GalleryItem[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((item) => {
    if (!isEntry(item) || typeof item !== "object" || item === null) {
      return [];
    }
    const row = item as GalleryItem;
    if (typeof row.path !== "string" || typeof row.preview !== "string") {
      return [];
    }
    return [
      {
        ...row,
        createdAt: typeof row.createdAt === "string" ? row.createdAt : "",
        preview: row.preview === "" ? "" : `data:image/jpeg;base64,${row.preview}`,
      },
    ];
  });
}

export async function loadPromptImages(entry: PromptEntry): Promise<PromptImage[]> {
  if (!isTauri()) {
    return [];
  }
  return promptImagesFrom(await invoke("list_prompt_images", { entry }));
}

export async function loadRecentPrompts(): Promise<PromptEntry[]> {
  if (!isTauri()) {
    return [];
  }
  return promptsFrom(await invoke("list_recent_prompts"));
}

export async function setPromptFavorite(entry: PromptEntry, pinned: boolean): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("set_prompt_favorite", { entry, pinned });
}

export async function loadGallery(): Promise<GalleryItem[]> {
  if (!isTauri()) {
    return [];
  }
  return galleryFrom(await invoke("list_gallery"));
}
