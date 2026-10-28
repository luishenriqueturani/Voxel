import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  hasOlderPrompts,
  loadOlderPrompts,
  loadPromptImages,
  loadRecentPrompts,
  type PromptEntry,
  setPromptFavorite,
} from "../../native/history";
import { RecentPrompts } from "./RecentPrompts";

vi.mock("../../native/history", () => ({
  loadRecentPrompts: vi.fn(async () => []),
  loadOlderPrompts: vi.fn(async () => []),
  hasOlderPrompts: vi.fn(async () => false),
  loadPromptImages: vi.fn(async () => []),
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
  beforeEach(() => {
    vi.mocked(hasOlderPrompts).mockResolvedValue(false);
    vi.mocked(loadOlderPrompts).mockResolvedValue([]);
    vi.mocked(loadPromptImages).mockResolvedValue([]);
  });

  it("abre o recente e só aplica ao repetir", async () => {
    vi.mocked(loadRecentPrompts).mockResolvedValue([gato]);
    vi.mocked(loadPromptImages).mockResolvedValue([{ path: "/tmp/gato.png", seed: 9 }]);
    const applied = renderList();
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "um gato" }));
    const dialog = await screen.findByRole("dialog", { name: "um gato" });
    expect(applied.onPromptChange).not.toHaveBeenCalled();
    expect(within(dialog).getByText("Prompt negativo: borrado")).toBeInTheDocument();
    expect(within(dialog).getByText("Modelo: sd15")).toBeInTheDocument();
    expect(within(dialog).getByText("Passos: 20")).toBeInTheDocument();
    expect(within(dialog).getByText("1024×576")).toBeInTheDocument();
    expect(within(dialog).getByText("CFG: 7")).toBeInTheDocument();
    expect(within(dialog).getByText("Seed: 9")).toBeInTheDocument();
    expect(await within(dialog).findByText("gato.png")).toBeInTheDocument();

    await user.click(within(dialog).getByRole("button", { name: "Fechar" }));
    expect(applied.onPromptChange).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "um gato" }));
    const again = await screen.findByRole("dialog", { name: "um gato" });
    await user.click(within(again).getByRole("button", { name: "Repetir" }));
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

  it("traz o restante no fim da lista", async () => {
    vi.mocked(loadRecentPrompts).mockResolvedValue([gato]);
    vi.mocked(hasOlderPrompts).mockResolvedValue(true);
    vi.mocked(loadOlderPrompts).mockResolvedValue([{ ...gato, prompt: "um cão", seed: 4 }]);
    renderList();
    const user = userEvent.setup();
    const more = await screen.findByRole("button", { name: "Ver o restante" });
    expect(more.closest("ul")).not.toBeNull();
    expect(more.closest(".overflow-y-auto")).not.toBeNull();
    await user.click(more);
    expect(await screen.findByRole("button", { name: "um cão" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Ver o restante" })).not.toBeInTheDocument();
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
