import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { imagePaths } from "../lib/mesh";

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
  optimizedGlb: string | null;
  optimizedObj: string | null;
  optimizedFbx: string | null;
  references: string[];
};

export type MeshOptimize = {
  triangles: number;
  textureSize: number;
};

export function meshOptimize(params: {
  optimizeMesh?: boolean;
  targetTriangles?: number;
  textureSize?: number;
}): MeshOptimize | null {
  if (params.optimizeMesh !== true) {
    return null;
  }
  const triangles =
    params.targetTriangles !== undefined && params.targetTriangles > 0
      ? params.targetTriangles
      : 10_000;
  const textureSize =
    params.textureSize === 512 || params.textureSize === 2048 ? params.textureSize : 1024;
  return { triangles, textureSize };
}

function optionalPath(value: unknown): string | null {
  return typeof value === "string" && value !== "" ? value : null;
}

function readMesh(value: unknown): MeshResult | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const mesh = value as MeshResult;
  if (
    typeof mesh.imagePath !== "string" ||
    typeof mesh.glbPath !== "string" ||
    typeof mesh.objPath !== "string" ||
    typeof mesh.preview !== "string"
  ) {
    return null;
  }
  return {
    imagePath: mesh.imagePath,
    glbPath: mesh.glbPath,
    objPath: mesh.objPath,
    preview: mesh.preview,
    optimizedGlb: optionalPath(mesh.optimizedGlb),
    optimizedObj: optionalPath(mesh.optimizedObj),
    optimizedFbx: optionalPath(mesh.optimizedFbx),
    references: referencePaths(mesh.references),
  };
}

function referencePaths(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

export async function pickMeshImages(): Promise<string[]> {
  if (!isTauri()) {
    return [];
  }
  const selected = await open({
    multiple: true,
    filters: [{ name: "Imagem", extensions: ["png", "jpg", "jpeg", "webp", "bmp"] }],
  });
  if (Array.isArray(selected)) {
    return imagePaths(selected);
  }
  return typeof selected === "string" ? imagePaths([selected]) : [];
}

export async function meshFromImage(
  views: string[],
  primary: string,
  optimize: MeshOptimize | null = null,
): Promise<MeshResult | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("mesh_from_image", { views, primary, optimize });
  return readMesh(value);
}
