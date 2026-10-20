import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { imageSrc } from "./generation";

export const EDIT_EVENT = "edit://image";

export type EditedImage = {
  src: string;
  path: string;
  transparent: boolean;
};

export function editFromPayload(value: unknown): EditedImage | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const payload = value as { preview?: unknown; path?: unknown; transparent?: unknown };
  if (
    typeof payload.preview !== "string" ||
    typeof payload.path !== "string" ||
    typeof payload.transparent !== "boolean"
  ) {
    return null;
  }
  return {
    src: imageSrc(payload.preview),
    path: payload.path,
    transparent: payload.transparent,
  };
}

async function editedImage(
  command: string,
  args: Record<string, unknown>,
): Promise<EditedImage | null> {
  if (!isTauri()) {
    return null;
  }
  let edited: EditedImage | null = null;
  const unlisten = await listen(EDIT_EVENT, (event) => {
    edited = editFromPayload(event.payload);
  });
  try {
    await invoke(command, args);
    if (edited === null) {
      throw new Error("a imagem não chegou");
    }
    return edited;
  } finally {
    unlisten();
  }
}

export function openImageForEdit(title: string): Promise<EditedImage | null> {
  if (!isTauri()) {
    return Promise.resolve(null);
  }
  return open({
    title,
    multiple: false,
    filters: [{ name: "Imagem", extensions: ["png", "jpg", "jpeg", "webp"] }],
  }).then((selected) => {
    if (typeof selected !== "string") {
      return null;
    }
    return editedImage("open_edit_image", { path: selected });
  });
}

export function removeImageBackground(source: string): Promise<EditedImage | null> {
  return editedImage("remove_background", { source });
}
