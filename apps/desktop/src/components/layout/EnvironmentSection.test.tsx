import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EnvironmentSection } from "./EnvironmentSection";

const absent = {
  phase: "absent",
  percent: null,
  receivedBytes: null,
  totalBytes: null,
  backend: null,
  detail: null,
};

describe("seção do ambiente 3D", () => {
  it("mostra o estado e pede para reinstalar ou remover", async () => {
    const user = userEvent.setup();
    const onReinstall = vi.fn();
    const onRemove = vi.fn();
    const { rerender } = render(
      <EnvironmentSection progress={absent} onReinstall={onReinstall} onRemove={onRemove} />,
    );

    expect(screen.getByText("Estado: não instalado")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Remover" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Reinstalar" }));
    expect(onReinstall).toHaveBeenCalledOnce();

    rerender(
      <EnvironmentSection
        progress={{ ...absent, phase: "ready", backend: "rocm_native" }}
        onReinstall={onReinstall}
        onRemove={onRemove}
      />,
    );
    expect(screen.getByText("Estado: pronto (ROCm)")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Remover" }));
    expect(onRemove).toHaveBeenCalledOnce();
  });

  it("trava os botões enquanto a instalação anda", () => {
    render(
      <EnvironmentSection
        progress={{ ...absent, phase: "preparing", percent: 45 }}
        onReinstall={vi.fn()}
        onRemove={vi.fn()}
      />,
    );

    expect(screen.getByText("Estado: instalando")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reinstalar" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Remover" })).toBeDisabled();
  });
});
