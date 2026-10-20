import { describe, expect, it } from "vitest";
import { applyAppearance, firstImagePath, type MeshNode, trianglesIn } from "./mesh";

describe("firstImagePath", () => {
  it("escolhe a primeira imagem e ignora o resto", () => {
    expect(firstImagePath(["notas.txt", "/tmp/gato.PNG", "/tmp/outro.jpg"])).toBe("/tmp/gato.PNG");
    expect(firstImagePath(["modelo.glb"])).toBeNull();
  });
});

function material() {
  return {
    wireframe: false,
    vertexColors: true,
    map: "textura" as unknown,
    color: {
      value: "#000000",
      set(next: string) {
        this.value = next;
      },
    },
    needsUpdate: false,
    userData: {} as { savedMap?: unknown; hadColors?: boolean },
  };
}

function scene(): { root: MeshNode; paint: ReturnType<typeof material> } {
  const paint = material();
  const mesh: MeshNode = {
    isMesh: true,
    geometry: {
      index: { count: 6 },
      getAttribute: (name) => (name === "position" || name === "color" ? { count: 6 } : undefined),
    },
    material: paint,
    traverse: (visit) => visit(mesh),
  };
  const root: MeshNode = {
    traverse: (visit) => {
      visit(root);
      visit(mesh);
    },
  };
  return { root, paint };
}

describe("malha", () => {
  it("conta os triângulos pelo índice e, sem índice, pelas posições", () => {
    const indexed = scene();
    expect(trianglesIn(indexed.root)).toBe(2);
    const positions: MeshNode = {
      isMesh: true,
      geometry: { index: null, getAttribute: () => ({ count: 9 }) },
      traverse: (visit) => visit(positions),
    };
    expect(trianglesIn(positions)).toBe(3);
  });

  it("alterna arame e textura sem perder o mapa", () => {
    const { root, paint } = scene();
    applyAppearance(root, true, false);
    expect(paint.wireframe).toBe(true);
    expect(paint.map).toBeNull();
    expect(paint.vertexColors).toBe(false);
    expect(paint.color.value).toBe("#c8c8c8");
    applyAppearance(root, false, true);
    expect(paint.wireframe).toBe(false);
    expect(paint.map).toBe("textura");
    expect(paint.vertexColors).toBe(true);
    expect(paint.color.value).toBe("#ffffff");
  });
});
