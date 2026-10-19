import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useErrorNotice } from "./useErrorNotice";

const pending = vi.hoisted(() => ({
  resolve: undefined as ((unlisten: () => void) => void) | undefined,
}));

vi.mock("../native/errors", () => ({
  listenToErrors: vi.fn(
    () =>
      new Promise<() => void>((resolve) => {
        pending.resolve = resolve;
      }),
  ),
}));

describe("useErrorNotice", () => {
  it("cancela a escuta se o componente sair antes dela começar", async () => {
    const stop = vi.fn();
    const { unmount } = renderHook(() => useErrorNotice(() => undefined));

    unmount();
    pending.resolve?.(() => stop());
    await Promise.resolve();

    expect(stop).toHaveBeenCalledOnce();
  });
});
