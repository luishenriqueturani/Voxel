import { openPath } from "@tauri-apps/plugin-opener";
import { describe, expect, it, vi } from "vitest";
import { openLocalPath } from "./files";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: vi.fn(),
}));

describe("abrir arquivo fora do Tauri", () => {
  it("não chama o sistema", async () => {
    await openLocalPath("/tmp/erro.log");
    expect(openPath).not.toHaveBeenCalled();
  });
});
