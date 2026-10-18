import { describe, expect, it, vi } from "vitest";
import { listenToMenu, syncNativeMenu } from "./menu";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("menu nativo fora do Tauri", () => {
  it("não tenta falar com o processo Rust", async () => {
    await expect(syncNativeMenu({ file: "Arquivo" })).resolves.toBeUndefined();
    const stop = await listenToMenu(() => undefined);
    expect(stop()).toBeUndefined();
  });
});
