import { useTranslation } from "react-i18next";
import { openLocalPath } from "../../native/files";
import { formatElapsed, type QueueJob, queueSummary } from "../../native/queue";
import { Button } from "../ui/button";

type QueueReportProps = {
  job: QueueJob;
};

export function QueueReport({ job }: QueueReportProps) {
  const { t } = useTranslation();
  const summary = queueSummary(job);
  const elapsed = formatElapsed(summary.elapsedSecs);
  const time =
    elapsed.hours > 0
      ? t("queue.elapsedHours", elapsed)
      : t("queue.elapsedMinutes", { minutes: elapsed.minutes });
  const failed = job.items.filter((item) => item.status === "failed");

  return (
    <section className="flex max-w-2xl flex-col gap-2 rounded-md border border-zinc-200 p-3 text-sm dark:border-zinc-800">
      <h2 className="font-medium">{t("queue.reportTitle")}</h2>
      <p>{t("queue.reportDone", { count: summary.done })}</p>
      <p>{t("queue.reportFailed", { count: summary.failed })}</p>
      <p>{t("queue.reportCancelled", { count: summary.cancelled })}</p>
      <p>{t("queue.reportTime", { time })}</p>
      {failed.length === 0 ? null : (
        <ul className="flex flex-col gap-1">
          {failed.map((item) => (
            <li key={item.id} className="flex items-center justify-between gap-2">
              <span className="min-w-0 truncate">{item.prompt}</span>
              {item.errorLog ? (
                <Button variant="ghost" onClick={() => void openLocalPath(item.errorLog ?? "")}>
                  {t("queue.openLog")}
                </Button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
