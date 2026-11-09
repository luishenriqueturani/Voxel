import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueueJob } from "../../native/queue";
import { QueueControls } from "./QueueControls";

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
    {
      id: "c",
      status: "pending",
      prompt: "aldeã",
      count: 1,
      mesh: "none",
      images: 0,
      meshes: 0,
    },
  ],
};

const native = vi.hoisted(() => ({
  current: null as QueueJob | null,
}));

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    loadUnfinishedQueueJob: vi.fn(async () => native.current),
    pauseQueue: vi.fn(async () => {
      if (native.current === null) {
        throw new Error("fila vazia");
      }
      native.current = { ...native.current, status: "paused" };
      return native.current;
    }),
    resumeQueue: vi.fn(async () => {
      if (native.current === null) {
        throw new Error("fila vazia");
      }
      native.current = { ...native.current, status: "running" };
      return native.current;
    }),
    cancelQueue: vi.fn(async () => {
      native.current = null;
    }),
    reorderQueueItems: vi.fn(async (_jobId: string, pendingIds: string[]) => {
      if (native.current === null) {
        throw new Error("fila vazia");
      }
      const locked = native.current.items.filter(
        (item) => !(item.status === "pending" && item.images === 0),
      );
      const pending = pendingIds.map((id) => native.current?.items.find((item) => item.id === id));
      native.current = {
        ...native.current,
        items: [...locked, ...pending.filter((item) => item !== undefined)],
      };
      return native.current;
    }),
  };
});

describe("controles da fila", () => {
  beforeEach(() => {
    native.current = {
      ...job,
      items: job.items.map((item) => ({ ...item })),
    };
  });

  it("pausa, reordena pendentes e cancela", async () => {
    const user = userEvent.setup();
    render(<QueueControls />);
    expect(await screen.findByText(/Fila em execução: NPCs/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Pausar" }));
    expect(await screen.findByText(/Fila pausada: NPCs/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Retomar" }));
    expect(await screen.findByText(/Fila em execução: NPCs/)).toBeInTheDocument();
    const firstPending = screen.getAllByRole("button", { name: "Descer na fila" })[0];
    if (firstPending === undefined) {
      throw new Error("pendente ausente");
    }
    await user.click(firstPending);
    expect(screen.getByText("aldeã")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancelar fila" }));
    expect(screen.queryByText(/NPCs/)).not.toBeInTheDocument();
  });
});
