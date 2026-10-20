import { type Dispatch, type SetStateAction, useCallback, useRef, useState } from "react";
import { openImageForEdit, removeImageBackground } from "../native/background";
import type { GalleryImage } from "../native/generation";

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
} {
  const [removing, setRemoving] = useState(false);
  const removingRef = useRef(false);

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

  return { removing, openImage, removeBackground };
}
