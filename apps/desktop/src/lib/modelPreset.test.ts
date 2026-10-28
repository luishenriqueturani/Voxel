import { afterEach, describe, expect, it, vi } from "vitest";
import type { GenerationParams } from "../components/layout/PromptPanel";
import {
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

  it("grava a família e aplica o preset enfileirado nesta geração", () => {
    savePreset("peso.safetensors", { family: "flux", resolution: 1024, steps: 30, cfg: 3.5 });
    queuePreset({ family: "flux", resolution: 1024, steps: 30, cfg: 3.5 });
    expect(paramsForRun(params, "peso.safetensors")).toMatchObject({
      resolution: 1024,
      width: 1024,
      height: 1024,
      steps: 30,
      cfg: 3.5,
    });
    expect(paramsForRun(params, "peso.safetensors")).toEqual(params);
  });
});
