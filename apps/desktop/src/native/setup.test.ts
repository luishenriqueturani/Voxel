import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { listenToSetup, loadSetupStatus, resetSetup, runSetup } from "./setup";

const tauri = vi.hoisted(() => ({ inside: true }));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => tauri.inside,
  invoke: vi.fn(async (command: string) => {
    if (command === "setup_status") {
      return { completed: false, profile: null, backend: null, steps: [] };
    }
    return { completed: true, profile: "intermediario", backend: "vulkan", steps: [] };
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
}));

describe("setup no Tauri", () => {
  beforeEach(() => {
    tauri.inside = true;
    vi.mocked(invoke).mockClear();
  });

  it("lê o estado e reaproveita a mesma execução", async () => {
    await expect(loadSetupStatus()).resolves.toMatchObject({ completed: false });
    const first = runSetup();
    const second = runSetup();
    await expect(first).resolves.toMatchObject({ backend: "vulkan" });
    await expect(second).resolves.toMatchObject({ backend: "vulkan" });
    expect(invoke).toHaveBeenCalledWith("run_setup");
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === "run_setup"),
    ).toHaveLength(1);
  });

  it("refazer limpa a execução em andamento", async () => {
    await resetSetup();
    expect(invoke).toHaveBeenCalledWith("reset_setup");
    await runSetup();
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === "run_setup"),
    ).toHaveLength(1);
  });

  it("ignora resposta que não é o estado do setup", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ language: "en", theme: "dark" });
    await expect(loadSetupStatus()).resolves.toBeNull();
  });

  it("fora do Tauri não chama o processo", async () => {
    tauri.inside = false;
    await expect(loadSetupStatus()).resolves.toBeNull();
    await expect(runSetup()).resolves.toBeNull();
    await resetSetup();
    await expect(listenToSetup(() => undefined)).resolves.toBeTypeOf("function");
    expect(invoke).not.toHaveBeenCalled();
  });
});
