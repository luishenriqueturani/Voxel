import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  EMPTY_GALLERY_QUERY,
  type GalleryFacets,
  type GalleryItem,
  type GalleryQuery,
  loadGallery,
  loadGalleryFacets,
} from "../../native/history";
import { Button } from "../ui/button";
import { GalleryCompare } from "./GalleryCompare";
import { GalleryFilters } from "./GalleryFilters";

type GalleryViewProps = {
  onOpen: (item: GalleryItem) => void;
  onError: (error: unknown) => void;
};

const EMPTY_FACETS: GalleryFacets = { models: [], flows: [] };

function togglePick(current: string[], path: string): string[] {
  if (current.includes(path)) {
    return current.filter((item) => item !== path);
  }
  if (current.length < 2) {
    return [...current, path];
  }
  return [current[1] ?? path, path];
}

export function GalleryView({ onOpen, onError }: GalleryViewProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<GalleryQuery>(EMPTY_GALLERY_QUERY);
  const [facets, setFacets] = useState<GalleryFacets>(EMPTY_FACETS);
  const [items, setItems] = useState<GalleryItem[]>([]);
  const [picked, setPicked] = useState<string[]>([]);
  const [comparing, setComparing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadGalleryFacets()
      .then((loaded) => {
        if (!cancelled) {
          setFacets(loaded);
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

  useEffect(() => {
    let cancelled = false;
    void loadGallery(filter)
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
  }, [filter, onError]);

  const selected = items.filter((item) => picked.includes(item.path));
  const left = selected[0];
  const right = selected[1];
  const filteredEmpty =
    items.length === 0 &&
    (filter.prompt !== "" ||
      filter.modelId !== "" ||
      filter.flowId !== "" ||
      filter.kind !== "" ||
      filter.from !== "" ||
      filter.to !== "");

  return (
    <div className="flex w-full flex-col gap-4 self-start">
      <GalleryFilters filter={filter} facets={facets} onChange={setFilter} />
      {comparing && left && right ? (
        <GalleryCompare left={left} right={right} onBack={() => setComparing(false)} />
      ) : (
        <>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="ghost"
              disabled={selected.length !== 2}
              onClick={() => setComparing(true)}
            >
              {t("gallery.compare")}
            </Button>
            {picked.length > 0 ? (
              <Button
                variant="ghost"
                onClick={() => {
                  setPicked([]);
                  setComparing(false);
                }}
              >
                {t("gallery.compareClear")}
              </Button>
            ) : null}
          </div>
          {items.length === 0 ? (
            <p className="text-sm text-zinc-500">
              {filteredEmpty ? t("gallery.noMatch") : t("gallery.empty")}
            </p>
          ) : (
            <ul className="grid w-full grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-3">
              {items.map((item) => (
                <li key={item.path} className="relative">
                  <label className="absolute top-1 left-1 z-10 rounded bg-black/60 px-1">
                    <input
                      type="checkbox"
                      className="align-middle"
                      checked={picked.includes(item.path)}
                      aria-label={t("gallery.compareSelect", { prompt: item.prompt })}
                      onChange={() => setPicked((current) => togglePick(current, item.path))}
                    />
                  </label>
                  <button
                    type="button"
                    aria-label={item.prompt}
                    className="flex w-full flex-col gap-1 text-left"
                    onClick={() => onOpen(item)}
                  >
                    {item.preview === "" ? (
                      <span className="flex aspect-square w-full items-center justify-center rounded-md bg-zinc-200 text-xs text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300">
                        {item.kind === "mesh" ? t("gallery.kindMesh") : ""}
                      </span>
                    ) : (
                      <img
                        alt=""
                        src={item.preview}
                        className="aspect-square w-full rounded-md object-cover"
                      />
                    )}
                    <span className="truncate text-xs text-zinc-600 dark:text-zinc-300">
                      {item.kind === "mesh"
                        ? `${t("gallery.kindBadge3d")} · ${item.prompt}`
                        : item.prompt}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </div>
  );
}
