import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { loadRecentPrompts, type PromptEntry, setPromptFavorite } from "../../native/history";
import { RecentPrompts } from "./RecentPrompts";

vi.mock("../../native/history", () => ({
  loadRecentPrompts: vi.fn(async () => []),
  setPromptFavorite: vi.fn(async () => undefined),
}));

const gato: PromptEntry = {
  prompt: "um gato",
  negativePrompt: "borrado",
  modelId: "sd15",
  steps: 20,
  width: 1024,
  height: 576,
  cfg: 7,
  seed: 9,
  pinned: false,
};

const casa: PromptEntry = {
  ...gato,
  prompt: "uma casa",
  negativePrompt: "",
  seed: 3,
  pinned: true,
};

function renderList() {
  const onPromptChange = vi.fn();
  const onParamsChange = vi.fn();
  const onModelChange = vi.fn();
  const onNegativePromptChange = vi.fn();
  const onRestore = vi.fn();
  render(
    <RecentPrompts
      count={2}
      onPromptChange={onPromptChange}
      onNegativePromptChange={onNegativePromptChange}
      onModelChange={onModelChange}
      onParamsChange={onParamsChange}
      onError={vi.fn()}
      onRestore={onRestore}
    />,
  );
  return { onPromptChange, onParamsChange, onModelChange, onNegativePromptChange, onRestore };
}

describe("RecentPrompts", () => {
  it("recarrega o prompt e os parâmetros ao clicar", async () => {
    vi.mocked(loadRecentPrompts).mockResolvedValue([gato]);
    const applied = renderList();
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "um gato" }));
    expect(applied.onPromptChange).toHaveBeenCalledWith("um gato");
    expect(applied.onNegativePromptChange).toHaveBeenCalledWith("borrado");
    expect(applied.onModelChange).toHaveBeenCalledWith("sd15");
    expect(applied.onParamsChange).toHaveBeenCalledWith(
      expect.objectContaining({
        aspect: "16:9",
        width: 1024,
        height: 576,
        steps: 20,
        seed: "9",
        count: 2,
      }),
    );
    expect(applied.onRestore).toHaveBeenCalledWith(gato);
  });

  it("filtra pela busca e fixa o prompt", async () => {
    vi.mocked(loadRecentPrompts).mockResolvedValue([gato, casa]);
    renderList();
    const user = userEvent.setup();
    expect(await screen.findByRole("button", { name: "um gato" })).toBeInTheDocument();
    await user.type(screen.getByRole("searchbox", { name: "Buscar prompts" }), "casa");
    expect(screen.queryByRole("button", { name: "um gato" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Desafixar" }));
    expect(setPromptFavorite).toHaveBeenCalledWith(casa, false);
  });
});
