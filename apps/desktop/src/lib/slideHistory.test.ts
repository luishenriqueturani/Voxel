import { describe, expect, it } from "vitest";
import { popSlides, pushSlides } from "./slideHistory";

describe("slideHistory", () => {
  it("guarda a imagem anterior e a devolve sem alterar o original", () => {
    const current = [{ src: "antes" }];
    const history = pushSlides([], current);
    current[0] = { src: "depois" };
    const restored = popSlides(history);
    expect(restored.slides).toEqual([{ src: "antes" }]);
    expect(restored.history).toEqual([]);
    expect(popSlides(restored.history).slides).toBeNull();
  });
});
