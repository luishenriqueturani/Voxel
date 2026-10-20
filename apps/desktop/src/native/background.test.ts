import { describe, expect, it } from "vitest";
import { editFromPayload } from "./background";

describe("editFromPayload", () => {
  it("monta a prévia quando o evento traz o PNG", () => {
    expect(
      editFromPayload({ preview: "aaaa", path: "/tmp/imagem_01_sem_fundo.png", transparent: true }),
    ).toEqual({
      src: "data:image/png;base64,aaaa",
      path: "/tmp/imagem_01_sem_fundo.png",
      transparent: true,
    });
  });

  it("ignora um evento incompleto", () => {
    expect(editFromPayload({ path: "/tmp/a.png" })).toBeNull();
    expect(editFromPayload(null)).toBeNull();
  });
});
