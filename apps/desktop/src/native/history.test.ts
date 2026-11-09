import { describe, expect, it } from "vitest";
import { companionObj, galleryFrom, isMeshItem, promptImagesFrom, promptsFrom } from "./history";

const entry = {
  prompt: "um gato",
  negativePrompt: "",
  modelId: "sd15",
  steps: 25,
  width: 512,
  height: 512,
  cfg: 7,
  seed: 3,
  pinned: true,
};

describe("promptsFrom", () => {
  it("ignora linhas incompletas", () => {
    expect(promptsFrom([entry, { prompt: "só o texto" }])).toEqual([entry]);
  });
});

describe("promptImagesFrom", () => {
  it("fica só com caminho e seed", () => {
    expect(promptImagesFrom([{ path: "/tmp/a.png", seed: 4 }, { path: 1 }])).toEqual([
      { path: "/tmp/a.png", seed: 4 },
    ]);
  });
});

describe("galleryFrom", () => {
  it("marca a miniatura como jpeg", () => {
    const [item] = galleryFrom([
      { ...entry, path: "/tmp/a.png", preview: "abc", createdAt: "hoje", kind: "image" },
    ]);
    expect(item?.preview).toBe("data:image/jpeg;base64,abc");
    expect(item?.path).toBe("/tmp/a.png");
    expect(item?.kind).toBe("image");
    expect(item?.flowId).toBe("");
  });

  it("deixa vazio quando não há miniatura", () => {
    const [item] = galleryFrom([{ ...entry, path: "/tmp/a.png", preview: "" }]);
    expect(item?.preview).toBe("");
  });

  it("aceita malha sem pinned", () => {
    const [item] = galleryFrom([
      {
        prompt: "gato",
        negativePrompt: "",
        modelId: "triposr",
        steps: 0,
        width: 0,
        height: 0,
        cfg: 0,
        seed: 1,
        path: "/tmp/gato.glb",
        preview: "",
        kind: "mesh",
        flowId: "flow-1",
        flowName: "NPCs",
      },
    ]);
    expect(item?.kind).toBe("mesh");
    expect(item?.pinned).toBe(false);
    expect(item?.flowName).toBe("NPCs");
  });
});

describe("isMeshItem", () => {
  it("reconhece glb e o obj ao lado", () => {
    expect(isMeshItem({ kind: "mesh", path: "/tmp/a.glb" })).toBe(true);
    expect(isMeshItem({ path: "/tmp/a.gltf" })).toBe(true);
    expect(isMeshItem({ kind: "image", path: "/tmp/a.png" })).toBe(false);
    expect(companionObj("/tmp/a.glb")).toBe("/tmp/a.obj");
  });
});
