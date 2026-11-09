import { describe, expect, it } from "vitest";
import {
  blankItem,
  clampItemCount,
  defaultMeshMode,
  moveItem,
  needsAllConfirmation,
  parseFlowFile,
  resolveItem,
  serializeFlowFile,
} from "./flow";

const template = {
  body: "{prompt}, corpo inteiro, T-pose",
  negative: "múltiplas pessoas",
  model2d: "sd15",
  model3d: "triposr",
  resolution: 512,
  aspect: "1:1",
  steps: 20,
  cfg: 7,
  seed: "",
  postprocess: false,
};

describe("fluxo", () => {
  it("deixa o 3D desligado quando há várias imagens", () => {
    expect(defaultMeshMode(1)).toBe("none");
    expect(defaultMeshMode(4)).toBe("none");
    expect(needsAllConfirmation(1, "all")).toBe(false);
    expect(needsAllConfirmation(4, "all")).toBe(true);
    expect(needsAllConfirmation(4, "later")).toBe(false);
    expect(blankItem("a", 3).mesh).toBe("none");
  });

  it("reordena, limita a quantidade e sobrescreve o template", () => {
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    expect(clampItemCount(0)).toBe(1);
    expect(clampItemCount(99)).toBe(64);
    const item = {
      ...blankItem("i1", 3),
      prompt: "ferreiro",
      mesh: "later" as const,
      overrides: { seed: "1234", steps: 28, postprocess: true },
    };
    expect(resolveItem(template, item)).toEqual({
      body: template.body,
      prompt: "ferreiro, corpo inteiro, T-pose",
      negative: "múltiplas pessoas",
      model2d: "sd15",
      model3d: "triposr",
      resolution: 512,
      aspect: "1:1",
      steps: 28,
      cfg: 7,
      seed: "1234",
      postprocess: true,
      count: 3,
      mesh: "later",
    });
  });

  it("grava e relê o arquivo do fluxo", () => {
    const text = serializeFlowFile({
      name: "NPCs da vila",
      templateId: "tpl-1",
      items: [
        {
          id: "i1",
          prompt: "aldeã com cesta",
          count: 4,
          mesh: "later",
          overrides: { seed: "1234" },
        },
      ],
    });
    expect(text).toContain('"version": 1');
    const parsed = parseFlowFile(text);
    expect(parsed.name).toBe("NPCs da vila");
    expect(parsed.templateId).toBe("tpl-1");
    expect(parsed.items).toEqual([
      {
        id: "i1",
        prompt: "aldeã com cesta",
        count: 4,
        mesh: "later",
        overrides: { seed: "1234" },
      },
    ]);
  });
});
