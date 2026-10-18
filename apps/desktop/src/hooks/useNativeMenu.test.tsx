import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useNativeMenu } from "./useNativeMenu";

const pending = vi.hoisted(() => ({
  resolve: undefined as ((unlisten: () => void) => void) | undefined,
}));

vi.mock("../native/menu", () => ({
  syncNativeMenu: vi.fn(async () => undefined),
  listenToMenu: vi.fn(
    () =>
      new Promise<() => void>((resolve) => {
        pending.resolve = resolve;
      }),
  ),
}));

describe("useNativeMenu", () => {
  it("cancela a escuta se o componente sair antes dela começar", async () => {
    const stop = vi.fn();
    const { unmount } = renderHook(() => useNativeMenu(() => undefined));

    unmount();
    pending.resolve?.(() => stop());
    await Promise.resolve();

    expect(stop).toHaveBeenCalledOnce();
  });
});
