import { describe, expect, it } from "vitest";
import { galleryFrom, promptImagesFrom, promptsFrom } from "./history";

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
      { ...entry, path: "/tmp/a.png", preview: "abc", createdAt: "hoje" },
    ]);
    expect(item?.preview).toBe("data:image/jpeg;base64,abc");
    expect(item?.path).toBe("/tmp/a.png");
  });

  it("deixa vazio quando não há miniatura", () => {
    const [item] = galleryFrom([{ ...entry, path: "/tmp/a.png", preview: "" }]);
    expect(item?.preview).toBe("");
  });
});
