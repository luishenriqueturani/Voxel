import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  enqueueMeshPicks,
  laterItems,
  loadQueueThumbnails,
  type QueueJob,
} from "../../native/queue";
import { Button } from "../ui/button";

type QueuePicksProps = {
  job: QueueJob;
  onQueued: (job: QueueJob) => void;
  onFailure: (message: string) => void;
};

export function QueuePicks({ job, onQueued, onFailure }: QueuePicksProps) {
  const { t } = useTranslation();
  const items = laterItems(job);
  const pathKey = items.flatMap((item) => item.imagePaths ?? []).join("\0");
  const [selected, setSelected] = useState<Record<string, string[]>>({});
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const paths = useMemo(() => (pathKey === "" ? [] : pathKey.split("\0")), [pathKey]);

  useEffect(() => {
    let cancelled = false;
    void loadQueueThumbnails(paths).then((next) => {
      if (!cancelled) {
        setThumbs(next);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [paths]);

  if (items.length === 0) {
    return null;
  }

  const picks = items.flatMap((item) => {
    const paths = selected[item.id] ?? [];
    return paths.length === 0 ? [] : [{ itemId: item.id, paths }];
  });

  function toggle(itemId: string, path: string) {
    setSelected((current) => {
      const known = current[itemId] ?? [];
      const next = known.includes(path)
        ? known.filter((entry) => entry !== path)
        : [...known, path];
      return { ...current, [itemId]: next };
    });
  }

  return (
    <section className="flex max-w-2xl flex-col gap-3 rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
      <h2 className="font-medium">{t("queue.picksTitle")}</h2>
      <p className="text-sm text-zinc-500">{t("queue.picksHint")}</p>
      {items.map((item) => (
        <div key={item.id} className="flex flex-col gap-2">
          <p className="text-sm font-medium">{item.prompt}</p>
          <ul className="flex flex-wrap gap-2">
            {(item.imagePaths ?? []).map((path, index) => {
              const checked = (selected[item.id] ?? []).includes(path);
              const src = thumbs[path];
              return (
                <li key={path}>
                  <label className="flex cursor-pointer flex-col gap-1">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={() => toggle(item.id, path)}
                      aria-label={t("queue.picksImage", { prompt: item.prompt, index: index + 1 })}
                    />
                    {src === undefined ? (
                      <span className="h-20 w-20 rounded bg-zinc-200 dark:bg-zinc-800" />
                    ) : (
                      <img alt="" src={src} className="h-20 w-20 rounded object-cover" />
                    )}
                  </label>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
      <Button
        disabled={picks.length === 0}
        onClick={() => {
          void enqueueMeshPicks(job.id, picks)
            .then(onQueued)
            .catch((error: unknown) => {
              onFailure(error instanceof Error ? error.message : String(error));
            });
        }}
      >
        {t("queue.picksSend")}
      </Button>
    </section>
  );
}
