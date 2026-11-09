import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  enqueueFlow,
  formatElapsed,
  laterItems,
  loadUnfinishedQueueJob,
  movableItems,
  pauseQueue,
  queueCounts,
  queueSummary,
  toggleQueuePause,
} from "./queue";

const state = vi.hoisted(() => ({
  inside: false,
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => state.inside,
  invoke: (...args: unknown[]) => state.invoke(...args),
}));

const job = {
  id: "job-1",
  name: "NPCs",
  status: "pending",
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

describe("fila nativa", () => {
  beforeEach(() => {
    state.inside = false;
    state.invoke.mockReset();
  });

  it("devolve um job local fora do aplicativo", async () => {
    const created = await enqueueFlow("flow-1");
    expect(created.id).toBe("queue-flow-1");
    expect(created.status).toBe("pending");
    expect(state.invoke).not.toHaveBeenCalled();
  });

  it("enfileira pelo comando do aplicativo", async () => {
    state.inside = true;
    state.invoke.mockResolvedValue(job);
    await expect(enqueueFlow("flow-1")).resolves.toEqual(job);
    expect(state.invoke).toHaveBeenCalledWith("enqueue_flow", { id: "flow-1" });
  });

  it("lê o job aberto e pausa a fila", async () => {
    state.inside = true;
    state.invoke.mockResolvedValue({ ...job, status: "paused" });
    await expect(loadUnfinishedQueueJob()).resolves.toMatchObject({ status: "paused" });
    await expect(pauseQueue()).resolves.toMatchObject({ status: "paused" });
    expect(state.invoke).toHaveBeenCalledWith("unfinished_queue_job");
    expect(state.invoke).toHaveBeenCalledWith("pause_queue");
  });

  it("lista só os itens que ainda podem mudar de lugar", () => {
    expect(movableItems(job).map((item) => item.id)).toEqual(["b"]);
    expect(queueCounts(job)).toEqual({ done: 1, total: 2, paused: false });
    expect(queueCounts(null)).toEqual({ done: 0, total: 0, paused: false });
    expect(laterItems(job)).toEqual([]);
    expect(
      laterItems({
        ...job,
        items: [
          {
            id: "a",
            status: "done",
            prompt: "ferreiro",
            count: 1,
            mesh: "later",
            images: 1,
            meshes: 0,
            imagePaths: ["/tmp/a.png"],
          },
        ],
      }).map((item) => item.id),
    ).toEqual(["a"]);
    expect(
      queueSummary({
        ...job,
        createdAt: "2026-11-09T10:00:00Z",
        updatedAt: "2026-11-09T10:05:00Z",
      }).elapsedSecs,
    ).toBe(300);
    expect(formatElapsed(7800)).toEqual({ hours: 2, minutes: 10 });
  });

  it("pausa ou retoma conforme o estado atual", async () => {
    state.inside = true;
    state.invoke.mockResolvedValue({ ...job, status: "paused" });
    await expect(toggleQueuePause()).resolves.toMatchObject({ status: "paused" });
    expect(state.invoke).toHaveBeenCalledWith("resume_queue");
    state.invoke.mockResolvedValue(job);
    await expect(toggleQueuePause()).resolves.toMatchObject({ status: "pending" });
    expect(state.invoke).toHaveBeenCalledWith("pause_queue");
  });
});
