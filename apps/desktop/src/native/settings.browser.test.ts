import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { loadStoredSettings, saveStoredSettings } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
  invoke: vi.fn(),
}));

describe("configurações fora do Tauri", () => {
  it("não consulta o banco", async () => {
    await expect(loadStoredSettings()).resolves.toBeNull();
    await saveStoredSettings({ language: "en" });
    expect(invoke).not.toHaveBeenCalled();
  });
});
