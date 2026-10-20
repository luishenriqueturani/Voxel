import type { PythonProgress } from "../native/pythonEnv";

type Translate = (key: string, options?: { detail: string }) => string;

function gib(bytes: number): string {
  return (bytes / 1024 ** 3).toFixed(1);
}

export function environmentLabel(progress: PythonProgress | undefined, t: Translate): string {
  if (progress === undefined) {
    return t("status.unknown");
  }
  if (progress.phase === "ready") {
    return t("status.envReady");
  }
  if (progress.phase === "failed") {
    return t("status.envFailed");
  }
  if (progress.phase === "testing") {
    return t("status.envTesting");
  }
  if (progress.phase === "preparing") {
    const parts: string[] = [];
    if (progress.percent !== null) {
      parts.push(`${progress.percent}%`);
    }
    if (progress.totalBytes !== null) {
      const total = gib(progress.totalBytes);
      parts.push(
        progress.receivedBytes === null
          ? `${total} GiB`
          : `${gib(progress.receivedBytes)}/${total} GiB`,
      );
    }
    if (parts.length === 0) {
      return t("status.envPreparingPlain");
    }
    return t("status.envPreparing", { detail: parts.join(" · ") });
  }
  return t("status.envAbsent");
}
