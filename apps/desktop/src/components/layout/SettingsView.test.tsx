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
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByText("Recomendado pelo setup: Vulkan")).toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "Backend de imagem" }), "cpu");
    expect(onImageBackendChange).toHaveBeenCalledWith("cpu");
  });
});
