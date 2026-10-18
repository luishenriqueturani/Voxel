import { describe, expect, it, vi } from "vitest";
import { listenToMenu, syncNativeMenu } from "./menu";

const api = vi.hoisted(() => ({
  invoke: vi.fn(async () => undefined),
  listen: vi.fn(async (_event: string, onEvent: (event: { payload: string }) => void) => {
    onEvent({ payload: "edit.preferences" });
    return vi.fn();
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: api.invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: api.listen,
}));

describe("menu nativo dentro do Tauri", () => {
  it("envia os rótulos e escuta as ações", async () => {
    const onAction = vi.fn();

    await syncNativeMenu({ file: "Arquivo" });
    const stop = await listenToMenu(onAction);

    expect(api.invoke).toHaveBeenCalledWith("set_menu_labels", { labels: { file: "Arquivo" } });
    expect(onAction).toHaveBeenCalledWith("edit.preferences");
    expect(stop).toBeTypeOf("function");
  });
});
