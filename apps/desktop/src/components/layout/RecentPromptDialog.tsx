import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { loadPromptImages, type PromptEntry, type PromptImage } from "../../native/history";
import { Button } from "../ui/button";

type RecentPromptDialogProps = {
  entry: PromptEntry;
  onClose: () => void;
  onRepeat: () => void;
  onError: (error: unknown) => void;
};

export function RecentPromptDialog({ entry, onClose, onRepeat, onError }: RecentPromptDialogProps) {
  const { t } = useTranslation();
  const [images, setImages] = useState<PromptImage[]>([]);

  useEffect(() => {
    let cancelled = false;
    void loadPromptImages(entry)
      .then((loaded) => {
        if (!cancelled) {
          setImages(loaded);
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

  const negative =
    entry.negativePrompt.trim() === "" ? t("panel.recentNoNegative") : entry.negativePrompt;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
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
        className="relative z-10 flex max-h-full w-full max-w-lg flex-col gap-3 overflow-y-auto rounded-lg border border-zinc-200 bg-white p-4 shadow-lg dark:border-zinc-800 dark:bg-zinc-950"
      >
        <h2 id="recent-prompt-title" className="text-base font-medium">
          {entry.prompt}
        </h2>
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
        {images.length === 0 ? (
          <p className="text-sm text-zinc-500">{t("panel.recentImagesEmpty")}</p>
        ) : (
          <ul className="flex flex-wrap gap-2">
            {images.map((image) => {
              const name = fileName(image.path);
              const src = imageSrc(image.path);
              return (
                <li key={`${image.path}:${image.seed}`}>
                  {src ? (
                    <img alt={name} src={src} className="h-20 w-20 rounded object-cover" />
                  ) : (
                    <span>{name}</span>
                  )}
                </li>
              );
            })}
          </ul>
        )}
        <div className="flex gap-2">
          <Button onClick={onRepeat}>{t("panel.repeat")}</Button>
          <Button variant="ghost" onClick={onClose}>
            {t("panel.close")}
          </Button>
        </div>
      </section>
    </div>
  );
}

function imageSrc(path: string): string | null {
  if (!isTauri()) {
    return null;
  }
  return convertFileSrc(path);
}

function fileName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}
