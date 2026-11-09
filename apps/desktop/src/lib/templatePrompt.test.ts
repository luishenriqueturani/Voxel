import { describe, expect, it } from "vitest";
import { composeGeneration, composeNegative, composePrompt, extraNegative } from "./templatePrompt";

describe("montagem do prompt", () => {
  it("coloca o texto base depois do prompt", () => {
    expect(composePrompt("corpo inteiro, T-pose", "ferreiro", "after")).toBe(
      "ferreiro, corpo inteiro, T-pose",
    );
  });

  it("coloca o texto base antes do prompt", () => {
    expect(composePrompt("foto de estúdio", "ferreiro", "before")).toBe(
      "foto de estúdio, ferreiro",
    );
  });

  it("ainda honra o marcador antigo {prompt}", () => {
    expect(composePrompt("{prompt}, T-pose", "ferreiro", "before")).toBe("ferreiro, T-pose");
  });

  it("junta o negativo do template com o da geração", () => {
    expect(composeNegative("múltiplas pessoas", "borrado")).toBe("múltiplas pessoas, borrado");
    expect(composeGeneration({ body: "T-pose", negative: "fundo" }, "ferreiro", "borrado")).toEqual(
      {
        prompt: "ferreiro, T-pose",
        negative: "fundo, borrado",
      },
    );
  });

  it("separa o extra do negativo composto sem duplicar o template", () => {
    expect(extraNegative("fundo", "fundo")).toBe("");
    expect(extraNegative("fundo", "fundo, borrado")).toBe("borrado");
    expect(extraNegative("", "borrado")).toBe("borrado");
    expect(extraNegative("fundo", "só isso")).toBe("só isso");
  });
});
