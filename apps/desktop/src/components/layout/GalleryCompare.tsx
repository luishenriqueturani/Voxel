import { useTranslation } from "react-i18next";
import type { GalleryItem } from "../../native/history";
import { Button } from "../ui/button";

type GalleryCompareProps = {
  left: GalleryItem;
  right: GalleryItem;
  onBack: () => void;
};

function Frame({ item, caption }: { item: GalleryItem; caption: string }) {
  const { t } = useTranslation();
  return (
    <figure className="flex min-w-0 flex-1 flex-col gap-2">
      <figcaption className="text-sm font-medium text-zinc-700 dark:text-zinc-200">
        {caption}
      </figcaption>
      {item.preview === "" ? (
        <div className="flex aspect-square w-full items-center justify-center rounded-md bg-zinc-200 text-sm text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300">
          {item.kind === "mesh" ? t("gallery.kindMesh") : t("gallery.emptyPreview")}
        </div>
      ) : (
        <img
          alt={item.prompt}
          src={item.preview}
          className="aspect-square w-full rounded-md object-contain"
        />
      )}
      <p className="truncate text-sm text-zinc-600 dark:text-zinc-300">{item.prompt}</p>
    </figure>
  );
}

export function GalleryCompare({ left, right, onBack }: GalleryCompareProps) {
  const { t } = useTranslation();
  return (
    <div className="flex w-full flex-col gap-3">
      <Button variant="ghost" className="self-start" onClick={onBack}>
        {t("gallery.compareBack")}
      </Button>
      <div className="grid w-full grid-cols-1 gap-4 md:grid-cols-2">
        <Frame item={left} caption={t("gallery.compareA")} />
        <Frame item={right} caption={t("gallery.compareB")} />
      </div>
    </div>
  );
}
