import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SettingsView } from "./SettingsView";

describe("configurações de hardware", () => {
  it("troca o backend e mostra o recomendado", async () => {
    const user = userEvent.setup();
    const onImageBackendChange = vi.fn();
    render(
      <SettingsView
        section="hardware"
        onSectionChange={vi.fn()}
        theme="light"
        onThemeChange={vi.fn()}
        onLanguageChange={vi.fn()}
        imageBackend="vulkan"
        recommendedBackend="vulkan"
        onImageBackendChange={onImageBackendChange}
        hfTokenConfigured={false}
        onHfTokenSave={vi.fn()}
        onHfTokenClear={vi.fn()}
        models={[]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onBack={vi.fn()}
        environment={null}
        onReinstallEnvironment={vi.fn()}
        onRemoveEnvironment={vi.fn()}
      />,
    );

    expect(screen.getByText("Recomendado pelo setup: Vulkan")).toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "Backend de imagem" }), "cpu");
    expect(onImageBackendChange).toHaveBeenCalledWith("cpu");
  });
});

describe("token do Hugging Face", () => {
  it("grava o token e oferece apagar quando já existe", async () => {
    const user = userEvent.setup();
    const onHfTokenSave = vi.fn();
    const onHfTokenClear = vi.fn();
    render(
      <SettingsView
        section="models"
        onSectionChange={vi.fn()}
        theme="light"
        onThemeChange={vi.fn()}
        onLanguageChange={vi.fn()}
        imageBackend="vulkan"
        recommendedBackend="vulkan"
        onImageBackendChange={vi.fn()}
        hfTokenConfigured
        onHfTokenSave={onHfTokenSave}
        onHfTokenClear={onHfTokenClear}
        models={[]}
        onDownload={vi.fn()}
        onRemove={vi.fn()}
        onOpenFolder={vi.fn()}
        onImport={vi.fn()}
        onBack={vi.fn()}
        environment={null}
        onReinstallEnvironment={vi.fn()}
        onRemoveEnvironment={vi.fn()}
      />,
    );

    expect(screen.getByText("Token salvo neste computador.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Modelos" }).parentElement).toHaveClass(
      "overflow-y-auto",
    );
    await user.type(screen.getByLabelText("Token do Hugging Face"), "hf_teste");
    await user.click(screen.getByRole("button", { name: "Salvar token" }));
    expect(onHfTokenSave).toHaveBeenCalledWith("hf_teste");
    expect(screen.getByLabelText("Token do Hugging Face")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "Apagar token" }));
    expect(onHfTokenClear).toHaveBeenCalled();
  });
});
