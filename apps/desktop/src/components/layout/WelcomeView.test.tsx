import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { SetupStep } from "../../native/setup";
import { WelcomeView } from "./WelcomeView";

const steps: SetupStep[] = [
  { id: "hardware", state: "done", detail: "AMD Radeon RX 7600 · 32 GiB" },
  { id: "vulkan", state: "passed", detail: "" },
  { id: "rocm", state: "not_needed", detail: "" },
  { id: "cpu", state: "not_needed", detail: "" },
  { id: "profile", state: "done", detail: "intermediario" },
];

describe("boas-vindas do setup", () => {
  it("mostra o progresso e só continua quando o perfil está escolhido", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    const { rerender } = render(<WelcomeView steps={steps.slice(0, 2)} onContinue={onContinue} />);

    expect(screen.getByRole("heading", { name: "Bem-vindo ao Voxel" })).toBeInTheDocument();
    expect(screen.getByText("AMD Radeon RX 7600 · 32 GiB")).toBeInTheDocument();
    expect(screen.getByText("Vulkan")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();

    rerender(<WelcomeView steps={steps} onContinue={onContinue} />);
    expect(screen.getByText("Perfil Intermediário")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    expect(onContinue).toHaveBeenCalledOnce();
  });
});
