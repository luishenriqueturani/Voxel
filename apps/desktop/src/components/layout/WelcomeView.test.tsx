import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ProfileCatalog, SetupStep } from "../../native/setup";
import { WelcomeView } from "./WelcomeView";

const steps: SetupStep[] = [
  { id: "hardware", state: "done", detail: "AMD Radeon RX 7600 · 32 GiB" },
  { id: "vulkan", state: "passed", detail: "" },
  { id: "rocm", state: "not_needed", detail: "" },
  { id: "cpu", state: "not_needed", detail: "" },
  { id: "profile", state: "done", detail: "intermediario" },
];

const catalog: ProfileCatalog[] = [
  {
    profile: "minimo",
    models: [
      { id: "sd15", bytes: 4_265_146_304, license: "CreativeML OpenRAIL-M", optional: false },
      { id: "triposr", bytes: 1_677_246_742, license: "MIT", optional: false },
    ],
  },
  {
    profile: "intermediario",
    models: [
      { id: "sd15", bytes: 4_265_146_304, license: "CreativeML OpenRAIL-M", optional: false },
      { id: "triposr", bytes: 1_677_246_742, license: "MIT", optional: false },
      {
        id: "sf3d",
        bytes: 4_024_289_892,
        license: "Stability AI Community License",
        optional: true,
      },
    ],
  },
];

const props = {
  steps,
  catalog,
  backend: "vulkan",
  confirming: false,
  image: null,
  outcome: null,
  detail: "",
};

describe("boas-vindas do setup", () => {
  it("sugere os modelos do perfil e só entra depois da imagem de teste", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onContinue = vi.fn();
    const { rerender } = render(
      <WelcomeView
        {...props}
        steps={steps.slice(0, 2)}
        onConfirm={onConfirm}
        onContinue={onContinue}
      />,
    );

    expect(screen.getByRole("heading", { name: "Bem-vindo ao Voxel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
    expect(
      screen.queryByRole("checkbox", { name: /Stable Diffusion 1.5/ }),
    ).not.toBeInTheDocument();

    rerender(<WelcomeView {...props} onConfirm={onConfirm} onContinue={onContinue} />);
    expect(screen.getByText(/CreativeML OpenRAIL-M/)).toBeInTheDocument();
    expect(screen.getByText(/Stability AI Community License/)).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /Stable Fast 3D/ })).not.toBeChecked();

    await user.click(screen.getByRole("button", { name: "Modo avançado" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Perfil" }), "minimo");
    expect(screen.queryByText(/Stability AI Community License/)).not.toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "Backend" }), "cpu");
    await user.click(screen.getByRole("button", { name: "Gerar imagem de teste" }));
    expect(onConfirm).toHaveBeenCalledWith({
      models: ["sd15", "triposr"],
      profile: "minimo",
      backend: "cpu",
    });
    expect(onContinue).not.toHaveBeenCalled();

    rerender(
      <WelcomeView
        {...props}
        image="iVBORw0KGgo"
        outcome="passed"
        onConfirm={onConfirm}
        onContinue={onContinue}
      />,
    );
    expect(screen.getByRole("img", { name: "Imagem de teste" })).toHaveAttribute(
      "src",
      "data:image/png;base64,iVBORw0KGgo",
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    expect(onContinue).toHaveBeenCalledOnce();
  });
});
