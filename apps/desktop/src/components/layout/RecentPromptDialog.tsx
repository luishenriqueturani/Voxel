import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { promptLabel } from "../../lib/promptTitle";
import { copyPreview, saveGeneratedImage } from "../../native/generation";
import { loadPromptImages, type PromptEntry, type PromptImage } from "../../native/history";
import { Button } from "../ui/button";
import { ImageViewer } from "./ImageViewer";

type RecentPromptDialogProps = {
  entry: PromptEntry;
  onClose: () => void;
  onRepeat: () => void;
  onError: (error: unknown) => void;
};

function ignore() {
  return undefined;
}

export function RecentPromptDialog({ entry, onClose, onRepeat, onError }: RecentPromptDialogProps) {
  const { t } = useTranslation();
  const [images, setImages] = useState<PromptImage[]>([]);
  const [index, setIndex] = useState(0);

  useEffect(() => {
    let cancelled = false;
    void loadPromptImages(entry)
      .then((loaded) => {
        if (!cancelled) {
          setImages(loaded);
          setIndex(0);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          onError(error);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [entry, onError]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const current = images[index] ?? null;
  const src = current === null ? null : viewerSrc(current.path);
  const heading = promptLabel(entry);
  const negative =
    entry.negativePrompt.trim() === "" ? t("panel.recentNoNegative") : entry.negativePrompt;

  return (
    <div className="fixed inset-0 z-50 flex p-4">
      <button
        type="button"
        aria-label={t("panel.close")}
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="recent-prompt-title"
        className="relative z-10 flex h-full w-full min-h-0 overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-lg dark:border-zinc-800 dark:bg-zinc-950"
      >
        <div className="flex w-80 shrink-0 flex-col gap-3 overflow-y-auto border-r border-zinc-200 p-4 dark:border-zinc-800">
          <h2 id="recent-prompt-title" className="text-base font-medium">
            {heading}
          </h2>
          {heading === entry.prompt ? null : (
            <p className="text-sm text-zinc-600 dark:text-zinc-300">{entry.prompt}</p>
          )}
          <p>
            {t("panel.negative")}: {negative}
          </p>
          <p>
            {t("panel.model")}: {entry.modelId}
          </p>
          <p>
            {t("panel.steps")}: {entry.steps}
          </p>
          <p>{t("panel.size", { width: entry.width, height: entry.height })}</p>
          <p>
            {t("panel.cfg")}: {entry.cfg}
          </p>
          <p>
            {t("panel.seed")}: {entry.seed}
          </p>
          <h3 className="text-sm font-medium">{t("panel.recentImages")}</h3>
          {current === null ? (
            <p className="text-sm text-zinc-500">{t("panel.recentImagesEmpty")}</p>
          ) : (
            <p>{fileName(current.path)}</p>
          )}
          <div className="mt-auto flex gap-2">
            <Button onClick={onRepeat}>{t("panel.repeat")}</Button>
            <Button variant="ghost" onClick={onClose}>
              {t("panel.close")}
            </Button>
          </div>
        </div>
        <div className="min-h-0 min-w-0 flex-1 p-4" data-testid="recent-viewer">
          <ImageViewer
            src={src}
            step={0}
            total={0}
            canSave={current !== null}
            canRegenerate={false}
            index={index}
            count={images.length}
            onPrevious={() => setIndex((currentIndex) => Math.max(0, currentIndex - 1))}
            onNext={() => setIndex((currentIndex) => Math.min(images.length - 1, currentIndex + 1))}
            onSave={() => {
              if (current) {
                void saveGeneratedImage(current.path).catch(onError);
              }
            }}
            onCopy={() => {
              if (src) {
                void copyPreview(src).catch(onError);
              }
            }}
            onRegenerate={ignore}
            viewOnly
          />
        </div>
      </section>
    </div>
  );
}

function viewerSrc(path: string): string {
  if (isTauri()) {
    return convertFileSrc(path);
  }
  return path;
}

function fileName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}
