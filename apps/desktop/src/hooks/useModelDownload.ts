import { useCallback, useEffect, useRef, useState } from "react";
import {
  claimDownload,
  type DownloadProgress,
  downloadModel,
  type ListedModel,
  listenToDownloadProgress,
  loadModels,
  releaseDownload,
  subscribeActiveDownload,
} from "../native/models";

export function useModelDownload(
  setLibrary: (models: ListedModel[]) => void,
  onError: (error: unknown) => void,
): {
  modelProgress: DownloadProgress | null;
  downloadCatalogModel: (id: string) => void;
} {
  const [modelProgress, setModelProgress] = useState<DownloadProgress | null>(null);
  const activeRef = useRef<string | null>(null);
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  useEffect(() => {
    const reload = () => {
      void loadModels()
        .then((models) => {
          setLibrary(models);
        })
        .catch((error: unknown) => {
          onErrorRef.current(error);
        });
    };
    window.addEventListener("voxel-models", reload);
    return () => window.removeEventListener("voxel-models", reload);
  }, [setLibrary]);

  useEffect(() => {
    return subscribeActiveDownload((id) => {
      activeRef.current = id;
      setModelProgress(id ? { modelId: id, received: 0, total: 0 } : null);
    });
  }, []);

  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    void listenToDownloadProgress((progress) => {
      if (activeRef.current === progress.modelId) {
        setModelProgress(progress);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  const downloadCatalogModel = useCallback(
    (id: string) => {
      if (!claimDownload(id)) {
        return;
      }
      void downloadModel(id, true)
        .then(() => loadModels())
        .then((models) => {
          setLibrary(models);
        })
        .catch((error: unknown) => {
          onErrorRef.current(error);
        })
        .finally(() => {
          releaseDownload(id);
        });
    },
    [setLibrary],
  );

  return { modelProgress, downloadCatalogModel };
}
