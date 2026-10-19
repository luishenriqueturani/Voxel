import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { frameSize, type GenerationParams, PromptPanel } from "./PromptPanel";

const params: GenerationParams = {
  resolution: 1024,
  aspect: "1:1",
  width: 1024,
  height: 1024,
  steps: 25,
  cfg: 7,
  seed: "",
  count: 1,
};

function renderPanel(onParamsChange = vi.fn()) {
  render(
    <PromptPanel
      prompt=""
      onPromptChange={vi.fn()}
      negativePrompt=""
      onNegativePromptChange={vi.fn()}
      models={[]}
      modelId=""
      onModelChange={vi.fn()}
      params={params}
      onParamsChange={onParamsChange}
      outputMode="2d"
      onOutputModeChange={vi.fn()}
      generating={false}
      onGenerate={vi.fn()}
    />,
  );
  return onParamsChange;
}

describe("frameSize", () => {
  it("encaixa a proporção na grade de 64, com o lado longo na resolução", () => {
    expect(frameSize(1024, "16:9")).toEqual({ width: 1024, height: 576 });
    expect(frameSize(1024, "9:16")).toEqual({ width: 576, height: 1024 });
    expect(frameSize(512, "4:3")).toEqual({ width: 512, height: 384 });
    expect(frameSize(512, "3:4")).toEqual({ width: 384, height: 512 });
    expect(frameSize(768, "16:9")).toEqual({ width: 768, height: 448 });
  });
});

describe("PromptPanel", () => {
  it("separa as proporções horizontais e verticais", async () => {
    const user = userEvent.setup();
    const onParamsChange = renderPanel();
    await user.click(screen.getByRole("button", { name: "Configurações do modelo" }));

    const wide = screen.getByRole("option", { name: "16:9" });
    const tall = screen.getByRole("option", { name: "9:16" });
    expect(wide.parentElement).toHaveAttribute("label", "Horizontal");
    expect(tall.parentElement).toHaveAttribute("label", "Vertical");
    expect(screen.getByRole("option", { name: "1:1" }).parentElement?.tagName).toBe("SELECT");

    await user.selectOptions(screen.getByRole("combobox", { name: "Proporção" }), "16:9");
    expect(onParamsChange).toHaveBeenCalledWith(
      expect.objectContaining({ aspect: "16:9", width: 1024, height: 576 }),
    );

    const stepsTip = screen.getByText(/Mais passos detalham mais/).closest("[role='tooltip']");
    expect(stepsTip).toHaveClass("invisible");
    await user.click(screen.getByRole("button", { name: "Sobre Passos" }));
    expect(stepsTip).toHaveClass("visible");
    expect(stepsTip).not.toHaveClass("invisible");

    await user.selectOptions(screen.getByRole("combobox", { name: "Proporção" }), "9:16");
    expect(onParamsChange).toHaveBeenCalledWith(
      expect.objectContaining({ aspect: "9:16", width: 576, height: 1024 }),
    );
  });

  it("mostra o loader e desativa o botão enquanto gera", () => {
    render(
      <PromptPanel
        prompt=""
        onPromptChange={vi.fn()}
        negativePrompt=""
        onNegativePromptChange={vi.fn()}
        models={[]}
        modelId=""
        onModelChange={vi.fn()}
        params={params}
        onParamsChange={vi.fn()}
        outputMode="2d"
        onOutputModeChange={vi.fn()}
        generating
        onGenerate={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Gerando…" })).toBeDisabled();
  });
});
