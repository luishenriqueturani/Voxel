import { beforeEach, describe, expect, it, vi } from "vitest";
import { blankFlow } from "../lib/flow";
import { createFlow, loadFlows, pickFlowToOpen, pickFlowToSave } from "./flows";

const state = vi.hoisted(() => ({
  inside: false,
  invoke: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => state.inside,
  invoke: (...args: unknown[]) => state.invoke(...args),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => state.open(...args),
  save: (...args: unknown[]) => state.save(...args),
}));

describe("fluxos nativos", () => {
  beforeEach(() => {
    state.inside = false;
    state.invoke.mockReset();
    state.open.mockReset();
    state.save.mockReset();
  });

  it("guarda o fluxo na memória fora do Tauri", async () => {
    const created = await createFlow(blankFlow("NPCs", "item-1"));
    expect(created.name).toBe("NPCs");
    expect(created.items[0]?.prompt).toBe("");
    const listed = await loadFlows();
    expect(listed.some((flow) => flow.id === created.id)).toBe(true);
  });

  it("pede o caminho no diálogo quando está no aplicativo", async () => {
    state.inside = true;
    state.open.mockResolvedValue("/tmp/vila.json");
    state.save.mockResolvedValue("/tmp/saida.json");
    await expect(pickFlowToOpen()).resolves.toBe("/tmp/vila.json");
    await expect(pickFlowToSave(null)).resolves.toBe("/tmp/saida.json");
    expect(state.open).toHaveBeenCalledWith(
      expect.objectContaining({ filters: [{ name: "Fluxo", extensions: ["json"] }] }),
    );
  });
});
