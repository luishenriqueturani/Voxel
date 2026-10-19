import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { resetSetup } from "./native/setup";

const menuActions = vi.hoisted(() => ({
  current: undefined as ((id: string) => void) | undefined,
}));

vi.mock("./native/setup", () => ({
  loadSetupStatus: vi.fn(async () => ({
    completed: false,
    profile: null,
    backend: null,
    steps: [],
  })),
  runSetup: vi.fn(async () => ({
    completed: true,
    profile: "intermediario",
    backend: "vulkan",
    steps: [
      { id: "hardware", state: "done", detail: "AMD Radeon RX 7600 · 32 GiB" },
      { id: "vulkan", state: "passed", detail: "" },
      { id: "profile", state: "done", detail: "intermediario" },
    ],
  })),
  resetSetup: vi.fn(async () => undefined),
  listenToSetup: vi.fn(async () => () => undefined),
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

describe("setup na abertura", () => {
  beforeEach(() => {
    vi.mocked(resetSetup).mockClear();
  });

  it("mostra o progresso e entra no app ao continuar", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Bem-vindo ao Voxel" })).toBeInTheDocument();
    expect(await screen.findByText("Vulkan")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    expect(screen.getByRole("textbox", { name: "Prompt" })).toBeInTheDocument();
  });

  it("refaz o setup pelo menu", async () => {
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Bem-vindo ao Voxel" })).toBeInTheDocument();
    await act(() => menuActions.current?.("help.redoSetup"));
    expect(resetSetup).toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Bem-vindo ao Voxel" })).toBeInTheDocument();
  });
});
