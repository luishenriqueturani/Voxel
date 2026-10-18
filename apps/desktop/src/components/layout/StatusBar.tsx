import { isTauri } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

type StatusBarProps = {
  onOpenSettings: () => void;
};

export function StatusBar({ onOpenSettings }: StatusBarProps) {
  const { t } = useTranslation();
  const unknown = t("status.unknown");

  return (
    <footer className="flex h-8 shrink-0 items-center gap-4 border-t border-zinc-200 px-3 text-xs text-zinc-600 dark:border-zinc-800 dark:text-zinc-400">
      <span>
        {t("status.gpu")}: {unknown}
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
