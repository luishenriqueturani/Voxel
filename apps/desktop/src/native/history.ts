import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  title?: string;
};

export type GalleryKind = "image" | "mesh";

export type GalleryItem = PromptEntry & {
  path: string;
  createdAt: string;
  preview: string;
  kind: GalleryKind;
  flowId: string;
  flowName: string;
};

export type GalleryQuery = {
  prompt: string;
  modelId: string;
  flowId: string;
  kind: string;
  from: string;
  to: string;
};

export type GalleryFacets = {
  models: string[];
  flows: Array<{ id: string; name: string }>;
};

export const EMPTY_GALLERY_QUERY: GalleryQuery = {
  prompt: "",
  modelId: "",
  flowId: "",
  kind: "",
  from: "",
  to: "",
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

function asGalleryItem(value: unknown): GalleryItem | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const row = value as Partial<GalleryItem>;
  if (
    typeof row.path !== "string" ||
    typeof row.prompt !== "string" ||
    typeof row.negativePrompt !== "string" ||
    typeof row.modelId !== "string" ||
    typeof row.steps !== "number" ||
    typeof row.width !== "number" ||
    typeof row.height !== "number" ||
    typeof row.cfg !== "number" ||
    typeof row.seed !== "number"
  ) {
    return null;
  }
  const preview = typeof row.preview === "string" ? row.preview : "";
  return {
    path: row.path,
    prompt: row.prompt,
    negativePrompt: row.negativePrompt,
    modelId: row.modelId,
    steps: row.steps,
    width: row.width,
    height: row.height,
    cfg: row.cfg,
    seed: row.seed,
    pinned: typeof row.pinned === "boolean" ? row.pinned : false,
    createdAt: typeof row.createdAt === "string" ? row.createdAt : "",
    preview: preview === "" ? "" : `data:image/jpeg;base64,${preview}`,
    kind: row.kind === "mesh" ? "mesh" : "image",
    flowId: typeof row.flowId === "string" ? row.flowId : "",
    flowName: typeof row.flowName === "string" ? row.flowName : "",
  };
}

export function galleryFrom(value: unknown): GalleryItem[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((item) => {
    const row = asGalleryItem(item);
    return row === null ? [] : [row];
  });
}

export function isMeshItem(item: { kind?: string; path: string }): boolean {
  return item.kind === "mesh" || /\.(glb|gltf)$/i.test(item.path);
}

export function companionObj(glb: string): string {
  return glb.replace(/\.(glb|gltf)$/i, ".obj");
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

export async function hasOlderPrompts(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  return (await invoke("has_older_prompts")) === true;
}

export async function loadOlderPrompts(): Promise<PromptEntry[]> {
  if (!isTauri()) {
    return [];
  }
  return promptsFrom(await invoke("list_older_prompts"));
}

export async function setPromptFavorite(entry: PromptEntry, pinned: boolean): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("set_prompt_favorite", { entry, pinned });
}

export const TITLE_EVENT = "prompt://title";

export type PromptTitleEvent = {
  prompt: string;
  title: string;
};

export async function listenToPromptTitle(
  onTitle: (payload: PromptTitleEvent) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<PromptTitleEvent>(TITLE_EVENT, (event) => {
    onTitle(event.payload);
  });
}

export async function loadGallery(
  filter: GalleryQuery = EMPTY_GALLERY_QUERY,
): Promise<GalleryItem[]> {
  if (!isTauri()) {
    return [];
  }
  return galleryFrom(await invoke("list_gallery", { filter }));
}

export async function loadGalleryFacets(): Promise<GalleryFacets> {
  if (!isTauri()) {
    return { models: [], flows: [] };
  }
  const value = await invoke("list_gallery_facets");
  if (typeof value !== "object" || value === null) {
    return { models: [], flows: [] };
  }
  const row = value as Partial<GalleryFacets>;
  return {
    models: Array.isArray(row.models) ? row.models.filter((item) => typeof item === "string") : [],
    flows: Array.isArray(row.flows)
      ? row.flows.flatMap((flow) => {
          if (typeof flow !== "object" || flow === null) {
            return [];
          }
          const item = flow as { id?: unknown; name?: unknown };
          if (typeof item.id !== "string" || typeof item.name !== "string") {
            return [];
          }
          return [{ id: item.id, name: item.name }];
        })
      : [],
  };
}
