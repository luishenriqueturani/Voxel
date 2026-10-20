import { useCallback, useEffect, useRef, useState } from "react";
import {
  type DownloadProgress,
  downloadModel,
  type ListedModel,
  listenToDownloadProgress,
  loadModels,
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
      activeRef.current = id;
      setModelProgress({ modelId: id, received: 0, total: 0 });
      void downloadModel(id, true)
        .then(() => loadModels())
        .then((models) => {
          setLibrary(models);
        })
        .catch((error: unknown) => {
          onErrorRef.current(error);
        })
        .finally(() => {
          if (activeRef.current === id) {
            activeRef.current = null;
            setModelProgress(null);
          }
        });
    },
    [setLibrary],
  );

  return { modelProgress, downloadCatalogModel };
}
