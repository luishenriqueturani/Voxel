import { type Dispatch, type SetStateAction, useCallback, useRef, useState } from "react";
import { loadImageFile, openImageForEdit, removeImageBackground } from "../native/background";
import type { GalleryImage } from "../native/generation";
import { loadPromptImages, type PromptEntry } from "../native/history";

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
  openGeneration: (entry: { path: string; seed: number }) => void;
  openHistory: (entry: PromptEntry) => void;
} {
  const [removing, setRemoving] = useState(false);
  const removingRef = useRef(false);
  const historyTicket = useRef(0);

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
    removingRef.current = true;
    setRemoving(true);
    void removeImageBackground(current.path)
      .then((image) => {
        if (image === null) {
          return;
        }
        setSlides((existing) =>
          existing.map((slide, position) =>
            position === index
              ? { ...slide, src: image.src, path: image.path, transparent: true }
              : slide,
          ),
        );
      })
      .catch(reportFailure)
      .finally(() => {
        removingRef.current = false;
        setRemoving(false);
      });
  }, [busy, reportFailure, setSlides, slideIndex, slides]);

  const openGeneration = useCallback(
    (entry: { path: string; seed: number }) => {
      if (busy || removingRef.current) {
        return;
      }
      void loadImageFile(entry.path)
        .then((image) => {
          if (image === null) {
            return;
          }
          setSlides([
            {
              src: image.src,
              path: image.path,
              seed: entry.seed,
              transparent: image.transparent,
            },
          ]);
          setSlideIndex(0);
          setView("workspace");
        })
        .catch(reportFailure);
    },
    [busy, reportFailure, setSlideIndex, setSlides, setView],
  );

  const openHistory = useCallback(
    (entry: PromptEntry) => {
      if (busy || removingRef.current) {
        return;
      }
      const ticket = historyTicket.current + 1;
      historyTicket.current = ticket;
      void loadPromptImages(entry)
        .then(async (files) => {
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
          setSlides(next);
          setSlideIndex(next.length === 0 ? 0 : next.length - 1);
          setView("workspace");
        })
        .catch(reportFailure);
    },
    [busy, reportFailure, setSlideIndex, setSlides, setView],
  );

  return { removing, openImage, removeBackground, openGeneration, openHistory };
}
