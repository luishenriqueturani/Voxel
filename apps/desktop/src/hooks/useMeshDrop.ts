import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { firstImagePath } from "../lib/mesh";

type DragDrop = {
  paths?: string[];
};

export function useMeshDrop(onImage: (path: string) => void): void {
  useEffect(() => {
    if (!isTauri()) {
      return undefined;
    }
    let stop: () => void = () => undefined;
    let cancelled = false;
    void listen<DragDrop>("tauri://drag-drop", (event) => {
      const image = firstImagePath(event.payload.paths ?? []);
      if (image) {
        onImage(image);
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
  }, [onImage]);
}
