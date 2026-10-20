import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { GalleryItem } from "../../native/history";
import { loadGallery } from "../../native/history";
import { GalleryView } from "./GalleryView";

vi.mock("../../native/history", () => ({
  loadGallery: vi.fn(async () => []),
}));

const item: GalleryItem = {
  path: "/tmp/gato.png",
  prompt: "um gato",
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
};

describe("GalleryView", () => {
  it("mostra o vazio e abre a geração escolhida", async () => {
    vi.mocked(loadGallery).mockResolvedValueOnce([]);
    const onOpen = vi.fn();
    const { rerender } = render(<GalleryView key={0} onOpen={onOpen} onError={vi.fn()} />);
    expect(await screen.findByText("Nenhuma geração ainda")).toBeInTheDocument();

    vi.mocked(loadGallery).mockResolvedValue([item]);
    rerender(<GalleryView key={1} onOpen={onOpen} onError={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "um gato" }));
    expect(onOpen).toHaveBeenCalledWith(item);
  });
});
