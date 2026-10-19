import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

const pending = vi.hoisted(() => ({
  resolve: undefined as ((value: null) => void) | undefined,
}));

vi.mock("./native/generation", () => ({
  generateImage: vi.fn(
    () =>
      new Promise((resolve) => {
        pending.resolve = resolve;
      }),
  ),
  imageSrc: (preview: string) => preview,
  loadGenerationDefaults: vi.fn(async () => null),
  saveGeneratedImage: vi.fn(async () => undefined),
  copyPreview: vi.fn(async () => undefined),
}));

vi.mock("./native/menu", () => ({
  syncNativeMenu: vi.fn(async () => undefined),
  listenToMenu: vi.fn(async () => () => undefined),
}));

describe("geração", () => {
  it("desativa o botão até a imagem terminar", async () => {
    const user = userEvent.setup();
    render(<App />);

    const generate = screen.getByRole("button", { name: "Gerar" });
    await user.click(generate);

    const busy = screen.getByRole("button", { name: "Gerando…" });
    expect(busy).toBeDisabled();
    expect(busy).toHaveAttribute("aria-busy", "true");

    pending.resolve?.(null);
    expect(await screen.findByRole("button", { name: "Gerar" })).toBeEnabled();
  });
});
