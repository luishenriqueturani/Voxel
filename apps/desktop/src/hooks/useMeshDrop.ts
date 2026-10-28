import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { imagePaths } from "../lib/mesh";

type DragDrop = {
  paths?: string[];
};

export function useMeshDrop(onImages: (paths: string[]) => void): void {
  useEffect(() => {
    if (!isTauri()) {
      return undefined;
    }
    let stop: () => void = () => undefined;
    let cancelled = false;
    void listen<DragDrop>("tauri://drag-drop", (event) => {
      const images = imagePaths(event.payload.paths ?? []);
      if (images.length > 0) {
        onImages(images);
      }
    })
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
          return;
        }
        stop = unlisten;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      stop();
    };
  }, [onImages]);
}
