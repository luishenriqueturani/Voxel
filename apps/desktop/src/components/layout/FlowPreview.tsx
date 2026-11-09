import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueueJob } from "../../hooks/useQueueJob";
import {
  imageSrc,
  listenToGenerationProgress,
  type GenerationProgress,
} from "../../native/generation";
import {
  itemStatusKey,
  loadQueueThumbnails,
  previewQueueItem,
  queueCounts,
  queueJobFinished,
  type QueueJob,
} from "../../native/queue";
import { ImageStage } from "./ImageStage";
import { QueueControls } from "./QueueControls";

type FlowPreviewProps = {
  queued: boolean;
};

type Frame = {
  step: number;
  total: number;
  preview: string | null;
  image: number;
  images: number;
};

export function FlowPreview({ queued }: FlowPreviewProps) {
  const { t } = useTranslation();
  const { job, refresh } = useQueueJob(true);
  const [pickedId, setPickedId] = useState<string | null>(null);
  const [shownPath, setShownPath] = useState<string | null>(null);
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const [frame, setFrame] = useState<Frame | null>(null);
  const counts = queueCounts(job);
  const finished = job !== null && queueJobFinished(job);
  const jobId = job?.id;
  const active = job === null ? null : previewQueueItem(job, pickedId);
  const activeId = active?.id;
  const paths = active?.imagePaths ?? [];
  const pathKey = paths.join("\0");
  const generating =
    job !== null && !finished && (active?.status === "images" || active?.status === "mesh");
  const liveSrc = frame?.preview ? imageSrc(frame.preview) : null;
  const thumbSrc = shownPath === null ? undefined : thumbs[shownPath];
  const src = generating && liveSrc !== null ? liveSrc : (thumbSrc ?? liveSrc);

  useEffect(() => {
    if (queued) {
      void refresh();
    }
  }, [queued, refresh]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: só troca quando o job muda de id
  useEffect(() => {
    setPickedId(null);
    setShownPath(null);
    setFrame(null);
  }, [jobId]);

  useEffect(() => {
    if (activeId === undefined) {
      setShownPath(null);
      return;
    }
    const next = pathKey === "" ? [] : pathKey.split("\0");
    setShownPath(generating ? (next[next.length - 1] ?? null) : (next[0] ?? null));
  }, [activeId, generating, pathKey]);

  useEffect(() => {
    const all = job === null ? [] : job.items.flatMap((item) => item.imagePaths ?? []);
    let cancelled = false;
    void loadQueueThumbnails(all).then((next) => {
      if (!cancelled) {
        setThumbs(next);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [job]);

  useEffect(() => {
    let stop: () => void = () => undefined;
    let cancelled = false;
    void listenToGenerationProgress((progress) => {
      setFrame(frameFrom(progress));
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop();
    };
  }, []);

  return (
    <aside
      aria-label={t("flows.preview")}
      className="flex min-h-0 min-w-[18rem] flex-1 flex-col gap-3 overflow-hidden border-l border-zinc-200 p-4 dark:border-zinc-800"
    >
      <header className="flex items-baseline justify-between gap-2">
        <h2 className="text-lg font-semibold">{t("flows.preview")}</h2>
        {job === null ? null : (
          <p
            role="status"
            aria-label={t(counts.paused ? "queue.progressPaused" : "queue.progress", counts)}
          >
            {t(counts.paused ? "queue.progressPaused" : "queue.progress", counts)}
          </p>
        )}
      </header>
      <figure className="relative flex min-h-48 flex-1 flex-col overflow-hidden rounded-md bg-zinc-100 dark:bg-zinc-950">
        {src === null || src === undefined ? (
          <figcaption className="m-auto px-4 text-center text-sm text-zinc-500">
            {job === null
              ? t("flows.previewEmpty")
              : generating
                ? t("generation.preparing")
                : t("flows.previewNone")}
          </figcaption>
        ) : (
          <ImageStage src={src} alt={t("generation.preview")} hint>
            {generating && frame !== null && frame.total > 0 ? (
              <p className="absolute bottom-2 left-2 rounded bg-black/70 px-2 py-1 text-xs text-white">
                {t("generation.step", { step: frame.step, total: frame.total })}
                {frame.images > 1
                  ? ` · ${t("generation.image", { image: frame.image, images: frame.images })}`
                  : ""}
              </p>
            ) : null}
          </ImageStage>
        )}
      </figure>
      {paths.length > 1 ? (
        <ul className="flex shrink-0 gap-2 overflow-x-auto">
          {paths.map((path, index) => {
            const thumb = thumbs[path];
            return (
              <li key={path}>
                <button
                  type="button"
                  aria-pressed={path === shownPath}
                  aria-label={t("flows.previewImage", { index: index + 1 })}
                  className="rounded border border-transparent aria-pressed:border-zinc-900 dark:aria-pressed:border-zinc-100"
                  onClick={() => setShownPath(path)}
                >
                  {thumb === undefined ? (
                    <span className="block h-16 w-16 rounded bg-zinc-200 dark:bg-zinc-800" />
                  ) : (
                    <img alt="" src={thumb} className="h-16 w-16 rounded object-cover" />
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}
      {job === null ? null : (
        <ItemList job={job} activeId={active?.id ?? null} onPick={setPickedId} />
      )}
      <QueueControls />
    </aside>
  );
}

function ItemList({
  job,
  activeId,
  onPick,
}: {
  job: QueueJob;
  activeId: string | null;
  onPick: (id: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <ol className="flex max-h-48 shrink-0 flex-col gap-1 overflow-y-auto">
      {job.items.map((item, index) => (
        <li key={item.id}>
          <button
            type="button"
            aria-pressed={item.id === activeId}
            className="flex w-full items-baseline justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-zinc-200 aria-pressed:bg-zinc-200 dark:hover:bg-zinc-800 dark:aria-pressed:bg-zinc-800"
            onClick={() => onPick(item.id)}
          >
            <span className="min-w-0 truncate">
              {t("flows.item", { index: index + 1 })}: {item.prompt}
            </span>
            <span className="shrink-0 text-zinc-500">
              {t(`queue.itemStatus.${itemStatusKey(item.status)}`)} · {item.images}/{item.count}
            </span>
          </button>
        </li>
      ))}
    </ol>
  );
}

function frameFrom(progress: GenerationProgress): Frame {
  return {
    step: progress.step,
    total: progress.total,
    preview: progress.preview,
    image: (progress.index ?? 0) + 1,
    images: progress.count ?? 1,
  };
}
