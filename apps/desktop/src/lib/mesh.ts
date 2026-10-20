const IMAGE = /\.(png|jpe?g|webp|bmp)$/i;
const CLAY = "#c8c8c8";
const WHITE = "#ffffff";

export function firstImagePath(paths: readonly string[]): string | null {
  return paths.find((path) => IMAGE.test(path.trim())) ?? null;
}

type GeometryLike = {
  index: { count: number } | null;
  getAttribute: (name: string) => { count: number } | undefined;
};

type MaterialLike = {
  wireframe: boolean;
  vertexColors: boolean;
  map: unknown;
  color?: { set: (value: string) => void };
  needsUpdate: boolean;
  userData: {
    savedMap?: unknown;
    hadColors?: boolean;
  };
};

export type MeshNode = {
  isMesh?: boolean;
  geometry?: GeometryLike;
  material?: MaterialLike | MaterialLike[] | null;
  traverse: (visit: (child: MeshNode) => void) => void;
};

function materialsOf(node: MeshNode): MaterialLike[] {
  if (!node.material) {
    return [];
  }
  return Array.isArray(node.material) ? node.material : [node.material];
}

export function trianglesIn(root: MeshNode): number {
  let total = 0;
  root.traverse((child) => {
    if (!child.isMesh || !child.geometry) {
      return;
    }
    const indexed = child.geometry.index?.count;
    const positions = child.geometry.getAttribute("position")?.count ?? 0;
    total += Math.floor((indexed ?? positions) / 3);
  });
  return total;
}

export function applyAppearance(root: MeshNode, wireframe: boolean, textured: boolean): void {
  root.traverse((child) => {
    if (!child.isMesh) {
      return;
    }
    const colors = child.geometry?.getAttribute("color");
    for (const material of materialsOf(child)) {
      if (material.userData.hadColors === undefined) {
        material.userData.hadColors = material.vertexColors || Boolean(colors);
      }
      if (material.map != null && material.userData.savedMap === undefined) {
        material.userData.savedMap = material.map;
      }
      material.wireframe = wireframe;
      if (textured) {
        material.map = material.userData.savedMap ?? null;
        material.vertexColors = material.userData.hadColors === true;
        material.color?.set(WHITE);
      } else {
        material.map = null;
        material.vertexColors = false;
        material.color?.set(CLAY);
      }
      material.needsUpdate = true;
    }
  });
}
