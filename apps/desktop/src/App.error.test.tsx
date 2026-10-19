import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { ErrorNotice } from "./native/errors";
import { openLocalPath } from "./native/files";

const pending = vi.hoisted(() => ({
  current: undefined as ((notice: ErrorNotice) => void) | undefined,
}));

vi.mock("./native/errors", () => ({
  listenToErrors: vi.fn(async (onNotice: (notice: ErrorNotice) => void) => {
    pending.current = onNotice;
    return () => undefined;
  }),
}));

vi.mock("./native/files", () => ({
  openLocalPath: vi.fn(async () => undefined),
}));

const kernel: ErrorNotice = {
  summaryKey: "error.summary.incompatibleKernel",
  log: "HSA_STATUS_ERROR_INVALID_ISA\nmiopen Asm",
  path: "/tmp/voxel/erro.log",
};

async function show(notice: ErrorNotice) {
  render(<App />);
  await waitFor(() => expect(pending.current).toBeTypeOf("function"));
  const onNotice = pending.current;
  if (onNotice === undefined) {
    throw new Error("erro sem escuta");
  }
  await act(() => {
    onNotice(notice);
  });
}

describe("App com modal de erro", () => {
  beforeEach(() => {
    pending.current = undefined;
  });

  it("mostra o resumo, copia o log e abre o arquivo", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
    await show(kernel);

    expect(
      screen.getByText(
        "O processo de geração foi encerrado pelo driver da GPU durante a decodificação da imagem.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Log")).toHaveTextContent("miopen Asm");

    await user.click(screen.getByRole("button", { name: "Copiar" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Copiado" })).toBeInTheDocument(),
    );
    expect(writeText).toHaveBeenCalledWith(kernel.log);

    await user.click(screen.getByRole("button", { name: "Abrir arquivo de log" }));
    expect(openLocalPath).toHaveBeenCalledWith(kernel.path);

    await user.click(screen.getByRole("button", { name: "Fechar" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("abre o diagnóstico e esconde o modal", async () => {
    const user = userEvent.setup();
    await show(kernel);

    await user.click(screen.getByRole("button", { name: "Abrir diagnóstico" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Hardware" })).toBeInTheDocument();
  });

  it("usa o resumo genérico quando a chave é desconhecida e ignora falha ao copiar", async () => {
    const user = userEvent.setup();
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockRejectedValue(new Error("sem área de transferência"));
    await show({ ...kernel, summaryKey: "error.summary.nope" });

    expect(
      screen.getByText("A geração falhou por um motivo que o Voxel ainda não reconhece."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copiar" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(kernel.log));
    expect(screen.queryByRole("button", { name: "Copiado" })).not.toBeInTheDocument();
  });
});
