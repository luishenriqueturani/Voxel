import { afterEach, describe, expect, it, vi } from "vitest";
import type { GenerationParams } from "../components/layout/PromptPanel";
import {
  explicitBackend,
  onPresetRequest,
  paramsForRun,
  presetFor,
  queuePreset,
  savePreset,
  takeQueuedPreset,
} from "./modelPreset";

const params: GenerationParams = {
  resolution: 768,
  aspect: "1:1",
  width: 768,
  height: 768,
  steps: 25,
  cfg: 7,
  seed: "",
  count: 1,
};

afterEach(() => {
  takeQueuedPreset();
  onPresetRequest(() => undefined)();
  localStorage.removeItem("voxel.modelPresets");
});

describe("preset do modelo", () => {
  it("deixa o modelo medido gerar e pede o modal para um arquivo sem preset", () => {
    const ask = vi.fn();
    const stop = onPresetRequest(ask);
    expect(presetFor("sd15")?.family).toBe("sd15");
    expect(paramsForRun(params, "sd15")).toEqual(params);
    expect(paramsForRun(params, "peso.safetensors")).toBeNull();
    expect(ask).toHaveBeenCalledOnce();
    stop();
  });

  it("grava a família e mantém resolução, passos e CFG do painel", () => {
    savePreset("peso.safetensors", {
      family: "flux",
      resolution: 1024,
      steps: 30,
      cfg: 3.5,
      backend: "rocm",
    });
    queuePreset({ family: "flux", resolution: 1024, steps: 30, cfg: 3.5, backend: "rocm" });
    expect(explicitBackend("peso.safetensors")).toBe("rocm");
    expect(paramsForRun(params, "peso.safetensors")).toEqual(params);
  });

  it("aceita um preset antigo sem backend e não o trata como escolha", () => {
    localStorage.setItem(
      "voxel.modelPresets",
      JSON.stringify({
        "peso.safetensors": { family: "qwen", resolution: 1024, steps: 40, cfg: 4 },
      }),
    );
    expect(presetFor("peso.safetensors")?.family).toBe("qwen");
    expect(explicitBackend("peso.safetensors")).toBeNull();
  });
});
