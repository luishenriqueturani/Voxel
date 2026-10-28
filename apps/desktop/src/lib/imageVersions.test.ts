import { describe, expect, it } from "vitest";
import type { GalleryImage } from "../native/generation";
import { applyEditResult, showVersion } from "./imageVersions";

function slide(path: string): GalleryImage {
  return { src: path, path, seed: 1 };
}

describe("versões da imagem", () => {
  it("guarda a edição na imagem escolhida e conserva as outras", () => {
    const applied = applyEditResult([slide("/a.png"), slide("/b.png"), slide("/c.png")], 1, [
      { src: "/b-editada.png", path: "/b-editada.png", seed: 2 },
    ]);
    expect(applied.index).toBe(1);
    expect(applied.slides.map((item) => item.path)).toEqual(["/a.png", "/b-editada.png", "/c.png"]);
    expect(applied.slides[1]?.versions?.map((item) => item.path)).toEqual([
      "/b.png",
      "/b-editada.png",
    ]);
    const previous = showVersion(applied.slides[1] ?? slide("/b-editada.png"), 0);
    expect(previous.path).toBe("/b.png");
    expect(previous.versions).toHaveLength(2);
  });
});
