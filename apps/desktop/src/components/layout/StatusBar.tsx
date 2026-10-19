import { isTauri } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { MachineStatus } from "../../native/status";

type StatusBarProps = {
  status: MachineStatus | null;
  onOpenSettings: () => void;
};

function gib(bytes: number): string {
  return (bytes / 1024 ** 3).toFixed(1);
}

export function StatusBar({ status, onOpenSettings }: StatusBarProps) {
  const { t } = useTranslation();
  const unknown = t("status.unknown");
  const memory = (used: number | null, total: number | null) =>
    used === null || total === null
      ? unknown
      : t("status.gib", { used: gib(used), total: gib(total) });
  const gpu = status?.gpuName && status.backend ? `${status.gpuName} (${status.backend})` : unknown;

  return (
    <footer className="flex min-h-8 shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-t border-zinc-200 px-3 py-1 text-xs text-zinc-600 dark:border-zinc-800 dark:text-zinc-400">
      <span>
        {t("status.gpu")}: {gpu}
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
        {t("status.environment")}: {unknown}
      </span>
      {isTauri() ? null : (
        <button
          type="button"
          onClick={onOpenSettings}
          className="ml-auto underline-offset-2 hover:underline"
        >
          {t("status.preferences")}
        </button>
      )}
    </footer>
  );
}
