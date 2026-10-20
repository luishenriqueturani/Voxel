import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type GalleryItem, loadGallery } from "../../native/history";

type GalleryViewProps = {
  onOpen: (item: GalleryItem) => void;
  onError: (error: unknown) => void;
};

export function GalleryView({ onOpen, onError }: GalleryViewProps) {
  const { t } = useTranslation();
  const [items, setItems] = useState<GalleryItem[]>([]);

  useEffect(() => {
    let cancelled = false;
    void loadGallery()
      .then((loaded) => {
        if (!cancelled) {
          setItems(loaded);
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
  }, [onError]);

  if (items.length === 0) {
    return <p className="text-sm text-zinc-500">{t("gallery.empty")}</p>;
  }

  return (
    <ul className="grid w-full grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-3 self-start">
      {items.map((item) => (
        <li key={item.path}>
          <button
            type="button"
            aria-label={item.prompt}
            className="flex w-full flex-col gap-1 text-left"
            onClick={() => onOpen(item)}
          >
            {item.preview === "" ? (
              <span className="aspect-square w-full rounded-md bg-zinc-200 dark:bg-zinc-800" />
            ) : (
              <img
                alt=""
                src={item.preview}
                className="aspect-square w-full rounded-md object-cover"
              />
            )}
            <span className="truncate text-xs text-zinc-600 dark:text-zinc-300">{item.prompt}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}
