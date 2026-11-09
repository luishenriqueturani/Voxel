import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DownloadContext } from "../../downloads/DownloadProvider";
import type { QueueJob } from "../../native/queue";
import { StatusBar } from "./StatusBar";

const queue = vi.hoisted(() => ({
  current: null as QueueJob | null,
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    loadUnfinishedQueueJob: vi.fn(async () => queue.current),
  };
});

const job: QueueJob = {
  id: "job-1",
  name: "NPCs",
  status: "running",
  endRetry: false,
  items: [
    {
      id: "a",
      status: "done",
      prompt: "ferreiro",
      count: 1,
      mesh: "none",
      images: 1,
      meshes: 0,
    },
    {
      id: "b",
      status: "pending",
      prompt: "guarda",
      count: 1,
      mesh: "none",
      images: 0,
      meshes: 0,
    },
  ],
};

describe("StatusBar no app Tauri", () => {
  it("não mostra o atalho de preferências, que fica no menu nativo", () => {
    queue.current = null;
    render(
      <StatusBar status={null} onOpenSettings={() => undefined} onOpenQueue={() => undefined} />,
    );

    expect(screen.queryByRole("button", { name: "Preferências" })).not.toBeInTheDocument();
  });

  it("mostra GPU, backend, memórias e temperaturas", () => {
    queue.current = null;
    render(
      <StatusBar
        status={{
          gpuName: "RX 7600",
          backend: "vulkan",
          vramUsedBytes: 1_073_741_824,
          vramTotalBytes: 8_589_934_592,
          gpuPercent: 37,
          cpuPercent: 12,
          ramUsedBytes: 8_589_934_592,
          ramTotalBytes: 34_359_738_368,
          cpuCelsius: 54,
          gpuCelsius: 48,
        }}
        onOpenSettings={() => undefined}
        onOpenQueue={() => undefined}
      />,
    );

    const bar = screen.getByRole("contentinfo");
    expect(bar).toHaveTextContent("GPU: RX 7600");
    expect(bar).toHaveTextContent("Imagem: Vulkan");
    expect(bar).toHaveTextContent("VRAM: 1.0/8.0 GiB");
    expect(bar).toHaveTextContent("Uso GPU: 37%");
    expect(bar).toHaveTextContent("CPU: 12%");
    expect(bar).toHaveTextContent("RAM: 8.0/32.0 GiB");
    expect(bar).toHaveTextContent("Temp. CPU: 54 °C");
    expect(bar).toHaveTextContent("Temp. GPU: 48 °C");
  });

  it("mostra o progresso da fila e abre a tela", async () => {
    queue.current = job;
    const onOpenQueue = vi.fn();
    render(<StatusBar status={null} onOpenSettings={() => undefined} onOpenQueue={onOpenQueue} />);
    const button = await screen.findByRole("button", { name: "Fila: 1/2" });
    button.click();
    expect(onOpenQueue).toHaveBeenCalled();
  });

  it("mostra o download ativo à direita, com o restante da fila", () => {
    queue.current = null;
    render(
      <DownloadContext.Provider
        value={{
          active: { id: "sd15", label: "Stable Diffusion 1.5", kind: "catalog" },
          waiting: [{ id: "qwen", label: "Qwen", kind: "catalog" }],
          progress: { modelId: "sd15", received: 40, total: 100, phase: "download" },
          enqueueCatalog: () => undefined,
          enqueueHub: () => undefined,
        }}
      >
        <StatusBar status={null} onOpenSettings={() => undefined} onOpenQueue={() => undefined} />
      </DownloadContext.Provider>,
    );
    const label = screen.getByText("Baixando Stable Diffusion 1.5 40% · 1 na fila");
    expect(label.parentElement).toHaveClass("ml-auto");
  });
});
