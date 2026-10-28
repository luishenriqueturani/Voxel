import type { DownloadProgress, HubFile } from "../native/models";

export type DownloadJob = {
  id: string;
  label: string;
  kind: "catalog" | "hub";
  file?: HubFile;
};

export type DownloadSnapshot = {
  active: DownloadJob | null;
  waiting: DownloadJob[];
  progress: DownloadProgress | null;
};

export const emptyDownloadSnapshot: DownloadSnapshot = {
  active: null,
  waiting: [],
  progress: null,
};

export function hubDownloadId(file: HubFile): string {
  return `${file.repo}/${file.filename}`;
}

export function enqueueJob(state: DownloadSnapshot, job: DownloadJob): DownloadSnapshot {
  if (listed(state, job.id)) {
    return state;
  }
  if (state.active === null) {
    return {
      active: job,
      waiting: state.waiting,
      progress: { modelId: job.id, received: 0, total: 0 },
    };
  }
  return { ...state, waiting: [...state.waiting, job] };
}

export function settleActive(state: DownloadSnapshot): DownloadSnapshot {
  const [next, ...waiting] = state.waiting;
  if (next === undefined) {
    return emptyDownloadSnapshot;
  }
  return {
    active: next,
    waiting,
    progress: { modelId: next.id, received: 0, total: 0 },
  };
}

export function applyDownloadProgress(
  state: DownloadSnapshot,
  progress: DownloadProgress,
): DownloadSnapshot {
  if (state.active?.id !== progress.modelId) {
    return state;
  }
  return { ...state, progress };
}

function listed(state: DownloadSnapshot, id: string): boolean {
  return state.active?.id === id || state.waiting.some((job) => job.id === id);
}
