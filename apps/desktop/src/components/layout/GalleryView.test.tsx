import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { GalleryItem, GalleryQuery } from "../../native/history";
import { loadGallery, loadGalleryFacets } from "../../native/history";
import { GalleryView } from "./GalleryView";

vi.mock("../../native/history", () => ({
  EMPTY_GALLERY_QUERY: {
    prompt: "",
    modelId: "",
    flowId: "",
    kind: "",
    from: "",
    to: "",
  },
  loadGallery: vi.fn(async () => []),
  loadGalleryFacets: vi.fn(async () => ({ models: ["sd15"], flows: [] })),
}));

function item(prompt: string, path: string): GalleryItem {
  return {
    path,
    prompt,
    negativePrompt: "",
    modelId: "sd15",
    steps: 25,
    width: 512,
    height: 512,
    cfg: 7,
    seed: 4,
    pinned: false,
    createdAt: "2026-10-20",
    preview: "data:image/jpeg;base64,abc",
    kind: "image",
    flowId: "",
    flowName: "",
  };
}

const gato = item("um gato", "/tmp/gato.png");
const casa = item("uma casa", "/tmp/casa.png");

describe("GalleryView", () => {
  it("mostra o vazio e abre a geração escolhida", async () => {
    vi.mocked(loadGallery).mockResolvedValueOnce([]);
    const onOpen = vi.fn();
    const { rerender } = render(<GalleryView key={0} onOpen={onOpen} onError={vi.fn()} />);
    expect(await screen.findByText("Nenhuma geração ainda")).toBeInTheDocument();

    vi.mocked(loadGallery).mockResolvedValue([gato]);
    rerender(<GalleryView key={1} onOpen={onOpen} onError={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "um gato" }));
    expect(onOpen).toHaveBeenCalledWith(gato);
  });

  it("pesquisa no prompt e compara duas variações", async () => {
    vi.mocked(loadGalleryFacets).mockResolvedValue({
      models: ["sd15"],
      flows: [{ id: "flow-1", name: "NPCs" }],
    });
    vi.mocked(loadGallery).mockImplementation(async (filter?: GalleryQuery) => {
      if (filter?.prompt === "gato") {
        return [gato];
      }
      if (filter?.prompt) {
        return [];
      }
      return [gato, casa];
    });
    render(<GalleryView onOpen={vi.fn()} onError={vi.fn()} />);
    const user = userEvent.setup();
    expect(await screen.findByRole("button", { name: "um gato" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "uma casa" })).toBeInTheDocument();

    await user.type(screen.getByLabelText("Pesquisa"), "gato");
    await waitFor(() => {
      expect(loadGallery).toHaveBeenLastCalledWith(expect.objectContaining({ prompt: "gato" }));
    });
    expect(await screen.findByRole("button", { name: "um gato" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "uma casa" })).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText("Pesquisa"));
    expect(await screen.findByRole("button", { name: "uma casa" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Comparar" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: "Selecionar um gato para comparar" }));
    await user.click(screen.getByRole("checkbox", { name: "Selecionar uma casa para comparar" }));
    await user.click(screen.getByRole("button", { name: "Comparar" }));
    expect(screen.getByText("Variação A")).toBeInTheDocument();
    expect(screen.getByText("Variação B")).toBeInTheDocument();
    expect(screen.getAllByText("um gato").length).toBeGreaterThan(0);
    expect(screen.getAllByText("uma casa").length).toBeGreaterThan(0);
  });
});
