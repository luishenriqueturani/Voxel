import { describe, expect, it } from "vitest";
import {
  blankItem,
  clampItemCount,
  defaultMeshMode,
  meshForOutput,
  moveItem,
  needsAllConfirmation,
  outputOf,
  parseFlowFile,
  resolveItem,
  serializeFlowFile,
} from "./flow";

describe("fluxo", () => {
  it("deixa o 3D desligado quando há várias imagens", () => {
    expect(defaultMeshMode(1)).toBe("none");
    expect(defaultMeshMode(4)).toBe("none");
    expect(needsAllConfirmation(1, "all")).toBe(false);
    expect(needsAllConfirmation(4, "all")).toBe(true);
    expect(needsAllConfirmation(4, "later")).toBe(false);
    expect(blankItem("a", 3).mesh).toBe("none");
    expect(outputOf("none")).toBe("2d");
    expect(outputOf("later")).toBe("3d");
    expect(meshForOutput("2d", 4)).toBe("none");
    expect(meshForOutput("3d", 1)).toBe("all");
    expect(meshForOutput("3d", 4)).toBe("later");
  });

  it("reordena, limita a quantidade e aplica a sobrescrita do item", () => {
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    expect(clampItemCount(0)).toBe(1);
    expect(clampItemCount(99)).toBe(64);
    const item = {
      ...blankItem("i1", 3),
      prompt: "ferreiro",
      mesh: "later" as const,
      overrides: { steps: 28, postprocess: true },
    };
    expect(resolveItem(item)).toEqual({
      prompt: "ferreiro",
      negative: "",
      model2d: "sd15",
      model3d: "triposr",
      resolution: 512,
      aspect: "1:1",
      steps: 28,
      cfg: 7,
      seed: "",
      postprocess: true,
      count: 3,
      mesh: "later",
    });
    expect(
      resolveItem(
        { ...item, overrides: { ...item.overrides, negative: "borrado" } },
        { body: "T-pose", negative: "fundo", placement: "after" },
      ),
    ).toEqual(
      expect.objectContaining({
        prompt: "ferreiro, T-pose",
        negative: "fundo, borrado",
      }),
    );
  });

  it("grava e relê o arquivo do fluxo", () => {
    const text = serializeFlowFile({
      name: "NPCs da vila",
      templateId: null,
      items: [
        {
          id: "i1",
          prompt: "aldeã com cesta",
          count: 4,
          mesh: "later",
          templateId: "tpl-1",
          overrides: { seed: "1234" },
        },
      ],
    });
    expect(text).toContain('"version": 1');
    const parsed = parseFlowFile(text);
    expect(parsed.name).toBe("NPCs da vila");
    expect(parsed.items).toEqual([
      {
        id: "i1",
        prompt: "aldeã com cesta",
        count: 4,
        mesh: "later",
        templateId: "tpl-1",
        overrides: { seed: "1234" },
      },
    ]);
    const inherited = parseFlowFile(
      JSON.stringify({
        version: 1,
        name: "Antigo",
        templateId: "tpl-old",
        items: [{ id: "i1", prompt: "guarda", count: 1, mesh: "none", overrides: {} }],
      }),
    );
    expect(inherited.items[0]?.templateId).toBe("tpl-old");
  });
});
