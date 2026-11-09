import { useCallback, useEffect, useState } from "react";
import { loadUnfinishedQueueJob, type QueueJob } from "../native/queue";

const POLL_MS = 2000;

export function useQueueJob() {
  const [job, setJob] = useState<QueueJob | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await loadUnfinishedQueueJob();
      setJob(next);
      setFailure(null);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), POLL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [refresh]);

  async function run(work: () => Promise<QueueJob | undefined>) {
    try {
      const next = await work();
      setJob(next === undefined ? null : next);
      setFailure(null);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  return { job, failure, run };
}
