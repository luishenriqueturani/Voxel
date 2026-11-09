import { type Dispatch, type SetStateAction, useCallback, useRef, useState } from "react";
import { applyEditResult, showVersion } from "../lib/imageVersions";
import { popSlides, pushSlides } from "../lib/slideHistory";
import { loadImageFile, openImageForEdit, removeImageBackground } from "../native/background";
import type { GalleryImage } from "../native/generation";
import { loadPromptImages, type PromptEntry, type PromptImage } from "../native/history";

type GenerationRef = { path: string; seed: number } & Partial<PromptEntry>;

function asPromptEntry(entry: GenerationRef): PromptEntry | null {
  if (
    typeof entry.prompt !== "string" ||
    entry.prompt === "" ||
    typeof entry.negativePrompt !== "string" ||
    typeof entry.modelId !== "string" ||
    typeof entry.steps !== "number" ||
    typeof entry.width !== "number" ||
    typeof entry.height !== "number" ||
    typeof entry.cfg !== "number"
  ) {
    return null;
  }
  return {
    prompt: entry.prompt,
    negativePrompt: entry.negativePrompt,
    modelId: entry.modelId,
    steps: entry.steps,
    width: entry.width,
    height: entry.height,
    cfg: entry.cfg,
    seed: entry.seed,
    pinned: entry.pinned ?? false,
  };
}

async function filesFor(entry: GenerationRef): Promise<PromptImage[]> {
  const listed = asPromptEntry(entry);
  if (listed === null) {
    return [{ path: entry.path, seed: entry.seed }];
  }
  const files = await loadPromptImages(listed);
  if (files.length === 0) {
    return [{ path: entry.path, seed: entry.seed }];
  }
  return files.some((file) => file.path === entry.path)
    ? files
    : [...files, { path: entry.path, seed: entry.seed }];
}

type UseImageEditOptions = {
  slides: GalleryImage[];
  setSlides: Dispatch<SetStateAction<GalleryImage[]>>;
  slideIndex: number;
  setSlideIndex: Dispatch<SetStateAction<number>>;
  setView: (view: "workspace") => void;
  busy: boolean;
  reportFailure: (error: unknown) => void;
  openTitle: string;
};

export function useImageEdit({
  slides,
  setSlides,
  slideIndex,
  setSlideIndex,
  setView,
  busy,
  reportFailure,
  openTitle,
}: UseImageEditOptions): {
  removing: boolean;
  openImage: () => void;
  removeBackground: () => void;
  openGeneration: (entry: GenerationRef) => void;
  openHistory: (entry: PromptEntry) => void;
  canUndoEdit: boolean;
  holdEdit: () => void;
  commitEdit: () => void;
  discardEdit: () => void;
  restoreEdit: () => void;
  undoEdit: () => void;
  selectVersion: (index: number) => void;
} {
  const [removing, setRemoving] = useState(false);
  const [editHistory, setEditHistory] = useState<GalleryImage[][]>([]);
  const removingRef = useRef(false);
  const historyTicket = useRef(0);
  const pendingEdit = useRef<GalleryImage[] | null>(null);

  const holdEdit = useCallback(() => {
    pendingEdit.current = slides.length === 0 ? null : slides.map((slide) => ({ ...slide }));
  }, [slides]);

  const commitEdit = useCallback(() => {
    const snapshot = pendingEdit.current;
    pendingEdit.current = null;
    if (snapshot === null) {
      return;
    }
    setEditHistory((current) => pushSlides(current, snapshot));
  }, []);

  const discardEdit = useCallback(() => {
    pendingEdit.current = null;
  }, []);

  const restoreEdit = useCallback(() => {
    const snapshot = pendingEdit.current;
    pendingEdit.current = null;
    if (snapshot === null) {
      return;
    }
    setSlides(snapshot);
    setSlideIndex(Math.max(0, snapshot.length - 1));
  }, [setSlideIndex, setSlides]);

  const selectVersion = useCallback(
    (index: number) => {
      setSlides((current) =>
        current.map((slide, position) =>
          position === slideIndex ? showVersion(slide, index) : slide,
        ),
      );
    },
    [setSlides, slideIndex],
  );

  const undoEdit = useCallback(() => {
    const restored = popSlides(editHistory);
    if (restored.slides === null) {
      return;
    }
    setEditHistory(restored.history);
    setSlides(restored.slides);
    setSlideIndex(Math.max(0, restored.slides.length - 1));
  }, [editHistory, setSlideIndex, setSlides]);

  const openImage = useCallback(() => {
    if (busy || removingRef.current) {
      return;
    }
    void openImageForEdit(openTitle)
      .then((image) => {
        if (image === null) {
          return;
        }
        setSlides([
          { src: image.src, path: image.path, seed: null, transparent: image.transparent },
        ]);
        setSlideIndex(0);
        setView("workspace");
      })
      .catch(reportFailure);
  }, [busy, openTitle, reportFailure, setSlideIndex, setSlides, setView]);

  const removeBackground = useCallback(() => {
    const current = slides[slideIndex];
    if (current === undefined || busy || removingRef.current) {
      return;
    }
    const index = slideIndex;
    pendingEdit.current = slides.map((slide) => ({ ...slide }));
    removingRef.current = true;
    setRemoving(true);
    void removeImageBackground(current.path)
      .then((image) => {
        if (image === null) {
          pendingEdit.current = null;
          return;
        }
        const snapshot = pendingEdit.current;
        pendingEdit.current = null;
        if (snapshot !== null) {
          setEditHistory((current) => pushSlides(current, snapshot));
        }
        setSlides(
          (existing) =>
            applyEditResult(existing, index, [
              {
                src: image.src,
                path: image.path,
                seed: existing[index]?.seed ?? null,
                transparent: true,
              },
            ]).slides,
        );
      })
      .catch((error: unknown) => {
        pendingEdit.current = null;
        reportFailure(error);
      })
      .finally(() => {
        removingRef.current = false;
        setRemoving(false);
      });
  }, [busy, reportFailure, setSlides, slideIndex, slides]);

  const showFiles = useCallback(
    async (files: PromptImage[], ticket: number, focusPath?: string) => {
      const next: GalleryImage[] = [];
      for (const file of files) {
        if (historyTicket.current !== ticket) {
          return;
        }
        const image = await loadImageFile(file.path).catch(() => null);
        if (image === null) {
          continue;
        }
        next.push({
          src: image.src,
          path: image.path,
          seed: file.seed,
          transparent: image.transparent,
        });
      }
      if (historyTicket.current !== ticket) {
        return;
      }
      const focused =
        focusPath === undefined ? -1 : next.findIndex((slide) => slide.path === focusPath);
      setSlides(next);
      setSlideIndex(focused >= 0 ? focused : Math.max(0, next.length - 1));
      setView("workspace");
    },
    [setSlideIndex, setSlides, setView],
  );

  const openGeneration = useCallback(
    (entry: GenerationRef) => {
      if (busy || removingRef.current) {
        return;
      }
      const ticket = historyTicket.current + 1;
      historyTicket.current = ticket;
      void filesFor(entry)
        .then((files) => showFiles(files, ticket, entry.path))
        .catch(reportFailure);
    },
    [busy, reportFailure, showFiles],
  );

  const openHistory = useCallback(
    (entry: PromptEntry) => {
      if (busy || removingRef.current) {
        return;
      }
      const ticket = historyTicket.current + 1;
      historyTicket.current = ticket;
      void loadPromptImages(entry)
        .then((files) => showFiles(files, ticket))
        .catch(reportFailure);
    },
    [busy, reportFailure, showFiles],
  );

  return {
    removing,
    openImage,
    removeBackground,
    openGeneration,
    openHistory,
    canUndoEdit: editHistory.length > 0,
    holdEdit,
    commitEdit,
    discardEdit,
    restoreEdit,
    undoEdit,
    selectVersion,
  };
}
