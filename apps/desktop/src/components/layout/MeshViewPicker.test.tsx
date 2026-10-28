import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { MeshView } from "../../lib/meshViews";
import { MeshViewPicker } from "./MeshViewPicker";

const views: MeshView[] = [
  { path: "/tmp/frente.png", src: "data:image/png;base64,aa", seed: 1 },
  { path: "/tmp/lado.png", src: "data:image/png;base64,bb", seed: 2 },
  { path: "/tmp/tras.png", src: "data:image/png;base64,cc", seed: 3 },
];

function renderPicker(primaryPath = "/tmp/tras.png") {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  render(
    <MeshViewPicker
      views={views}
      primaryPath={primaryPath}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  return { onConfirm, onCancel };
}

describe("MeshViewPicker", () => {
  it("reordena, tira uma vista e confirma a principal", async () => {
    const user = userEvent.setup();
    const { onConfirm } = renderPicker();

    expect(screen.getByRole("radio", { name: "Vista principal de tras.png" })).toBeChecked();
    expect(screen.getByRole("button", { name: "Subir frente.png" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Subir tras.png" }));
    await user.click(screen.getByRole("checkbox", { name: "Usar lado.png" }));
    await user.click(screen.getByRole("radio", { name: "Vista principal de frente.png" }));
    await user.click(screen.getByRole("button", { name: "Gerar malha" }));

    expect(onConfirm).toHaveBeenCalledWith([views[0], views[2]], "/tmp/frente.png");
  });

  it("passa a principal para a próxima vista incluída", async () => {
    const user = userEvent.setup();
    renderPicker("/tmp/frente.png");

    await user.click(screen.getByRole("checkbox", { name: "Usar frente.png" }));
    expect(screen.getByRole("radio", { name: "Vista principal de lado.png" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Vista principal de frente.png" })).toBeDisabled();

    await user.click(screen.getByRole("checkbox", { name: "Usar lado.png" }));
    await user.click(screen.getByRole("checkbox", { name: "Usar tras.png" }));
    expect(screen.getByRole("button", { name: "Gerar malha" })).toBeDisabled();
  });

  it("cancelar não confirma a malha", async () => {
    const user = userEvent.setup();
    const { onConfirm, onCancel } = renderPicker();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onConfirm).not.toHaveBeenCalled();

    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(2);
  });

  it("explica a vista principal e aceita disco, geração e arraste", async () => {
    const user = userEvent.setup();
    const onAddPaths = vi.fn();
    const onAddFromDisk = vi.fn();
    const onAddCurrent = vi.fn();
    const props = {
      primaryPath: "/tmp/frente.png",
      canAddCurrent: true,
      onAddPaths,
      onAddFromDisk,
      onAddCurrent,
      onConfirm: vi.fn(),
      onCancel: vi.fn(),
    };
    const { rerender } = render(<MeshViewPicker {...props} views={views.slice(0, 1)} />);

    expect(screen.getByText(/reconstrói a vista principal/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Do disco" }));
    await user.click(screen.getByRole("button", { name: "Da geração" }));
    expect(onAddFromDisk).toHaveBeenCalledOnce();
    expect(onAddCurrent).toHaveBeenCalledOnce();

    fireEvent.drop(screen.getByRole("dialog"), {
      dataTransfer: { getData: () => "/tmp/notas.txt /tmp/lado.png" },
    });
    expect(onAddPaths).toHaveBeenCalledWith(["/tmp/lado.png"]);

    rerender(<MeshViewPicker {...props} views={views.slice(0, 2)} />);
    expect(screen.getByRole("checkbox", { name: "Usar lado.png" })).toBeChecked();
  });
});
