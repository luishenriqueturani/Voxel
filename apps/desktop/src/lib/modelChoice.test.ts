import { describe, expect, it } from "vitest";
import { selectableModels } from "./modelChoice";

describe("seletor de modelos", () => {
  it("mostra o instalado mesmo sem arquitetura e deixa malha e fundo de fora", () => {
    const models = selectableModels(
      [
        {
          id: "sd15",
          name: "Stable Diffusion 1.5",
          installed: true,
          kind: "image",
          architecture: null,
        },
        {
          id: "peso.safetensors",
          name: "peso.safetensors",
          installed: true,
          kind: "unknown",
          architecture: "unknown",
        },
        {
          id: "ausente.safetensors",
          name: "ausente.safetensors",
          installed: false,
          kind: "unknown",
          architecture: null,
        },
        {
          id: "triposr",
          name: "TripoSR",
          installed: true,
          kind: "mesh",
          architecture: null,
        },
        {
          id: "u2net",
          name: "U²-Net",
          installed: true,
          kind: "background",
          architecture: null,
        },
      ],
      "arquitetura desconhecida",
    );
    expect(models).toEqual([
      { id: "sd15", name: "Stable Diffusion 1.5" },
      { id: "peso.safetensors", name: "peso.safetensors · arquitetura desconhecida" },
    ]);
  });
});
