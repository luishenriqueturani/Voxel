import { describe, expect, it } from "vitest";
import { type DebugRun, renderDebug } from "./debugReport";

const run: DebugRun = {
  modelId: "hub/exemplo/sdxl",
  family: "sdxl",
  backend: "rocm",
  mode: "2d",
  edit: "nenhuma",
  width: 1024,
  height: 1024,
  steps: 20,
  cfg: 7,
  seed: "1",
  count: 1,
  resolution: 1024,
  aspect: "1:1",
};

describe("renderDebug", () => {
  it("grava a data, o erro e a configuração, sem o prompt", () => {
    const text = renderDebug("falhou", "VAE tensor ausente", run, new Date("2026-10-30T12:38:00Z"));
    expect(text).toContain("Quando:");
    expect(text).toContain("VAE tensor ausente");
    expect(text).toContain("Modelo: hub/exemplo/sdxl");
    expect(text).toContain("Backend: rocm");
    expect(text).toContain("Passos: 20");
    expect(text).not.toContain("prompt");
  });

  it("registra a ausência de geração", () => {
    const text = renderDebug("falhou", "rede", null, new Date("2026-10-30T12:38:00Z"));
    expect(text).toContain("nenhuma geração nesta sessão");
  });
});
