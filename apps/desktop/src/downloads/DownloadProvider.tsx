import { createContext, type ReactNode, useContext, useEffect, useMemo, useState } from "react";
import type { HubFile, ListedModel } from "../native/models";
import {
  enqueueCatalogDownload,
  enqueueHubDownload,
  getDownloadSnapshot,
  setDownloadHandlers,
  subscribeDownloadQueue,
} from "./downloadRunner";
import type { DownloadJob, DownloadSnapshot } from "./queueState";

export type DownloadApi = DownloadSnapshot & {
  enqueueCatalog: (id: string, label: string) => void;
  enqueueHub: (file: HubFile) => void;
};

const idle: DownloadApi = {
  active: null,
  waiting: [],
  progress: null,
  enqueueCatalog: () => undefined,
  enqueueHub: () => undefined,
};

export const DownloadContext = createContext<DownloadApi>(idle);

type DownloadProviderProps = {
  children: ReactNode;
  onLibrary: (models: ListedModel[]) => void;
  onError: (error: unknown) => void;
};

export function DownloadProvider({ children, onLibrary, onError }: DownloadProviderProps) {
  const [snapshot, setSnapshot] = useState<DownloadSnapshot>(getDownloadSnapshot);
  setDownloadHandlers({ onLibrary, onError });

  useEffect(() => subscribeDownloadQueue(setSnapshot), []);

  const value = useMemo<DownloadApi>(
    () => ({
      ...snapshot,
      enqueueCatalog: enqueueCatalogDownload,
      enqueueHub: enqueueHubDownload,
    }),
    [snapshot],
  );

  return <DownloadContext.Provider value={value}>{children}</DownloadContext.Provider>;
}

export function useDownloadQueue(): DownloadApi {
  return useContext(DownloadContext);
}

export function queuedDownloadIds(waiting: readonly DownloadJob[]): string[] {
  return waiting.map((job) => job.id);
}
