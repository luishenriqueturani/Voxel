import { describe, expect, it } from "vitest";
import { composeGeneration, composePrompt } from "./templatePrompt";

describe("montagem do prompt", () => {
  it("coloca o item no lugar de cada {prompt} e preserva o negativo", () => {
    const template = {
      body: "{prompt}, corpo inteiro, {prompt}",
      negative: "múltiplas pessoas",
    };
    expect(composePrompt(template.body, "ferreiro")).toBe("ferreiro, corpo inteiro, ferreiro");
    expect(composeGeneration(template, "ferreiro")).toEqual({
      prompt: "ferreiro, corpo inteiro, ferreiro",
      negative: "múltiplas pessoas",
    });
  });

  it("mantém o texto base quando o marcador não está no template", () => {
    expect(composePrompt("fundo branco", "ferreiro")).toBe("fundo branco");
  });
});
