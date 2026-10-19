import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ErrorModal } from "./ErrorModal";

const props = {
  summary: "O processo de geração foi encerrado pelo driver da GPU.",
  log: "HSA_STATUS_ERROR_INVALID_ISA\nmiopen Asm",
  copied: false,
  onCopy: vi.fn(),
  onOpenLog: vi.fn(),
  onOpenDiagnosis: vi.fn(),
  onClose: vi.fn(),
};

describe("ErrorModal", () => {
  it("mostra o resumo, o log rolável e as ações", async () => {
    const user = userEvent.setup();
    const onCopy = vi.fn();
    const onOpenLog = vi.fn();
    const onOpenDiagnosis = vi.fn();
    const onClose = vi.fn();
    render(
      <ErrorModal
        {...props}
        onCopy={onCopy}
        onOpenLog={onOpenLog}
        onOpenDiagnosis={onOpenDiagnosis}
        onClose={onClose}
      />,
    );

    expect(screen.getByRole("dialog", { name: "A geração falhou" })).toBeInTheDocument();
    expect(screen.getByText(props.summary)).toBeInTheDocument();
    const log = screen.getByLabelText("Log");
    expect(log).toHaveClass("overflow-auto");
    expect(log).toHaveTextContent("HSA_STATUS_ERROR_INVALID_ISA");

    await user.click(screen.getByRole("button", { name: "Copiar" }));
    await user.click(screen.getByRole("button", { name: "Abrir arquivo de log" }));
    await user.click(screen.getByRole("button", { name: "Abrir diagnóstico" }));
    await user.click(screen.getByRole("button", { name: "Fechar" }));

    expect(onCopy).toHaveBeenCalledOnce();
    expect(onOpenLog).toHaveBeenCalledOnce();
    expect(onOpenDiagnosis).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("troca o botão depois de copiar e fecha pelo fundo ou pelo Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<ErrorModal {...props} copied onClose={onClose} />);

    expect(screen.getByRole("button", { name: "Copiado" })).toBeInTheDocument();
    await user.click(screen.getByRole("dialog"));
    expect(onClose).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Fechar o aviso" }));
    expect(onClose).toHaveBeenCalledOnce();

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
