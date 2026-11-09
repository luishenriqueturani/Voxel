import { useTranslation } from "react-i18next";
import { useQueueJob } from "../../hooks/useQueueJob";
import { queueCounts } from "../../native/queue";
import { Button } from "../ui/button";
import { QueueControls } from "./QueueControls";

type QueueViewProps = {
  onBack: () => void;
};

const STATUSES = ["pending", "images", "mesh", "done", "failed", "cancelled"] as const;

function statusKey(status: string): (typeof STATUSES)[number] {
  return STATUSES.find((item) => item === status) ?? "pending";
}

export function QueueView({ onBack }: QueueViewProps) {
  const { t } = useTranslation();
  const { job, failure } = useQueueJob();
  const counts = queueCounts(job);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-6">
      <div>
        <Button variant="ghost" onClick={onBack}>
          {t("settings.back")}
        </Button>
      </div>
      <h1 className="text-lg font-semibold">{t("queue.title")}</h1>
      {failure !== null ? <p role="alert">{failure}</p> : null}
      {job === null ? (
        <p className="text-sm text-zinc-500">{t("queue.empty")}</p>
      ) : (
        <p className="text-sm">
          {t(counts.paused ? "queue.progressPaused" : "queue.progress", counts)}
        </p>
      )}
      <QueueControls />
      {job === null ? null : (
        <ol className="flex max-w-2xl flex-col gap-1">
          {job.items.map((item, index) => (
            <li
              key={item.id}
              className="flex items-baseline justify-between gap-3 rounded-md border border-zinc-200 px-3 py-2 text-sm dark:border-zinc-800"
            >
              <span className="min-w-0 truncate">
                {t("flows.item", { index: index + 1 })}: {item.prompt}
              </span>
              <span className="shrink-0 text-zinc-500">
                {t(`queue.itemStatus.${statusKey(item.status)}`)} · {item.images}/{item.count}
              </span>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
