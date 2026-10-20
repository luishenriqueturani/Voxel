import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";

const INLINE = /^(https?:|blob:)/;

export function meshUrl(path: string): string | null {
  if (INLINE.test(path)) {
    return path;
  }
  if (!isTauri()) {
    return null;
  }
  return convertFileSrc(path);
}

export type MeshResult = {
  imagePath: string;
  glbPath: string;
  objPath: string;
  preview: string;
};

function isMesh(value: unknown): value is MeshResult {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const mesh = value as MeshResult;
  return (
    typeof mesh.imagePath === "string" &&
    typeof mesh.glbPath === "string" &&
    typeof mesh.objPath === "string" &&
    typeof mesh.preview === "string"
  );
}

export async function meshFromImage(source: string): Promise<MeshResult | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("mesh_from_image", { source });
  return isMesh(value) ? value : null;
}
