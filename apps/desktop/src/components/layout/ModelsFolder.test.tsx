import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ModelsFolder } from "./ModelsFolder";

vi.mock("../../native/models", () => ({
  modelsDirectory: vi.fn(async () => "/home/luis/.local/share/voxel/models"),
  pickModelsDirectory: vi.fn(async () => "/mnt/dados/voxel-models"),
  setModelsDirectory: vi.fn(async (path: string) => path),
}));

import { pickModelsDirectory, setModelsDirectory } from "../../native/models";

describe("ModelsFolder", () => {
  it("mostra o caminho e pergunta se move ao trocar a pasta", async () => {
    render(<ModelsFolder onChanged={vi.fn()} onError={vi.fn()} />);
    expect(await screen.findByText("/home/luis/.local/share/voxel/models")).toBeInTheDocument();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Escolher pasta…" }));
    expect(await screen.findByText(/Mover os modelos já baixados/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Mover arquivos" }));
    await waitFor(() => {
      expect(setModelsDirectory).toHaveBeenCalledWith("/mnt/dados/voxel-models", true);
    });
    expect(pickModelsDirectory).toHaveBeenCalled();
  });
});
