import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

const updates: Array<{ language?: string; theme?: string }> = [];

const menuActions = vi.hoisted(() => ({
  current: undefined as ((id: string) => void) | undefined,
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(
    async (command: string, args?: { update?: { language?: string; theme?: string } }) => {
      if (command === "update_settings" && args?.update) {
        updates.push(args.update);
      }
      return { language: "en", theme: "dark" };
    },
  ),
}));

vi.mock("./native/errors", () => ({
  listenToErrors: vi.fn(async () => () => undefined),
}));

vi.mock("./native/menu", () => ({
  syncNativeMenu: vi.fn(async () => undefined),
  listenToMenu: vi.fn(async (onAction: (id: string) => void) => {
    menuActions.current = onAction;
    return () => undefined;
  }),
}));

describe("App com configurações no banco", () => {
  it("aplica o que está gravado e persiste a troca de tema", async () => {
    const user = userEvent.setup();
    updates.length = 0;
    render(<App />);

    await waitFor(() => {
      expect(document.documentElement.lang).toBe("en");
      expect(document.documentElement).toHaveClass("dark");
    });

    await waitFor(() => expect(menuActions.current).toBeTypeOf("function"));
    const onAction = menuActions.current;
    if (onAction === undefined) {
      throw new Error("menu sem ação");
    }
    await act(() => onAction("edit.preferences"));
    await user.click(screen.getByRole("radio", { name: "Light" }));

    await waitFor(() => expect(updates).toContainEqual({ theme: "light" }));

    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "pt-BR");
    await waitFor(() => expect(updates).toContainEqual({ language: "pt-BR" }));
  });
});
