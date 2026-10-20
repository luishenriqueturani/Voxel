import { type Dispatch, type MutableRefObject, type SetStateAction, useCallback } from "react";
import type { GenerationParams } from "../components/layout/PromptPanel";
import { canStartEdit, type EditPayload, generationRequest } from "../lib/edit";
import { type GalleryImage, generateImage, imageSrc, isCancelled } from "../native/generation";

type Frame = {
  step: number;
  total: number;
  preview: string | null;
};

type GenerationRequestOptions = {
  generatingRef: MutableRefObject<boolean>;
  removing: boolean;
  editRef: MutableRefObject<EditPayload>;
  holdEdit: () => void;
  commitEdit: () => void;
  discardEdit: () => void;
  restoreEdit: () => void;
  setGenerating: (value: boolean) => void;
  setSlides: Dispatch<SetStateAction<GalleryImage[]>>;
  setSlideIndex: Dispatch<SetStateAction<number>>;
  setFrame: Dispatch<SetStateAction<Frame | null>>;
  setHistoryRevision: Dispatch<SetStateAction<number>>;
  params: GenerationParams;
  prompt: string;
  negativePrompt: string;
  modelId: string;
  reportFailure: (error: unknown) => void;
};

export function useGenerationRequest({
  generatingRef,
  removing,
  editRef,
  holdEdit,
  commitEdit,
  discardEdit,
  restoreEdit,
  setGenerating,
  setSlides,
  setSlideIndex,
  setFrame,
  setHistoryRevision,
  params,
  prompt,
  negativePrompt,
  modelId,
  reportFailure,
}: GenerationRequestOptions): (seedOverride?: number) => void {
  return useCallback(
    (seedOverride?: number) => {
      if (generatingRef.current || removing) {
        return;
      }
      const edit = editRef.current;
      if (!canStartEdit(edit)) {
        return;
      }
      const editing = edit.mode === "vary" || edit.mode === "inpaint";
      holdEdit();
      generatingRef.current = true;
      setGenerating(true);
      if (edit.mode !== "vary" && edit.mode !== "inpaint") {
        setSlides([]);
        setSlideIndex(0);
      }
      setFrame(null);
      const typed = params.seed.trim() === "" ? null : Number(params.seed);
      const seed = seedOverride ?? (typed !== null && Number.isFinite(typed) ? typed : null);
      void generateImage(
        generationRequest(prompt, negativePrompt, modelId, params, seed, edit),
        (progress) => {
          setFrame((current) => ({
            step: progress.step,
            total: progress.total,
            preview: progress.preview ?? current?.preview ?? null,
          }));
          if (
            progress.ready &&
            progress.preview &&
            progress.path &&
            typeof progress.seed === "number"
          ) {
            const slide = {
              src: imageSrc(progress.preview),
              path: progress.path,
              seed: progress.seed,
            };
            const index = progress.index ?? 0;
            setSlides((current) => {
              const next = current.slice();
              next[index] = slide;
              return next;
            });
            setSlideIndex(index);
          }
        },
      )
        .then((image) => {
          if (image === null || image.images.length === 0) {
            return;
          }
          if (editing) {
            commitEdit();
          }
          setSlides(image.images);
          setSlideIndex(image.images.length - 1);
          setHistoryRevision((current) => current + 1);
        })
        .catch((error: unknown) => {
          if (isCancelled(error)) {
            restoreEdit();
            return;
          }
          reportFailure(error);
        })
        .finally(() => {
          discardEdit();
          generatingRef.current = false;
          setGenerating(false);
        });
    },
    [
      commitEdit,
      discardEdit,
      editRef,
      generatingRef,
      holdEdit,
      modelId,
      negativePrompt,
      params,
      prompt,
      removing,
      reportFailure,
      restoreEdit,
      setFrame,
      setGenerating,
      setHistoryRevision,
      setSlideIndex,
      setSlides,
    ],
  );
}
