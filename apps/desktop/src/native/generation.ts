import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const GENERATION_EVENT = "generation://progress";

export type GenerationProgress = {
  step: number;
  total: number;
  preview: string | null;
};

export type GeneratedImage = {
  path: string;
  preview: string | null;
};

export async function generateImage(
  request: { prompt: string; negativePrompt: string | null },
  onProgress: (progress: GenerationProgress) => void,
): Promise<GeneratedImage | null> {
  if (!isTauri()) {
    return null;
  }
  const stop = await listen<GenerationProgress>(GENERATION_EVENT, (event) => {
    onProgress(event.payload);
  });
  try {
    return await invoke<GeneratedImage>("generate_image", { request });
  } finally {
    stop();
  }
}
