import { describe, expect, it } from "vitest";
import { promptLabel } from "./promptTitle";

describe("promptLabel", () => {
  it("usa o título curto e ignora lixo", () => {
    expect(promptLabel({ prompt: "um gato", title: "felino no telhado" })).toBe(
      "felino no telhado",
    );
    expect(promptLabel({ prompt: "um gato", title: "Thinking" })).toBe("um gato");
    expect(promptLabel({ prompt: "um gato" })).toBe("um gato");
  });

  it("não trata o prompt copiado como título", () => {
    const prompt = "Female mage, age 21, cheerful expression, Bright yellow eyes";
    expect(promptLabel({ prompt, title: prompt.slice(0, 40) })).toBe(prompt);
  });
});
