import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
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

function View({ shown, sourcePath }: { shown: string | null; sourcePath: string | null }) {
  const editRef = useRef<EditPayload>(blankEdit);
  return (
    <WorkspaceView
      prompt="um gato"
      onPromptChange={vi.fn()}
      negativePrompt=""
      onNegativePromptChange={vi.fn()}
      models={[]}
      modelId="sd15"
      onModelChange={vi.fn()}
      params={params}
      onParamsChange={vi.fn()}
      outputMode="2d"
      onOutputModeChange={vi.fn()}
      generating={false}
      locked={false}
      onGenerate={vi.fn()}
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
});
