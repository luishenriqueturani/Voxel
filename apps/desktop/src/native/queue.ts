import { invoke, isTauri } from "@tauri-apps/api/core";

export type QueueItem = {
  id: string;
  status: string;
  prompt: string;
  count: number;
  mesh: string;
  images: number;
  meshes: number;
  errorLog?: string | null;
  errorSummary?: string | null;
  imagePaths?: string[];
};

export type QueueJob = {
  id: string;
  name: string;
  status: string;
  endRetry: boolean;
  createdAt?: string;
  updatedAt?: string;
  items: QueueItem[];
};

export type MeshPick = {
  itemId: string;
  paths: string[];
};

export type QueueSummary = {
  done: number;
  failed: number;
  cancelled: number;
  later: number;
  elapsedSecs: number;
};

function isItem(value: unknown): value is QueueItem {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const item = value as QueueItem;
  return (
    typeof item.id === "string" &&
    typeof item.status === "string" &&
    typeof item.prompt === "string" &&
    typeof item.count === "number" &&
    typeof item.mesh === "string" &&
    typeof item.images === "number" &&
    typeof item.meshes === "number"
  );
}

function isJob(value: unknown): value is QueueJob {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const job = value as QueueJob;
  return (
    typeof job.id === "string" &&
    typeof job.name === "string" &&
    typeof job.status === "string" &&
    typeof job.endRetry === "boolean" &&
    Array.isArray(job.items) &&
    job.items.every(isItem)
  );
}

async function readJob(command: string, args?: Record<string, unknown>): Promise<QueueJob> {
  const value = args === undefined ? await invoke(command) : await invoke(command, args);
  if (!isJob(value)) {
    throw new Error("fila inválida");
  }
  return value;
}

export function movableItems(job: QueueJob): QueueItem[] {
  return job.items.filter(
    (item) => item.status === "pending" && item.images === 0 && item.meshes === 0,
  );
}

export function queueCounts(job: QueueJob | null): {
  done: number;
  total: number;
  paused: boolean;
} {
  if (job === null) {
    return { done: 0, total: 0, paused: false };
  }
  const done = job.items.filter(
    (item) => item.status === "done" || item.status === "failed" || item.status === "cancelled",
  ).length;
  return { done, total: job.items.length, paused: job.status === "paused" };
}

export function laterItems(job: QueueJob): QueueItem[] {
  return job.items.filter(
    (item) => item.mesh === "later" && item.status === "done" && (item.imagePaths?.length ?? 0) > 0,
  );
}

export function queueSummary(job: QueueJob): QueueSummary {
  const start = Date.parse(job.createdAt ?? "");
  const end = Date.parse(job.updatedAt ?? "");
  const elapsedSecs =
    Number.isFinite(start) && Number.isFinite(end)
      ? Math.max(0, Math.round((end - start) / 1000))
      : 0;
  return {
    done: job.items.filter((item) => item.status === "done").length,
    failed: job.items.filter((item) => item.status === "failed").length,
    cancelled: job.items.filter((item) => item.status === "cancelled").length,
    later: laterItems(job).length,
    elapsedSecs,
  };
}

export function formatElapsed(secs: number): { hours: number; minutes: number } {
  return { hours: Math.floor(secs / 3600), minutes: Math.floor((secs % 3600) / 60) };
}

export async function enqueueFlow(id: string): Promise<QueueJob> {
  if (!isTauri()) {
    return {
      id: `queue-${id}`,
      name: id,
      status: "pending",
      endRetry: false,
      items: [],
    };
  }
  return readJob("enqueue_flow", { id });
}

export async function loadUnfinishedQueueJob(): Promise<QueueJob | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("unfinished_queue_job");
  if (value === null) {
    return null;
  }
  if (!isJob(value)) {
    throw new Error("fila inválida");
  }
  return value;
}

export async function pauseQueue(): Promise<QueueJob> {
  return readJob("pause_queue");
}

export async function resumeQueue(): Promise<QueueJob> {
  return readJob("resume_queue");
}

export async function cancelQueue(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("cancel_queue");
}

export async function reorderQueueItems(jobId: string, pendingIds: string[]): Promise<QueueJob> {
  return readJob("reorder_queue_items", { jobId, pendingIds });
}

export async function toggleQueuePause(): Promise<QueueJob | null> {
  const job = await loadUnfinishedQueueJob();
  if (job === null) {
    return null;
  }
  if (job.status === "paused") {
    return resumeQueue();
  }
  return pauseQueue();
}

export async function loadLatestQueueJob(): Promise<QueueJob | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("latest_queue_job");
  if (value === null) {
    return null;
  }
  if (!isJob(value)) {
    throw new Error("fila inválida");
  }
  return value;
}

export async function enqueueMeshPicks(jobId: string, picks: MeshPick[]): Promise<QueueJob> {
  return readJob("enqueue_mesh_picks", { jobId, picks });
}

export async function loadQueueThumbnails(paths: string[]): Promise<Record<string, string>> {
  if (!isTauri() || paths.length === 0) {
    return {};
  }
  const rows = await invoke<Array<{ path: string; preview: string }>>("queue_thumbnails", {
    paths,
  });
  const thumbs: Record<string, string> = {};
  for (const row of rows) {
    if (row.preview !== "") {
      thumbs[row.path] = `data:image/jpeg;base64,${row.preview}`;
    }
  }
  return thumbs;
}
