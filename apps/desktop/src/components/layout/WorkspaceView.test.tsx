import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { blankEdit, type EditPayload } from "../../lib/edit";
import type { GenerationParams } from "./PromptPanel";
import { WorkspaceView } from "./WorkspaceView";

const params: GenerationParams = {
  resolution: 512,
  aspect: "1:1",
  width: 512,
  height: 512,
  steps: 20,
  cfg: 7,
  seed: "",
  count: 1,
};

afterEach(() => {
  localStorage.removeItem("voxel.modelPresets");
});

function View({
  shown,
  sourcePath,
  modelId = "sd15",
  imageBackend = "vulkan",
  onGenerate = vi.fn(),
  onParamsChange = vi.fn(),
  onImageBackendChange = vi.fn(),
}: {
  shown: string | null;
  sourcePath: string | null;
  modelId?: string;
  imageBackend?: string;
  onGenerate?: () => void;
  onParamsChange?: (params: GenerationParams) => void;
  onImageBackendChange?: (backend: string) => void;
}) {
  const editRef = useRef<EditPayload>(blankEdit);
  return (
    <WorkspaceView
      prompt="um gato"
      onPromptChange={vi.fn()}
      negativePrompt=""
      onNegativePromptChange={vi.fn()}
      models={[{ id: modelId, name: modelId }]}
      modelId={modelId}
      onModelChange={vi.fn()}
      params={params}
      onParamsChange={onParamsChange}
      imageBackend={imageBackend}
      onImageBackendChange={onImageBackendChange}
      outputMode="2d"
      onOutputModeChange={vi.fn()}
      generating={false}
      locked={false}
      onGenerate={onGenerate}
      sourcePath={sourcePath}
      editRef={editRef}
      historyRevision={0}
      gallerySignal={0}
      viewerSignal={0}
      onHistoryError={vi.fn()}
      onRestoreHistory={vi.fn()}
      onOpenGeneration={vi.fn()}
      shown={shown}
      step={0}
      total={0}
      canSave={shown !== null}
      canRegenerate={false}
      canRemove={false}
      removing={false}
      transparent={false}
      index={0}
      count={shown === null ? 0 : 1}
      onPrevious={vi.fn()}
      onNext={vi.fn()}
      onSave={vi.fn()}
      onCopy={vi.fn()}
      onRegenerate={vi.fn()}
      onRemoveBackground={vi.fn()}
    />
  );
}

describe("WorkspaceView", () => {
  it("pede intensidade para variar, o pincel para pintar e o mosaico para a textura", async () => {
    const user = userEvent.setup();
    const image = "data:image/png;base64,aaaa";
    const { rerender } = render(<View shown={null} sourcePath={null} />);

    await user.click(screen.getByRole("radio", { name: "Variar" }));
    expect(screen.getByRole("slider", { name: "Intensidade" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Gerar" })).toBeDisabled();

    rerender(<View shown={image} sourcePath="/tmp/origem.png" />);
    expect(screen.getByRole("button", { name: "Gerar" })).toBeEnabled();

    await user.click(screen.getByRole("radio", { name: "Pintar" }));
    expect(screen.getByRole("slider", { name: "Pincel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Desfazer" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Gerar" })).toBeDisabled();

    await user.click(screen.getByRole("radio", { name: "Textura" }));
    const mosaic = await screen.findByTestId("texture-mosaic");
    expect(screen.getByRole("radio", { name: "Textura" })).toBeChecked();
    expect(mosaic.querySelectorAll("img")).toHaveLength(9);
    expect(screen.getByRole("button", { name: "Mosaico" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Gerar" })).toBeEnabled();
  });

  it("pergunta a família antes de gerar um arquivo sem preset", async () => {
    const user = userEvent.setup();
    const onGenerate = vi.fn();
    const onParamsChange = vi.fn();
    const onImageBackendChange = vi.fn();
    render(
      <View
        shown={null}
        sourcePath={null}
        modelId="peso.safetensors"
        onGenerate={onGenerate}
        onParamsChange={onParamsChange}
        onImageBackendChange={onImageBackendChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Gerar" }));
    expect(onGenerate).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog", { name: "Preset do modelo" })).toBeInTheDocument();
    expect(
      screen.getByText("A geração espera a família e o backend deste arquivo."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Resolução" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Passos" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "CFG" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onGenerate).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Gerar" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Família" }), "sdxl");
    await user.selectOptions(screen.getByRole("combobox", { name: "Backend de imagem" }), "rocm");
    await user.click(screen.getByRole("button", { name: "Salvar e gerar" }));
    expect(onParamsChange).not.toHaveBeenCalled();
    expect(onGenerate).toHaveBeenCalledOnce();
    expect(onImageBackendChange).toHaveBeenCalledWith("rocm");
  });

  it("abre o mesmo modal para alterar o preset de um modelo medido", async () => {
    const user = userEvent.setup();
    const onGenerate = vi.fn();
    const onParamsChange = vi.fn();
    render(
      <View
        shown={null}
        sourcePath={null}
        onGenerate={onGenerate}
        onParamsChange={onParamsChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Gerar" }));
    expect(onGenerate).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: "Preset" }));
    expect(screen.getByRole("dialog", { name: "Preset do modelo" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Família" })).toHaveValue("sd15");
    await user.click(screen.getByRole("button", { name: "Salvar" }));
    expect(onGenerate).toHaveBeenCalledOnce();
    expect(onParamsChange).not.toHaveBeenCalled();
  });
});
