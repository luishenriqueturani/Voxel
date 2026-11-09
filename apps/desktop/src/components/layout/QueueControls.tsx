import { useTranslation } from "react-i18next";
import { useQueueJob } from "../../hooks/useQueueJob";
import {
  cancelQueue,
  movableItems,
  pauseQueue,
  reorderQueueItems,
  resumeQueue,
} from "../../native/queue";
import { Button } from "../ui/button";

export function QueueControls() {
  const { t } = useTranslation();
  const { job, failure, run } = useQueueJob();

  if (job === null && failure === null) {
    return null;
  }

  const pending = job === null ? [] : movableItems(job);
  const paused = job?.status === "paused";

  async function move(from: number, to: number) {
    if (job === null || to < 0 || to >= pending.length) {
      return;
    }
    const ids = pending.map((item) => item.id);
    const swapped = ids.slice();
    const [removed] = swapped.splice(from, 1);
    if (removed === undefined) {
      return;
    }
    swapped.splice(to, 0, removed);
    await run(() => reorderQueueItems(job.id, swapped));
  }

  return (
    <section className="flex max-w-2xl flex-col gap-2 rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
      {job !== null ? (
        <p>
          {paused ? t("queue.paused") : t("queue.running")}: {job.name}
        </p>
      ) : null}
      {failure !== null ? <p role="alert">{failure}</p> : null}
      {job !== null ? (
        <div className="flex flex-wrap gap-2">
          {paused ? (
            <Button onClick={() => void run(resumeQueue)}>{t("queue.resume")}</Button>
          ) : (
            <Button onClick={() => void run(pauseQueue)}>{t("queue.pause")}</Button>
          )}
          <Button
            variant="ghost"
            onClick={() =>
              void run(async () => {
                await cancelQueue();
                return undefined;
              })
            }
          >
            {t("queue.cancel")}
          </Button>
        </div>
      ) : null}
      {pending.length > 0 ? (
        <ol className="flex flex-col gap-1">
          <li className="text-sm font-medium">{t("queue.pending")}</li>
          {pending.map((item, index) => (
            <li key={item.id} className="flex items-center gap-2">
              <span className="flex-1 truncate text-sm">{item.prompt}</span>
              <Button
                variant="ghost"
                disabled={index === 0}
                onClick={() => void move(index, index - 1)}
              >
                {t("queue.moveUp")}
              </Button>
              <Button
                variant="ghost"
                disabled={index === pending.length - 1}
                onClick={() => void move(index, index + 1)}
              >
                {t("queue.moveDown")}
              </Button>
            </li>
          ))}
        </ol>
      ) : null}
    </section>
  );
}
