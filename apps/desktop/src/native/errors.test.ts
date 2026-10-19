import { listen } from "@tauri-apps/api/event";
import { describe, expect, it, vi } from "vitest";
import { ERROR_EVENT, type ErrorNotice, listenToErrors } from "./errors";

const notice: ErrorNotice = {
  summaryKey: "error.summary.unknown",
  log: "boom",
  path: "/tmp/erro.log",
};

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, onEvent: (event: { payload: ErrorNotice }) => void) => {
    onEvent({ payload: notice });
    return vi.fn();
  }),
}));

describe("erros no Tauri", () => {
  it("escuta o evento que abre o modal", async () => {
    const onNotice = vi.fn();
    const stop = await listenToErrors(onNotice);

    expect(listen).toHaveBeenCalledWith(ERROR_EVENT, expect.any(Function));
    expect(onNotice).toHaveBeenCalledWith(notice);
    expect(stop).toBeTypeOf("function");
  });
});
