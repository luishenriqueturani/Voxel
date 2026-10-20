import { describe, expect, it } from "vitest";
import { firstImagePath } from "./mesh";

describe("firstImagePath", () => {
  it("escolhe a primeira imagem e ignora o resto", () => {
    expect(firstImagePath(["notas.txt", "/tmp/gato.PNG", "/tmp/outro.jpg"])).toBe("/tmp/gato.PNG");
    expect(firstImagePath(["modelo.glb"])).toBeNull();
  });
});
