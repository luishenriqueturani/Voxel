import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DownloadProgress, ListedModel } from "../../native/models";
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

function renderSection(
  onDownload = vi.fn(),
  onRemove = vi.fn(),
  progress: DownloadProgress | null = null,
) {
  const onOpenFolder = vi.fn();
  const onImport = vi.fn();
  render(
    <ModelsSection
      hfTokenConfigured={false}
      onHfTokenSave={vi.fn()}
      onHfTokenClear={vi.fn()}
      models={[compatible, incompatible, imported]}
      progress={progress}
      onDownload={onDownload}
      onRemove={onRemove}
      onOpenFolder={onOpenFolder}
      onImport={onImport}
    />,
  );
  return { onOpenFolder, onImport };
}

describe("gerenciador de modelos", () => {
  it("mantém um modelo baixado incompatível visível para remover", () => {
    const hugeDownload: ListedModel = {
      ...incompatible,
      id: "qwen.gguf",
      name: "qwen.gguf",
      filename: "qwen.gguf",
      catalog: false,
      installed: true,
      architecture: "qwen_image21",
    };
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[incompatible, hugeDownload]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
      />,
    );
    expect(screen.getByRole("heading", { name: "Baixados" })).toBeInTheDocument();
    expect(screen.getByText("qwen.gguf")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Remover qwen.gguf" })).toBeInTheDocument();
    expect(screen.queryByText("Enorme")).not.toBeInTheDocument();
  });

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

  it("mostra o quanto já foi baixado e trava os outros downloads", () => {
    renderSection(vi.fn(), vi.fn(), { modelId: "sd15", received: 40, total: 100 });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText("40 B de 100 B")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Baixar Stable Diffusion 1.5" })).toBeNull();
  });

  it("mostra os recomendados e deixa a busca fora do filtro", async () => {
    const user = userEvent.setup();
    const onDownloadHub = vi.fn();
    const files = [
      {
        repo: "lab/gato",
        filename: "peso.safetensors",
        bytes: 2048,
        sha256: "aa",
        license: "mit",
        gated: false,
      },
      {
        repo: "lab/gato",
        filename: "v2/outro.gguf",
        bytes: 4096,
        sha256: "",
        license: "apache-2.0",
        gated: false,
      },
    ];
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[compatible, incompatible, imported]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onSearchHub={vi.fn().mockResolvedValue(files)}
        onDownloadHub={onDownloadHub}
      />,
    );
    expect(screen.getByRole("heading", { name: "Recomendados" })).toBeInTheDocument();
    expect(screen.queryByText("Enorme")).not.toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: "Buscar" }), "gato");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    expect(await screen.findByText("peso.safetensors")).toBeInTheDocument();
    expect(screen.getByText("v2/outro.gguf")).toBeInTheDocument();
    expect(screen.getByText("mit")).toBeInTheDocument();
    expect(screen.getByText("apache-2.0")).toBeInTheDocument();
    expect(screen.getAllByText("Não medido")).toHaveLength(2);
    expect(screen.queryByText("Enorme")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Baixar peso.safetensors" }));
    expect(screen.getByRole("dialog").parentElement).toHaveClass("fixed");
    expect(screen.getByText(/não foi medido neste computador/)).toBeInTheDocument();
    expect(onDownloadHub).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Confirmar download" }));
    expect(onDownloadHub).toHaveBeenCalledWith(files[0]);
  });

  it("mantém o progresso do arquivo visível enquanto a lista rola", async () => {
    const user = userEvent.setup();
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[]}
        progress={{ modelId: "lab/gato/peso.safetensors", received: 40, total: 100 }}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onSearchHub={vi.fn().mockResolvedValue([
          {
            repo: "lab/gato",
            filename: "peso.safetensors",
            bytes: 2048,
            sha256: "aa",
            license: "mit",
            gated: false,
          },
        ])}
        onDownloadHub={vi.fn()}
      />,
    );
    await user.type(screen.getByRole("textbox", { name: "Buscar" }), "gato");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    expect(await screen.findAllByRole("progressbar")).toHaveLength(2);
    expect(screen.getAllByText("40 B de 100 B")).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Baixar peso.safetensors" })).toBeNull();
  });

  it("não baixa um repositório restrito sem token", async () => {
    const user = userEvent.setup();
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[compatible]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onSearchHub={vi.fn().mockResolvedValue([
          {
            repo: "lab/fechado",
            filename: "fechado.safetensors",
            bytes: 10,
            sha256: "",
            license: "other",
            gated: true,
          },
        ])}
        onDownloadHub={vi.fn()}
      />,
    );
    await user.type(screen.getByRole("textbox", { name: "Buscar" }), "fechado");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    expect(await screen.findByText(/Repositório restrito/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Baixar fechado.safetensors" })).toBeDisabled();
  });

  it("avisa que está conferindo o arquivo quando a transferência acaba", () => {
    renderSection(vi.fn(), vi.fn(), { modelId: "sd15", received: 100, total: 100 });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
    expect(screen.getByText("Verificando o arquivo…")).toBeInTheDocument();
  });
});
