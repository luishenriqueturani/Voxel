import {
  type DownloadProgress,
  downloadHubModel,
  downloadModel,
  type HubFile,
  type ListedModel,
  listenToDownloadProgress,
  loadModels,
} from "../native/models";
import {
  applyDownloadProgress,
  type DownloadJob,
  type DownloadSnapshot,
  emptyDownloadSnapshot,
  enqueueJob,
  hubDownloadId,
  settleActive,
} from "./queueState";

type DownloadHandlers = {
  onLibrary: (models: ListedModel[]) => void;
  onError: (error: unknown) => void;
};

let snapshot: DownloadSnapshot = emptyDownloadSnapshot;
let running = false;
let listening = false;
const listeners = new Set<(state: DownloadSnapshot) => void>();
let handlers: DownloadHandlers = {
  onLibrary: () => undefined,
  onError: () => undefined,
};

export function setDownloadHandlers(next: DownloadHandlers): void {
  handlers = next;
}

export function getDownloadSnapshot(): DownloadSnapshot {
  return snapshot;
}

export function subscribeDownloadQueue(listener: (state: DownloadSnapshot) => void): () => void {
  listeners.add(listener);
  listener(snapshot);
  ensureListen();
  return () => {
    listeners.delete(listener);
  };
}

export function enqueueCatalogDownload(id: string, label: string): void {
  push({ id, label, kind: "catalog" });
}

export function enqueueHubDownload(file: HubFile): void {
  push({ id: hubDownloadId(file), label: file.filename, kind: "hub", file });
}

export function resetDownloadQueueForTests(): void {
  snapshot = emptyDownloadSnapshot;
  running = false;
  listeners.clear();
}

function push(job: DownloadJob): void {
  const next = enqueueJob(snapshot, job);
  if (next === snapshot) {
    return;
  }
  snapshot = next;
  emit();
  void pump();
}

async function pump(): Promise<void> {
  if (running) {
    return;
  }
  const job = snapshot.active;
  if (job === null) {
    return;
  }
  running = true;
  try {
    if (job.kind === "hub" && job.file) {
      await downloadHubModel(job.file);
    } else {
      await downloadModel(job.id, true);
    }
    handlers.onLibrary(await loadModels());
  } catch (error) {
    handlers.onError(error);
  } finally {
    running = false;
    snapshot = settleActive(snapshot);
    emit();
    if (snapshot.active !== null) {
      void pump();
    }
  }
}

function ensureListen(): void {
  if (listening) {
    return;
  }
  listening = true;
  void listenToDownloadProgress((progress: DownloadProgress) => {
    const next = applyDownloadProgress(snapshot, progress);
    if (next === snapshot) {
      return;
    }
    snapshot = next;
    emit();
  });
}

function emit(): void {
  for (const listener of listeners) {
    listener(snapshot);
  }
}
