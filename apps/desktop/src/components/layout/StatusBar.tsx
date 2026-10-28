import { isTauri } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useDownloadQueue } from "../../downloads/DownloadProvider";
import { environmentLabel } from "../../lib/pythonStatus";
import type { PythonProgress } from "../../native/pythonEnv";
import type { MachineStatus } from "../../native/status";

type StatusBarProps = {
  status: MachineStatus | null;
  environment?: PythonProgress;
  onOpenSettings: () => void;
};

function imageBackendName(
  backend: string | null | undefined,
  t: (key: string) => string,
  unknown: string,
): string {
  if (backend === "vulkan" || backend === "rocm" || backend === "cuda" || backend === "cpu") {
    return t(`setup.backends.${backend}`);
  }
  return backend && backend !== "" ? backend : unknown;
}

function gib(bytes: number): string {
  return (bytes / 1024 ** 3).toFixed(1);
}

export function StatusBar({ status, environment, onOpenSettings }: StatusBarProps) {
  const { t } = useTranslation();
  const downloads = useDownloadQueue();
  const unknown = t("status.unknown");
  const download = downloadLabel(downloads, t);
  const memory = (used: number | null, total: number | null) =>
    used === null || total === null
      ? unknown
      : t("status.gib", { used: gib(used), total: gib(total) });
  const gpu = status?.gpuName ? status.gpuName : unknown;
  const backend = imageBackendName(status?.backend, t, unknown);

  return (
    <footer className="flex min-h-8 shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-t border-zinc-200 px-3 py-1 text-xs text-zinc-600 dark:border-zinc-800 dark:text-zinc-400">
      <span>
        {t("status.gpu")}: {gpu}
      </span>
      <span>
        {t("status.imageBackend")}: {backend}
      </span>
      <span>
        {t("status.vram")}: {memory(status?.vramUsedBytes ?? null, status?.vramTotalBytes ?? null)}
      </span>
      <span>
        {t("status.gpuUse")}:{" "}
        {status?.gpuPercent === null || status?.gpuPercent === undefined
          ? unknown
          : t("status.percent", { value: status.gpuPercent })}
      </span>
      <span>
        {t("status.cpu")}:{" "}
        {status?.cpuPercent === null || status?.cpuPercent === undefined
          ? unknown
          : t("status.percent", { value: status.cpuPercent })}
      </span>
      <span>
        {t("status.ram")}: {memory(status?.ramUsedBytes ?? null, status?.ramTotalBytes ?? null)}
      </span>
      <span>
        {t("status.cpuTemp")}:{" "}
        {status?.cpuCelsius === null || status?.cpuCelsius === undefined
          ? unknown
          : t("status.celsius", { value: status.cpuCelsius })}
      </span>
      <span>
        {t("status.gpuTemp")}:{" "}
        {status?.gpuCelsius === null || status?.gpuCelsius === undefined
          ? unknown
          : t("status.celsius", { value: status.gpuCelsius })}
      </span>
      <span>
        {t("status.queue")}: {unknown}
      </span>
      <span>
        {t("status.environment")}: {environmentLabel(environment, t)}
      </span>
      {download === null && isTauri() ? null : (
        <div className="ml-auto flex min-w-0 items-center gap-3">
          {download === null ? null : (
            <span className="truncate" title={download} aria-live="polite">
              {download}
            </span>
          )}
          {isTauri() ? null : (
            <button
              type="button"
              onClick={onOpenSettings}
              className="underline-offset-2 hover:underline"
            >
              {t("status.preferences")}
            </button>
          )}
        </div>
      )}
    </footer>
  );
}

function downloadLabel(
  downloads: ReturnType<typeof useDownloadQueue>,
  t: (key: string, values?: Record<string, unknown>) => string,
): string | null {
  const active = downloads.active;
  if (active === null) {
    return null;
  }
  const progress = downloads.progress;
  const verifying =
    progress?.phase === "verify" ||
    (progress !== null && progress.total > 0 && progress.received >= progress.total);
  const percent =
    progress !== null && progress.total > 0
      ? ` ${Math.min(100, Math.round((progress.received / progress.total) * 100))}%`
      : "";
  const waiting =
    downloads.waiting.length > 0
      ? ` · ${t("status.downloadWaiting", { count: downloads.waiting.length })}`
      : "";
  const name = verifying
    ? t("status.downloadVerify", { name: active.label })
    : t("status.download", { name: active.label });
  return `${name}${percent}${waiting}`;
}
