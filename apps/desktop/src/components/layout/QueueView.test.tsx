import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { QueueJob } from "../../native/queue";
import { QueueView } from "./QueueView";

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
      count: 2,
      mesh: "none",
      images: 0,
      meshes: 0,
    },
  ],
};

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    loadUnfinishedQueueJob: vi.fn(async () => job),
    pauseQueue: vi.fn(),
    resumeQueue: vi.fn(),
    cancelQueue: vi.fn(),
    reorderQueueItems: vi.fn(),
  };
});

describe("tela da fila", () => {
  it("mostra o progresso e cada item", async () => {
    render(<QueueView onBack={() => undefined} />);
    expect(await screen.findByRole("heading", { name: "Fila de execução" })).toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();
    expect(screen.getByText(/Item 1: ferreiro/)).toBeInTheDocument();
    expect(screen.getByText(/Concluído/)).toBeInTheDocument();
    expect(screen.getByText(/Item 2: guarda/)).toBeInTheDocument();
  });
});
