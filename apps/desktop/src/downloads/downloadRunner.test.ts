import { beforeEach, describe, expect, it, vi } from "vitest";
import type { HubPackage } from "../native/models";
import { downloadHubModel, downloadModel, loadModels } from "../native/models";
import {
  enqueueCatalogDownload,
  enqueueHubDownload,
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
    vi.mocked(downloadHubModel).mockReset();
    vi.mocked(downloadHubModel).mockResolvedValue(undefined);
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

  it("baixa cada parte do pacote em sequência", async () => {
    const seen: string[] = [];
    vi.mocked(downloadHubModel).mockImplementation(async (part) => {
      seen.push(part.filename);
    });
    const pkg: HubPackage = {
      repo: "lab/gato",
      name: "gato · merged-model",
      folder: "merged-model",
      license: "mit",
      gated: false,
      opener: "merged-model",
      directory: true,
      marker: "voxel-package.json",
      parts: [
        {
          repo: "lab/gato",
          filename: "a.safetensors",
          bytes: 1,
          sha256: "",
          role: "vae",
          shardIndex: null,
          shardCount: null,
        },
        {
          repo: "lab/gato",
          filename: "b.safetensors",
          bytes: 2,
          sha256: "ab",
          role: "unet",
          shardIndex: null,
          shardCount: null,
        },
      ],
    };
    enqueueHubDownload(pkg);
    await vi.waitUntil(() => getDownloadSnapshot().active === null);
    expect(seen).toEqual(["a.safetensors", "b.safetensors"]);
    expect(downloadHubModel).toHaveBeenCalledTimes(2);
    expect(vi.mocked(downloadHubModel).mock.calls[0]?.[1]).toBe(pkg);
    expect(vi.mocked(downloadHubModel).mock.calls[1]?.[1]).toBe(pkg);
  });
});
