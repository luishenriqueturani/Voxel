import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DownloadProgress, HubPackage, ListedModel } from "../../native/models";
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
    code: "requires_cuda",
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

function lone(
  repo: string,
  filename: string,
  extra: { license?: string; bytes?: number; sha256?: string; gated?: boolean } = {},
): HubPackage {
  return {
    repo,
    name: filename,
    folder: "",
    license: extra.license ?? "mit",
    gated: extra.gated ?? false,
    opener: filename,
    directory: false,
    marker: "",
    parts: [
      {
        repo,
        filename,
        bytes: extra.bytes ?? 2048,
        sha256: extra.sha256 ?? "aa",
        role: "file",
        shardIndex: null,
        shardCount: null,
      },
    ],
  };
}

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
    const downloadedHeading = screen.getByRole("heading", { name: "Baixados" });
    const downloadedColumn = downloadedHeading.closest(".overflow-y-auto");
    expect(downloadedColumn).not.toBeNull();
    expect(downloadedHeading.parentElement).not.toHaveClass("max-h-72");
    const search = screen.getByRole("textbox", { name: "Buscar" });
    const searchFrame = search.closest("form")?.parentElement?.querySelector(".overflow-y-auto");
    expect(searchFrame).toHaveClass("flex-1");
    expect(searchFrame).not.toHaveClass("max-h-72");
    expect(searchFrame).not.toBe(downloadedColumn);
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
    expect(screen.getByText(/recurso que esta máquina não tem/)).toBeInTheDocument();
    const confirm = screen.getByRole("button", { name: "Baixar mesmo assim" });
    expect(confirm).toBeDisabled();
    expect(onDownload).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole("checkbox", {
        name: "Entendi que este modelo pode não rodar neste computador.",
      }),
    );
    await user.click(confirm);
    expect(onDownload).toHaveBeenCalledWith("huge");
  });

  it("mostra a ressalva de memória e baixa sem confirmação extra", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    const caveat: ListedModel = {
      id: "sdxl",
      name: "SDXL",
      license: "OpenRAIL++",
      bytes: 7_000_000_000,
      filename: "sdxl.safetensors",
      installed: false,
      catalog: true,
      compatibility: "caveat",
      reason: {
        code: "needs_vram",
        requiredBytes: 16_000_000_000,
        availableBytes: 8_000_000_000,
      },
      architecture: null,
      kind: "image",
    };
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[compatible, caveat]}
        onDownload={onDownload}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
      />,
    );
    expect(screen.getByText("SDXL")).toBeInTheDocument();
    expect(screen.getByText(/Com ressalvas — Requer .* de VRAM/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Baixar SDXL" }));
    expect(screen.getByRole("button", { name: "Confirmar download" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "Baixar mesmo assim" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirmar download" }));
    expect(onDownload).toHaveBeenCalledWith("sdxl");
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

  it("mostra o quanto já foi baixado e deixa enfileirar outro", async () => {
    const user = userEvent.setup();
    renderSection(vi.fn(), vi.fn(), { modelId: "sd15", received: 40, total: 100 });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText("40 B de 100 B")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Baixar Stable Diffusion 1.5" })).toBeNull();
    await user.click(screen.getByRole("checkbox", { name: "Mostrar modelos incompatíveis" }));
    expect(screen.getByRole("button", { name: "Baixar Enorme" })).toBeEnabled();
  });

  it("mostra os recomendados e deixa a busca fora do filtro", async () => {
    const user = userEvent.setup();
    const onDownloadHub = vi.fn();
    const files: HubPackage[] = [
      lone("lab/gato", "peso.safetensors", { license: "mit", bytes: 2048, sha256: "aa" }),
      lone("lab/gato", "v2/outro.gguf", { license: "apache-2.0", bytes: 4096, sha256: "" }),
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

  it("mostra as partes de um pacote e baixa todas de uma vez", async () => {
    const user = userEvent.setup();
    const onDownloadHub = vi.fn();
    const pkg: HubPackage = {
      repo: "lab/gato",
      name: "gato · merged-model",
      folder: "merged-model",
      license: "mit",
      gated: false,
      opener: "merged-model",
      directory: true,
      marker: "voxel-package.json",
      parts: [
        {
          repo: "lab/gato",
          filename: "merged-model/text_encoder/model-00002-of-00004.safetensors",
          bytes: 5 * 1024 * 1024,
          sha256: "aa",
          role: "text_encoder",
          shardIndex: 2,
          shardCount: 4,
        },
        {
          repo: "lab/gato",
          filename: "merged-model/vae/diffusion_pytorch_model.safetensors",
          bytes: 1024,
          sha256: "",
          role: "vae",
          shardIndex: null,
          shardCount: null,
        },
      ],
    };
    render(
      <ModelsSection
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onSearchHub={vi.fn().mockResolvedValue([pkg])}
        onDownloadHub={onDownloadHub}
      />,
    );
    await user.type(screen.getByRole("textbox", { name: "Buscar" }), "gato");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    expect(await screen.findByText("gato · merged-model")).toBeInTheDocument();
    expect(screen.getByText("merged-model")).toBeInTheDocument();
    expect(screen.getByText("text encoder · parte 2 de 4 · 5 MiB")).toBeInTheDocument();
    expect(screen.getByText("VAE · 1024 B")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /Baixar/ })).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: "Baixar gato · merged-model" }));
    await user.click(screen.getByRole("button", { name: "Confirmar download" }));
    expect(onDownloadHub).toHaveBeenCalledTimes(1);
    expect(onDownloadHub).toHaveBeenCalledWith(pkg);
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
        onSearchHub={vi
          .fn()
          .mockResolvedValue([
            lone("lab/gato", "peso.safetensors", { bytes: 2048, sha256: "aa", license: "mit" }),
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
          lone("lab/fechado", "fechado.safetensors", {
            bytes: 10,
            sha256: "",
            license: "other",
            gated: true,
          }),
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

  it("mostra os bytes já lidos durante a verificação", () => {
    renderSection(vi.fn(), vi.fn(), {
      modelId: "sd15",
      received: 20,
      total: 80,
      phase: "verify",
    });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25");
    expect(screen.getByText("Verificando 20 B de 80 B")).toBeInTheDocument();
  });
});
