import { describe, expect, it } from "vitest";
import { generationRequest } from "./edit";

const params = { width: 512, height: 512, steps: 20, cfg: 7, count: 1 };

describe("generationRequest", () => {
  it("manda a imagem e a intensidade ao variar, e a máscara só ao pintar", () => {
    const vary = generationRequest("gato", "", "sd15", params, 3, {
      mode: "vary",
      strength: 0.4,
      sourcePath: "/origem.png",
      maskPng: "data:image/png",
      instruction: "de chapéu",
    });
    expect(vary).toMatchObject({
      prompt: "(de chapéu:1.6) BREAK [gato]",
      mode: "vary",
      strength: 0.4,
      sourcePath: "/origem.png",
      maskPng: null,
      negativePrompt: null,
    });

    const paint = generationRequest("gato", "texto", "sd15", params, 3, {
      mode: "inpaint",
      strength: 0.8,
      sourcePath: "/origem.png",
      maskPng: "data:image/png",
      instruction: "",
    });
    expect(paint.maskPng).toBe("data:image/png");
    expect(paint.negativePrompt).toBe("texto");

    const marked = generationRequest("cena (noite)", "", "sd15", params, null, {
      mode: "inpaint",
      strength: 0.75,
      sourcePath: "/origem.png",
      maskPng: "data:image/png",
      instruction: "gato (preto)",
    });
    expect(marked.prompt).toBe("(gato \\(preto\\):1.6) BREAK [cena \\(noite\\)]");

    const tile = generationRequest("tecido", "", "sd15", params, null, {
      mode: "tile",
      strength: 0.75,
      sourcePath: "/origem.png",
      maskPng: "data:image/png",
      instruction: "sem emenda",
    });
    expect(tile).toMatchObject({
      prompt: "tecido",
      mode: "tile",
      sourcePath: null,
      maskPng: null,
      seed: null,
    });
  });
});
