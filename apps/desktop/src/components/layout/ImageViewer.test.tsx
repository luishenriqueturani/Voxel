import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ImageViewer } from "./ImageViewer";

describe("ImageViewer", () => {
  it("aproxima com ctrl e scroll, arrasta com o botão do meio, copia, salva e regera", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    const onCopy = vi.fn();
    const onRegenerate = vi.fn();
    render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={2}
        total={4}
        canSave
        canRegenerate
        onSave={onSave}
        onCopy={onCopy}
        onRegenerate={onRegenerate}
      />,
    );

    const stage = screen.getByTestId("viewer-stage");
    expect(screen.getByText("Zoom 100%")).toBeInTheDocument();
    fireEvent.wheel(stage, { ctrlKey: true, deltaY: -100 });
    expect(screen.getByText("Zoom 110%")).toBeInTheDocument();
    fireEvent.wheel(stage, { deltaY: -100 });
    expect(screen.getByText("Zoom 110%")).toBeInTheDocument();

    fireEvent.pointerDown(stage, { button: 1, clientX: 10, clientY: 10, pointerId: 1 });
    fireEvent.pointerMove(stage, { clientX: 30, clientY: 50, pointerId: 1 });
    expect(screen.getByRole("img").getAttribute("style")).toContain("20px");
    expect(screen.getByRole("img").getAttribute("style")).toContain("40px");

    await user.click(screen.getByRole("button", { name: "Salvar" }));
    await user.click(screen.getByRole("button", { name: "Copiar imagem" }));
    await user.click(screen.getByRole("button", { name: "Regerar com a mesma seed" }));
    expect(onSave).toHaveBeenCalledOnce();
    expect(onCopy).toHaveBeenCalledOnce();
    expect(onRegenerate).toHaveBeenCalledOnce();
  });

  it("volta e avança entre as imagens prontas", async () => {
    const user = userEvent.setup();
    const onPrevious = vi.fn();
    const onNext = vi.fn();
    const props = {
      step: 0,
      total: 0,
      canSave: true,
      canRegenerate: true,
      count: 2,
      onPrevious,
      onNext,
      onSave: vi.fn(),
      onCopy: vi.fn(),
      onRegenerate: vi.fn(),
    };
    const view = render(<ImageViewer {...props} src="data:image/png;base64,aaaa" index={0} />);

    expect(screen.getByText("1 / 2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Anterior" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Próxima" }));
    expect(onNext).toHaveBeenCalledOnce();

    view.rerender(<ImageViewer {...props} src="data:image/png;base64,bbbb" index={1} />);
    expect(screen.getByRole("img")).toHaveAttribute("src", "data:image/png;base64,bbbb");
    expect(screen.getByText("2 / 2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Próxima" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Anterior" }));
    expect(onPrevious).toHaveBeenCalledOnce();
  });
});
