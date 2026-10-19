import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";

type ErrorModalProps = {
  summary: string;
  log: string;
  copied: boolean;
  onCopy: () => void;
  onOpenLog: () => void;
  onOpenDiagnosis: () => void;
  onRestoreRecommended?: () => void;
  onClose: () => void;
};

export function ErrorModal({
  summary,
  log,
  copied,
  onCopy,
  onOpenLog,
  onOpenDiagnosis,
  onRestoreRecommended,
  onClose,
}: ErrorModalProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLElement>(null);

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label={t("error.dismiss")}
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />
      <section
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="error-dialog-title"
        tabIndex={-1}
        className="relative z-10 flex max-h-full w-full max-w-xl flex-col gap-4 overflow-hidden rounded-lg border border-zinc-200 bg-white p-5 shadow-lg outline-none dark:border-zinc-800 dark:bg-zinc-950"
      >
        <div className="flex flex-col gap-2">
          <h2 id="error-dialog-title" className="text-lg font-semibold">
            {t("error.title")}
          </h2>
          <p>{summary}</p>
        </div>
        <section
          aria-label={t("error.log")}
          className="max-h-64 overflow-auto rounded-md bg-zinc-100 dark:bg-zinc-900"
        >
          <pre className="whitespace-pre-wrap p-3 font-mono text-xs text-zinc-800 dark:text-zinc-200">
            {log}
          </pre>
        </section>
        <div className="flex flex-wrap gap-2">
          <Button onClick={onCopy}>{copied ? t("error.copied") : t("error.copy")}</Button>
          <Button variant="ghost" onClick={onOpenLog}>
            {t("error.openLog")}
          </Button>
          <Button variant="ghost" onClick={onOpenDiagnosis}>
            {t("error.openDiagnosis")}
          </Button>
          {onRestoreRecommended ? (
            <Button onClick={onRestoreRecommended}>{t("error.restore")}</Button>
          ) : null}
          <Button variant="ghost" onClick={onClose}>
            {t("error.close")}
          </Button>
        </div>
      </section>
    </div>
  );
}
