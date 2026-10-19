import { openPath } from "@tauri-apps/plugin-opener";
import { describe, expect, it, vi } from "vitest";
import { openLocalPath } from "./files";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: vi.fn(async () => undefined),
}));

describe("abrir arquivo no Tauri", () => {
  it("entrega o caminho ao aplicativo padrão", async () => {
    await openLocalPath("/tmp/erro.log");
    expect(openPath).toHaveBeenCalledWith("/tmp/erro.log");
  });
});
