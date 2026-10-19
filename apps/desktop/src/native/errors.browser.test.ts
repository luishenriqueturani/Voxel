import { listen } from "@tauri-apps/api/event";
import { describe, expect, it, vi } from "vitest";
import { listenToErrors } from "./errors";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("erros fora do Tauri", () => {
  it("não escuta o evento", async () => {
    const stop = await listenToErrors(() => undefined);
    stop();
    expect(listen).not.toHaveBeenCalled();
  });
});
