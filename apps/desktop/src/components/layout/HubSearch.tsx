import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  claimDownload,
  type DownloadProgress,
  downloadHubModel,
  type HubFile,
  releaseDownload,
  searchHubModels,
} from "../../native/models";
import { Button } from "../ui/button";

type HubSearchProps = {
  hfTokenConfigured: boolean;
  progress?: DownloadProgress | null;
  onSearch?: (query: string) => Promise<HubFile[]>;
  onDownload?: (file: HubFile) => Promise<void> | void;
};

export function HubSearch({
  hfTokenConfigured,
  progress = null,
  onSearch = searchHubModels,
  onDownload = downloadHubModel,
}: HubSearchProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [files, setFiles] = useState<HubFile[]>([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);
  const [failure, setFailure] = useState("");
  const [pending, setPending] = useState<HubFile | null>(null);

  async function search() {
    const term = query.trim();
    if (term === "") {
      return;
    }
    setSearching(true);
    setFailure("");
    try {
      setFiles(await onSearch(term));
      setSearched(true);
    } catch (error: unknown) {
      setFiles([]);
      setSearched(true);
      setFailure(error instanceof Error ? error.message : String(error));
    } finally {
      setSearching(false);
    }
  }

  async function confirm(file: HubFile) {
    const id = hubId(file);
    if (!claimDownload(id)) {
      return;
    }
    setPending(null);
    setFailure("");
    try {
      await onDownload(file);
      window.dispatchEvent(new Event("voxel-models"));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    } finally {
      releaseDownload(id);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <form
        className="flex items-end gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          void search();
        }}
      >
        <label className="flex flex-1 flex-col gap-1">
          {t("models.search")}
          <input
            value={query}
            placeholder={t("models.searchPlaceholder")}
            onChange={(event) => setQuery(event.target.value)}
            className="rounded-md border border-zinc-300 bg-white px-2 py-1.5 dark:border-zinc-700 dark:bg-zinc-950"
          />
        </label>
        <Button type="submit" disabled={searching || query.trim() === ""}>
          {searching ? t("models.searching") : t("models.search")}
        </Button>
      </form>
      {searched && files.length === 0 && !failure ? (
        <p className="text-zinc-500">{t("models.searchEmpty")}</p>
      ) : null}
      <HubStatus files={files} progress={progress} failure={failure} />
      <ul className="flex flex-col gap-3">
        {files.map((file) => {
          const id = hubId(file);
          const blocked = file.gated && !hfTokenConfigured;
          return (
            <li key={id} className="rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="font-medium">{file.filename}</p>
                  <p className="text-zinc-500">{file.repo}</p>
                  <p className="text-zinc-500">{formatBytes(file.bytes)}</p>
                  <p className="text-zinc-500">{file.license || t("models.licenseUnknown")}</p>
                  <p>{t("models.unmeasured")}</p>
                  {blocked ? <p>{t("models.gatedToken")}</p> : null}
                </div>
                {progress?.modelId === id ? null : (
                  <Button
                    aria-label={`${t("models.download")} ${file.filename}`}
                    disabled={progress !== null || blocked}
                    onClick={() => setPending(file)}
                  >
                    {t("models.download")}
                  </Button>
                )}
              </div>
              {progress?.modelId === id ? <DownloadBar progress={progress} /> : null}
            </li>
          );
        })}
      </ul>
      {pending ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
          <button
            type="button"
            aria-label={t("models.cancel")}
            className="absolute inset-0 bg-black/50"
            onClick={() => setPending(null)}
          />
          <div
            role="dialog"
            aria-labelledby="hub-license-title"
            className="relative z-10 flex w-full max-w-md flex-col gap-3 rounded-md border border-zinc-300 bg-white p-4 dark:border-zinc-700 dark:bg-zinc-950"
          >
            <h3 id="hub-license-title" className="font-medium">
              {t("models.licenseTitle", { name: pending.filename })}
            </h3>
            <p>{pending.license || t("models.licenseUnknown")}</p>
            <p>{t("models.unmeasuredWarning")}</p>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={() => setPending(null)}>
                {t("models.cancel")}
              </Button>
              <Button disabled={progress !== null} onClick={() => confirm(pending)}>
                {t("models.confirmDownload")}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function hubId(file: HubFile): string {
  return `${file.repo}/${file.filename}`;
}

function HubStatus({
  files,
  progress,
  failure,
}: {
  files: HubFile[];
  progress: DownloadProgress | null;
  failure: string;
}) {
  const current = files.find((file) => hubId(file) === progress?.modelId) ?? null;
  if (!failure && !current) {
    return null;
  }
  return (
    <div
      aria-live="polite"
      className="sticky top-0 z-10 flex flex-col gap-1 rounded-md border border-zinc-300 bg-zinc-50 p-3 dark:border-zinc-700 dark:bg-zinc-950"
    >
      {current ? <p className="font-medium">{current.filename}</p> : null}
      {failure ? <p className="text-red-700 dark:text-red-300">{failure}</p> : null}
      {current && progress ? <DownloadBar progress={progress} /> : null}
    </div>
  );
}

function DownloadBar({ progress }: { progress: DownloadProgress }) {
  const { t } = useTranslation();
  const percent =
    progress.total <= 0 ? 0 : Math.min(100, Math.round((progress.received / progress.total) * 100));
  const verifying = progress.total > 0 && progress.received >= progress.total;
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
      <p className="mt-1 text-zinc-500">
        {verifying
          ? t("models.verifying")
          : progress.total <= 0
            ? t("models.downloading")
            : t("models.progress", {
                received: formatBytes(progress.received),
                total: formatBytes(progress.total),
              })}
      </p>
    </div>
  );
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
