import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { DownloadProgress, HubFile, ListedModel, ListedReason } from "../../native/models";
import { Button } from "../ui/button";
import { HubSearch } from "./HubSearch";

type ModelsSectionProps = {
  hfTokenConfigured: boolean;
  onHfTokenSave: (token: string) => void;
  onHfTokenClear: () => void;
  models: ListedModel[];
  progress?: DownloadProgress | null;
  onDownload: (id: string) => void;
  onRemove: (filename: string) => void;
  onOpenFolder: () => void;
  onImport: () => void;
  onSearchHub?: (query: string) => Promise<HubFile[]>;
  onDownloadHub?: (file: HubFile) => Promise<void> | void;
};

export function ModelsSection({
  hfTokenConfigured,
  onHfTokenSave,
  onHfTokenClear,
  models,
  progress = null,
  onDownload,
  onRemove,
  onOpenFolder,
  onImport,
  onSearchHub,
  onDownloadHub,
}: ModelsSectionProps) {
  const { t } = useTranslation();
  const [tokenDraft, setTokenDraft] = useState("");
  const [showIncompatible, setShowIncompatible] = useState(false);
  const [pending, setPending] = useState<ListedModel | null>(null);
  const recommended = models.filter(
    (model) => model.catalog && (showIncompatible || model.compatibility !== "incompatible"),
  );
  const downloaded = models.filter((model) => !model.catalog);

  return (
    <div className="grid grid-cols-[minmax(0,36rem)_minmax(0,1fr)] items-start gap-8 text-sm">
      <div className="flex min-w-0 flex-col gap-4">
        <label className="flex max-w-sm flex-col gap-1">
          {t("settings.token")}
          <input
            type="password"
            autoComplete="off"
            value={tokenDraft}
            onChange={(event) => setTokenDraft(event.target.value)}
            className="rounded-md border border-zinc-300 bg-white px-2 py-1.5 dark:border-zinc-700 dark:bg-zinc-950"
          />
        </label>
        <p className="text-zinc-500">{t("settings.tokenHint")}</p>
        {hfTokenConfigured ? <p>{t("settings.tokenSaved")}</p> : null}
        <div className="flex gap-2">
          <Button
            onClick={() => {
              onHfTokenSave(tokenDraft);
              setTokenDraft("");
            }}
            disabled={tokenDraft.trim() === ""}
          >
            {t("settings.tokenSave")}
          </Button>
          {hfTokenConfigured ? (
            <Button variant="ghost" onClick={onHfTokenClear}>
              {t("settings.tokenClear")}
            </Button>
          ) : null}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="ghost" onClick={onImport}>
            {t("models.import")}
          </Button>
          <Button variant="ghost" onClick={onOpenFolder}>
            {t("models.openFolder")}
          </Button>
        </div>
        <section className="flex flex-col gap-3">
          <h3 className="font-medium">{t("models.downloaded")}</h3>
          {downloaded.length === 0 ? (
            <p className="text-zinc-500">{t("models.downloadedEmpty")}</p>
          ) : (
            <ModelList
              models={downloaded}
              progress={progress}
              onDownload={setPending}
              onRemove={onRemove}
            />
          )}
        </section>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={showIncompatible}
            onChange={(event) => setShowIncompatible(event.target.checked)}
          />
          {t("models.showIncompatible")}
        </label>
        <section className="flex flex-col gap-3">
          <h3 className="font-medium">{t("models.recommended")}</h3>
          <ModelList
            models={recommended}
            progress={progress}
            onDownload={setPending}
            onRemove={onRemove}
          />
        </section>
        {pending ? (
          <DownloadDialog
            model={pending}
            onCancel={() => setPending(null)}
            onConfirm={() => {
              onDownload(pending.id);
              setPending(null);
            }}
          />
        ) : null}
      </div>
      <div className="min-w-0 border-l border-zinc-200 pl-8 dark:border-zinc-800">
        <HubSearch
          hfTokenConfigured={hfTokenConfigured}
          progress={progress}
          onSearch={onSearchHub}
          onDownload={onDownloadHub}
        />
      </div>
    </div>
  );
}

function ModelList({
  models,
  progress,
  onDownload,
  onRemove,
}: {
  models: ListedModel[];
  progress: DownloadProgress | null;
  onDownload: (model: ListedModel) => void;
  onRemove: (filename: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <ul className="flex flex-col gap-3">
      {models.map((model) => (
        <li key={model.id} className="rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="font-medium">{model.name}</p>
              <p className="text-zinc-500">
                {t(`models.seals.${model.compatibility}`)}
                {model.reason ? ` — ${reasonText(t, model.reason)}` : ""}
              </p>
              <p className="text-zinc-500">{formatBytes(model.bytes)}</p>
              {model.architecture ? (
                <p className="text-zinc-500">
                  {t("models.architecture", { name: model.architecture })}
                </p>
              ) : null}
            </div>
            <div className="flex gap-2">
              {model.catalog && !model.installed && progress?.modelId !== model.id ? (
                <Button
                  aria-label={`${t("models.download")} ${model.name}`}
                  disabled={progress !== null}
                  onClick={() => onDownload(model)}
                >
                  {t("models.download")}
                </Button>
              ) : null}
              {model.installed ? (
                <Button
                  variant="ghost"
                  aria-label={`${t("models.remove")} ${model.name}`}
                  onClick={() => onRemove(model.filename)}
                >
                  {t("models.remove")}
                </Button>
              ) : null}
            </div>
          </div>
          {progress?.modelId === model.id ? <DownloadBar progress={progress} /> : null}
        </li>
      ))}
    </ul>
  );
}

function DownloadBar({ progress }: { progress: DownloadProgress }) {
  const { t } = useTranslation();
  const percent = percentOf(progress);
  const verifying = progress.total > 0 && progress.received >= progress.total;
  const label = verifying
    ? t("models.verifying")
    : progress.total <= 0
      ? t("models.downloading")
      : t("models.progress", {
          received: formatBytes(progress.received),
          total: formatBytes(progress.total),
        });
  return (
    <div className="mt-2">
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        aria-label={t("models.downloading")}
        className="h-1.5 overflow-hidden rounded bg-zinc-200 dark:bg-zinc-800"
      >
        <div className="h-full bg-zinc-900 dark:bg-zinc-100" style={{ width: `${percent}%` }} />
      </div>
      <p className="mt-1 text-zinc-500">{label}</p>
    </div>
  );
}

function percentOf(progress: DownloadProgress): number {
  if (progress.total <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((progress.received / progress.total) * 100));
}

function DownloadDialog({
  model,
  onCancel,
  onConfirm,
}: {
  model: ListedModel;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  const incompatible = model.compatibility === "incompatible";
  const [accepted, setAccepted] = useState(false);
  return (
    <div
      role="dialog"
      aria-labelledby="model-license-title"
      className="flex flex-col gap-3 rounded-md border border-zinc-300 p-4 dark:border-zinc-700"
    >
      <h3 id="model-license-title" className="font-medium">
        {t("models.licenseTitle", { name: model.name })}
      </h3>
      <p>{model.license || t("models.licenseUnknown")}</p>
      {incompatible ? (
        <>
          <p>{t("models.incompatibleWarning")}</p>
          {model.reason ? <p>{reasonText(t, model.reason)}</p> : null}
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={accepted}
              onChange={(event) => setAccepted(event.target.checked)}
            />
            {t("models.understand")}
          </label>
        </>
      ) : null}
      <div className="flex gap-2">
        <Button variant="ghost" onClick={onCancel}>
          {t("models.cancel")}
        </Button>
        <Button disabled={incompatible && !accepted} onClick={onConfirm}>
          {incompatible ? t("models.confirmAnyway") : t("models.confirmDownload")}
        </Button>
      </div>
    </div>
  );
}

function reasonText(
  t: (key: string, values?: Record<string, string>) => string,
  reason: ListedReason,
): string {
  return t(`models.reasons.${reason.code}`, {
    required: formatBytes(reason.requiredBytes ?? 0),
    available: formatBytes(reason.availableBytes ?? 0),
    proven: formatBytes(reason.provenBytes ?? 0),
  });
}

function formatBytes(bytes: number): string {
  const gib = 1024 * 1024 * 1024;
  if (bytes >= gib) {
    return `${(bytes / gib).toFixed(1)} GiB`;
  }
  const mib = 1024 * 1024;
  if (bytes >= mib) {
    return `${Math.round(bytes / mib)} MiB`;
  }
  return `${bytes} B`;
}
