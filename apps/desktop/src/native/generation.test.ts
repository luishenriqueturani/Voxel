import { describe, expect, it } from "vitest";
import { buildSlides, isCancelled } from "./generation";

describe("buildSlides", () => {
  it("guarda cada imagem pronta e usa a última prévia se o evento final se perder", () => {
    const slides = buildSlides(
      [
        { path: "/tmp/imagem_01.png", seed: 10 },
        { path: "/tmp/imagem_02.png", seed: 11 },
      ],
      new Map([[0, "aaa"]]),
      "bbb",
    );

    expect(slides).toEqual([
      { src: "data:image/png;base64,aaa", path: "/tmp/imagem_01.png", seed: 10 },
      { src: "data:image/png;base64,bbb", path: "/tmp/imagem_02.png", seed: 11 },
    ]);
  });
});

describe("isCancelled", () => {
  it("reconhece a parada pedida e ignora as outras falhas", () => {
    expect(isCancelled("cancelado")).toBe(true);
    expect(isCancelled(new Error('protocolo: 500: {"message":"cancelado"}'))).toBe(true);
    expect(isCancelled({ message: "cancelado" })).toBe(true);
    expect(isCancelled("modelo ausente")).toBe(false);
  });
});
