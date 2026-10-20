import { useTranslation } from "react-i18next";

type PowerNoticeProps = {
  gpu: string | null;
  onDismiss: () => void;
};

export function PowerNotice({ gpu, onDismiss }: PowerNoticeProps) {
  const { t } = useTranslation();
  if (gpu === null) {
    return null;
  }
  return (
    <div
      role="status"
      className="flex items-start gap-4 border-b border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-100"
    >
      <p className="flex-1">{t("power.body", { gpu })}</p>
      <button
        type="button"
        className="shrink-0 rounded-md border border-amber-400 px-3 py-1 dark:border-amber-700"
        onClick={onDismiss}
      >
        {t("power.dismiss")}
      </button>
    </div>
  );
}
