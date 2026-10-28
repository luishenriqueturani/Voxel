import { describe, expect, it } from "vitest";
import {
  applyDownloadProgress,
  emptyDownloadSnapshot,
  enqueueJob,
  settleActive,
} from "./queueState";

const sd15 = { id: "sd15", label: "Stable Diffusion 1.5", kind: "catalog" as const };
const qwen = { id: "qwen", label: "Qwen", kind: "catalog" as const };

describe("fila de downloads", () => {
  it("começa o primeiro e guarda o seguinte", () => {
    const started = enqueueJob(emptyDownloadSnapshot, sd15);
    expect(started.active).toEqual(sd15);
    expect(started.waiting).toEqual([]);
    const queued = enqueueJob(started, qwen);
    expect(queued.active).toEqual(sd15);
    expect(queued.waiting).toEqual([qwen]);
    expect(enqueueJob(queued, sd15)).toBe(queued);
  });

  it("avança para o próximo quando o atual termina", () => {
    const queued = enqueueJob(enqueueJob(emptyDownloadSnapshot, sd15), qwen);
    const next = settleActive(queued);
    expect(next.active).toEqual(qwen);
    expect(next.waiting).toEqual([]);
    expect(settleActive(next)).toEqual(emptyDownloadSnapshot);
  });

  it("só atualiza o progresso do download atual", () => {
    const started = enqueueJob(emptyDownloadSnapshot, sd15);
    const moved = applyDownloadProgress(started, {
      modelId: "sd15",
      received: 4,
      total: 10,
      phase: "download",
    });
    expect(moved.progress).toEqual({
      modelId: "sd15",
      received: 4,
      total: 10,
      phase: "download",
    });
    expect(applyDownloadProgress(moved, { modelId: "qwen", received: 1, total: 2 })).toBe(moved);
  });
});
