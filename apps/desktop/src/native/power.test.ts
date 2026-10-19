import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadPowerNotice } from "./power";

const tauri = vi.hoisted(() => ({ inside: true }));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => tauri.inside,
  invoke: vi.fn(async () => "NVIDIA GeForce RTX 5090"),
}));

describe("aviso de power limit", () => {
  beforeEach(() => {
    tauri.inside = true;
    vi.mocked(invoke).mockClear();
    vi.mocked(invoke).mockResolvedValue("NVIDIA GeForce RTX 5090");
  });

  it("lê o nome da placa", async () => {
    await expect(loadPowerNotice()).resolves.toBe("NVIDIA GeForce RTX 5090");
    expect(invoke).toHaveBeenCalledWith("power_limit_notice");
  });

  it("não mostra nada fora do Tauri, sem placa ou com resposta de outro comando", async () => {
    tauri.inside = false;
    await expect(loadPowerNotice()).resolves.toBeNull();
    expect(invoke).not.toHaveBeenCalled();

    tauri.inside = true;
    vi.mocked(invoke).mockResolvedValue(null);
    await expect(loadPowerNotice()).resolves.toBeNull();

    vi.mocked(invoke).mockResolvedValue({ language: "en", theme: "dark" });
    await expect(loadPowerNotice()).resolves.toBeNull();

    vi.mocked(invoke).mockResolvedValue("");
    await expect(loadPowerNotice()).resolves.toBeNull();
  });
});
