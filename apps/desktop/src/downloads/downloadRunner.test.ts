import { beforeEach, describe, expect, it, vi } from "vitest";
import { downloadModel, loadModels } from "../native/models";
import {
  enqueueCatalogDownload,
  getDownloadSnapshot,
  resetDownloadQueueForTests,
} from "./downloadRunner";

vi.mock("../native/models", () => ({
  downloadModel: vi.fn(async () => undefined),
  downloadHubModel: vi.fn(async () => undefined),
  loadModels: vi.fn(async () => []),
  listenToDownloadProgress: vi.fn(async () => () => undefined),
}));

describe("execução da fila", () => {
  beforeEach(() => {
    resetDownloadQueueForTests();
    vi.mocked(downloadModel).mockReset();
    vi.mocked(downloadModel).mockResolvedValue(undefined);
    vi.mocked(loadModels).mockClear();
  });

  it("baixa um modelo por vez e continua o próximo", async () => {
    const order: string[] = [];
    vi.mocked(downloadModel).mockImplementation(async (id: string) => {
      order.push(id);
    });
    enqueueCatalogDownload("sd15", "Stable Diffusion 1.5");
    enqueueCatalogDownload("qwen", "Qwen");
    enqueueCatalogDownload("sd15", "Stable Diffusion 1.5");
    await vi.waitUntil(() => getDownloadSnapshot().active === null);
    expect(order).toEqual(["sd15", "qwen"]);
    expect(downloadModel).toHaveBeenCalledTimes(2);
    expect(loadModels).toHaveBeenCalledTimes(2);
  });
});
