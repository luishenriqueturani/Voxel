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
});
