import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ListedModel } from "../../native/models";
import { ModelsSection } from "./ModelsSection";

const compatible: ListedModel = {
  id: "sd15",
  name: "Stable Diffusion 1.5",
  license: "CreativeML OpenRAIL-M",
  bytes: 100,
  filename: "sd15.safetensors",
  installed: false,
  catalog: true,
  compatibility: "compatible",
  reason: null,
  architecture: null,
  kind: "image",
};

const incompatible: ListedModel = {
  id: "huge",
  name: "Enorme",
  license: "Licença de teste",
  bytes: 20_000_000_000,
  filename: "enorme.safetensors",
  installed: false,
  catalog: true,
  compatibility: "incompatible",
  reason: {
    code: "needs_vram",
    requiredBytes: 20_000_000_000,
    availableBytes: 8_000_000_000,
  },
  architecture: null,
  kind: "image",
};

const imported: ListedModel = {
  id: "local.safetensors",
  name: "local.safetensors",
  license: "",
  bytes: 32,
  filename: "local.safetensors",
  installed: true,
  catalog: false,
  compatibility: "compatible",
  reason: null,
  architecture: "stable-diffusion-v1",
  kind: "image",
};

function renderSection(onDownload = vi.fn(), onRemove = vi.fn()) {
  const onOpenFolder = vi.fn();
  const onImport = vi.fn();
  render(
    <ModelsSection
      hfTokenConfigured={false}
      onHfTokenSave={vi.fn()}
      onHfTokenClear={vi.fn()}
      models={[compatible, incompatible, imported]}
      onDownload={onDownload}
      onRemove={onRemove}
      onOpenFolder={onOpenFolder}
      onImport={onImport}
    />,
  );
  return { onOpenFolder, onImport };
}

describe("gerenciador de modelos", () => {
  it("esconde incompatíveis até a opção ser marcada", async () => {
    const user = userEvent.setup();
    renderSection();
    expect(screen.queryByText("Enorme")).not.toBeInTheDocument();
    expect(screen.getByText("Stable Diffusion 1.5")).toBeInTheDocument();
    await user.click(screen.getByRole("checkbox", { name: "Mostrar modelos incompatíveis" }));
    expect(screen.getByText("Enorme")).toBeInTheDocument();
  });

  it("mostra a licença antes de baixar", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    renderSection(onDownload);
    await user.click(screen.getByRole("button", { name: "Baixar Stable Diffusion 1.5" }));
    expect(screen.getByText("CreativeML OpenRAIL-M")).toBeInTheDocument();
    expect(onDownload).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Confirmar download" }));
    expect(onDownload).toHaveBeenCalledWith("sd15");
  });

  it("pede confirmação explícita para um modelo incompatível", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    renderSection(onDownload);
    await user.click(screen.getByRole("checkbox", { name: "Mostrar modelos incompatíveis" }));
    await user.click(screen.getByRole("button", { name: "Baixar Enorme" }));
    expect(screen.getByText("Licença de teste")).toBeInTheDocument();
    expect(screen.getByText(/incompatível com este computador/)).toBeInTheDocument();
    const confirm = screen.getByRole("button", { name: "Baixar mesmo assim" });
    expect(confirm).toBeDisabled();
    expect(onDownload).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole("checkbox", { name: "Entendi que este modelo não roda neste computador." }),
    );
    await user.click(confirm);
    expect(onDownload).toHaveBeenCalledWith("huge");
  });

  it("remove um modelo importado e abre a pasta", async () => {
    const user = userEvent.setup();
    const onRemove = vi.fn();
    const { onOpenFolder, onImport } = renderSection(vi.fn(), onRemove);
    expect(screen.getByText("Arquitetura: stable-diffusion-v1")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Remover local.safetensors" }));
    expect(onRemove).toHaveBeenCalledWith("local.safetensors");
    await user.click(screen.getByRole("button", { name: "Abrir pasta de modelos" }));
    expect(onOpenFolder).toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Importar modelo" }));
    expect(onImport).toHaveBeenCalled();
  });
});
