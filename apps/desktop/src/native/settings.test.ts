import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadStoredSettings, saveStoredSettings } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(async () => ({ language: "en", theme: "dark" })),
}));

describe("configurações no Tauri", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
  });

  it("lê o banco", async () => {
    await expect(loadStoredSettings()).resolves.toEqual({ language: "en", theme: "dark" });
    expect(invoke).toHaveBeenCalledWith("get_settings");
  });

  it("grava só o que mudou", async () => {
    await saveStoredSettings({ theme: "light" });
    expect(invoke).toHaveBeenCalledWith("update_settings", { update: { theme: "light" } });
  });
});
