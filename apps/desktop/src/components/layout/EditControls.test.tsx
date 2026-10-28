import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EditControls, editReady } from "./EditControls";

describe("EditControls", () => {
  it("mostra a intensidade ao variar e exige imagem e máscara para pintar", async () => {
    const user = userEvent.setup();
    const onModeChange = vi.fn();
    const { rerender } = render(
      <EditControls
        mode="new"
        onModeChange={onModeChange}
        strength={0.75}
        onStrengthChange={vi.fn()}
        hasSource={false}
        hasMask={false}
      />,
    );
    expect(screen.queryByRole("slider", { name: "Intensidade" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "Variar" }));
    expect(onModeChange).toHaveBeenCalledWith("vary");

    rerender(
      <EditControls
        mode="vary"
        onModeChange={onModeChange}
        strength={0.4}
        onStrengthChange={vi.fn()}
        hasSource={false}
        hasMask={false}
      />,
    );
    expect(screen.getByRole("textbox", { name: "O que mudar" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Intensidade" })).toHaveValue("0.4");
    expect(screen.getByText("40%")).toBeInTheDocument();
    expect(screen.getByText(/Abra ou gere uma imagem/)).toBeInTheDocument();
    expect(editReady("vary", false, false)).toBe(false);
    expect(editReady("vary", true, false)).toBe(true);
    expect(editReady("inpaint", true, false)).toBe(false);
    expect(editReady("inpaint", true, true)).toBe(true);
    expect(editReady("tile", false, false)).toBe(true);
  });

  it("descreve cada modo e usa a intensidade da referência", async () => {
    const user = userEvent.setup();
    const onPickReference = vi.fn();
    const onClearReference = vi.fn();
    const props = {
      onModeChange: vi.fn(),
      strength: 0.75,
      onStrengthChange: vi.fn(),
      hasSource: false,
      hasMask: false,
      onPickReference,
      onClearReference,
    };
    const { rerender } = render(<EditControls {...props} mode="new" referencePath={null} />);

    expect(screen.getByRole("radio", { name: "Nova" })).toHaveAccessibleDescription(
      /sem usar a imagem aberta/,
    );
    expect(screen.getByRole("radio", { name: "Variar" })).toHaveAccessibleDescription(
      /imagem atual/,
    );
    expect(screen.getByRole("radio", { name: "Pintar" })).toHaveAccessibleDescription(
      /área pintada/,
    );
    expect(screen.getByRole("radio", { name: "Textura" })).toHaveAccessibleDescription(
      /sem emenda/,
    );
    expect(document.getElementById("edit-tip-vary")).toHaveClass("group-hover:visible");
    expect(document.getElementById("edit-tip-vary")).toHaveClass("group-focus-within:visible");
    await user.click(screen.getByRole("button", { name: "Escolher imagem" }));
    expect(onPickReference).toHaveBeenCalledOnce();

    rerender(<EditControls {...props} mode="new" referencePath="/tmp/gato.png" />);
    expect(screen.getByRole("slider", { name: "Intensidade" })).toBeInTheDocument();
    expect(screen.getByText("gato.png")).toBeInTheDocument();
    expect(screen.getByText(/parte deste arquivo/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tirar referência" }));
    expect(onClearReference).toHaveBeenCalledOnce();
    expect(editReady("vary", false, false, true)).toBe(true);
  });
});
