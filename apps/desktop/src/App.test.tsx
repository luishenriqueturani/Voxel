import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

const menuActions = vi.hoisted(() => ({
  current: undefined as ((id: string) => void) | undefined,
}));

vi.mock("./native/menu", () => ({
  syncNativeMenu: vi.fn(async () => undefined),
  listenToMenu: vi.fn(async (onAction: (id: string) => void) => {
    menuActions.current = onAction;
    return () => undefined;
  }),
}));

describe("App", () => {
  it("mostra o painel, a área principal e a barra de estado", () => {
    render(<App />);

    expect(screen.getByRole("heading", { level: 1, name: "Voxel" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Prompt" })).toHaveFocus();
    expect(screen.getByRole("main", { name: "Área principal" })).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toHaveTextContent("GPU: —");
    expect(screen.getByText("Nenhum prompt recente")).toBeInTheDocument();
  });

  it("mantém o prompt ao gerar com Ctrl+Enter", async () => {
    const user = userEvent.setup();
    render(<App />);

    const prompt = screen.getByRole("textbox", { name: "Prompt" });
    await user.type(prompt, "orc");
    await user.keyboard("{Control>}{Enter}{/Control}");

    expect(prompt).toHaveValue("orc");
  });

  it("abre o prompt negativo e as configurações do modelo", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByText("Prompt negativo"));
    const negative = screen.getByRole("textbox", { name: "Prompt negativo" });
    await user.type(negative, "fundo");
    expect(negative).toHaveValue("fundo");

    await user.click(screen.getByRole("button", { name: "Configurações do modelo" }));
    expect(screen.getByRole("spinbutton", { name: "Passos" })).toHaveValue(25);
  });

  it("alterna 2D e 3D", async () => {
    const user = userEvent.setup();
    render(<App />);

    const threeD = screen.getByRole("radio", { name: "3D" });
    await user.click(threeD);
    expect(threeD).toBeChecked();
    await user.click(screen.getByRole("radio", { name: "2D" }));
    expect(screen.getByRole("radio", { name: "2D" })).toBeChecked();
  });

  it("reage aos itens do menu que já têm função", async () => {
    render(<App />);
    await waitFor(() => expect(menuActions.current).toBeTypeOf("function"));
    const onAction = menuActions.current;
    if (onAction === undefined) {
      throw new Error("menu sem ação");
    }

    await act(() => onAction("edit.preferences"));
    expect(screen.getByRole("heading", { level: 1, name: "Configurações" })).toBeInTheDocument();

    await act(() => onAction("view.theme"));
    expect(document.documentElement).toHaveClass("dark");
    await act(() => onAction("view.theme"));
    expect(document.documentElement).not.toHaveClass("dark");

    await act(() => onAction("generate.run"));
    await act(() => onAction("help.about"));
  });

  it("abre as configurações, troca o idioma e o tema, e volta", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Preferências" }));
    expect(screen.getByRole("heading", { level: 1, name: "Configurações" })).toBeInTheDocument();

    for (const section of ["Geral", "Hardware", "Modelos", "Ambiente 3D", "Execução", "Logs"]) {
      expect(screen.getByRole("button", { name: section })).toBeInTheDocument();
    }

    await user.selectOptions(screen.getByRole("combobox", { name: "Idioma" }), "en");
    expect(await screen.findByRole("heading", { level: 1, name: "Settings" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");

    await user.click(screen.getByRole("radio", { name: "Dark" }));
    expect(document.documentElement).toHaveClass("dark");
    await user.click(screen.getByRole("radio", { name: "Light" }));
    expect(document.documentElement).not.toHaveClass("dark");
    expect(localStorage.getItem("voxel.theme")).toBe("light");
    expect(localStorage.getItem("voxel.language")).toBe("en");

    await user.click(screen.getByRole("button", { name: "Hardware" }));
    expect(screen.getByRole("combobox", { name: "Image backend" })).toBeInTheDocument();
    expect(screen.getByText("Recommended by setup: Vulkan")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("button", { name: "Generate" })).toBeInTheDocument();
  });
});
