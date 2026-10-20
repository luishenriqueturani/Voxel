import { useTranslation } from "react-i18next";
import type { PythonProgress } from "../../native/pythonEnv";
import { Button } from "../ui/button";

type EnvironmentSectionProps = {
  progress: PythonProgress | null;
  onReinstall: () => void;
  onRemove: () => void;
};

const BACKENDS = ["cuda", "rocm_native", "rocm_safe_override", "xpu", "cpu"] as const;

function isBackend(value: string): value is (typeof BACKENDS)[number] {
  return BACKENDS.some((backend) => backend === value);
}

export function EnvironmentSection({ progress, onReinstall, onRemove }: EnvironmentSectionProps) {
  const { t } = useTranslation();
  const phase = progress?.phase ?? "absent";
  const working = phase === "preparing" || phase === "testing";
  const backend = progress?.backend;
  const state =
    phase === "ready" && backend !== null && backend !== undefined && isBackend(backend)
      ? t("settings.envReady", { backend: t(`settings.pythonBackends.${backend}`) })
      : phase === "failed"
        ? t("settings.envFailed")
        : phase === "preparing" || phase === "testing"
          ? t("settings.envWorking")
          : t("settings.envAbsent");

  return (
    <div className="flex max-w-sm flex-col gap-3 text-sm">
      <p>{t("settings.envState", { state })}</p>
      <div className="flex gap-2">
        <Button type="button" disabled={working} onClick={onReinstall}>
          {t("settings.envReinstall")}
        </Button>
        <Button
          type="button"
          variant="ghost"
          disabled={working || phase === "absent"}
          onClick={onRemove}
        >
          {t("settings.envRemove")}
        </Button>
      </div>
    </div>
  );
}
