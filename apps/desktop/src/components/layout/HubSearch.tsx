import type { TFunction } from "i18next";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type DownloadProgress,
  downloadHubModel,
  type HubPackage,
  type HubPart,
  searchHubModels,
} from "../../native/models";
import { Button } from "../ui/button";

type HubSearchProps = {
  hfTokenConfigured: boolean;
  progress?: DownloadProgress | null;
  queuedIds?: readonly string[];
  onSearch?: (query: string) => Promise<HubPackage[]>;
  onDownload?: (pkg: HubPackage) => Promise<void> | void;
};

async function downloadPackage(pkg: HubPackage): Promise<void> {
  for (const part of pkg.parts) {
    await downloadHubModel(part, pkg);
  }
}

export function HubSearch({
  hfTokenConfigured,
  progress = null,
  queuedIds = [],
  onSearch = searchHubModels,
  onDownload = downloadPackage,
}: HubSearchProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [packages, setPackages] = useState<HubPackage[]>([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);
  const [failure, setFailure] = useState("");
  const [pending, setPending] = useState<HubPackage | null>(null);

  async function search() {
    const term = query.trim();
    if (term === "") {
      return;
    }
    setSearching(true);
    setFailure("");
    try {
      setPackages(await onSearch(term));
      setSearched(true);
    } catch (error: unknown) {
      setPackages([]);
      setSearched(true);
      setFailure(error instanceof Error ? error.message : String(error));
    } finally {
      setSearching(false);
    }
  }

  async function confirm(pkg: HubPackage) {
    setPending(null);
    setFailure("");
    try {
      await onDownload(pkg);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
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
      <HubStatus packages={packages} progress={progress} failure={failure} />
      <div className="min-h-0 flex-1 overflow-y-auto rounded-md border border-zinc-200 p-2 dark:border-zinc-800">
        {searched && packages.length === 0 && !failure ? (
          <p className="text-zinc-500">{t("models.searchEmpty")}</p>
        ) : null}
        <ul className="flex flex-col gap-3">
          {packages.map((pkg) => (
            <PackageCard
              key={packageKey(pkg)}
              pkg={pkg}
              hfTokenConfigured={hfTokenConfigured}
              progress={progress}
              queuedIds={queuedIds}
              onChoose={() => setPending(pkg)}
            />
          ))}
        </ul>
      </div>
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
              {t("models.licenseTitle", { name: pending.name })}
            </h3>
            <p>{pending.license || t("models.licenseUnknown")}</p>
            <p>{t("models.unmeasuredWarning")}</p>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={() => setPending(null)}>
                {t("models.cancel")}
              </Button>
              <Button onClick={() => confirm(pending)}>{t("models.confirmDownload")}</Button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function PackageCard({
  pkg,
  hfTokenConfigured,
  progress,
  queuedIds,
  onChoose,
}: {
  pkg: HubPackage;
  hfTokenConfigured: boolean;
  progress: DownloadProgress | null;
  queuedIds: readonly string[];
  onChoose: () => void;
}) {
  const { t } = useTranslation();
  const blocked = pkg.gated && !hfTokenConfigured;
  const active = pkg.parts.some((part) => hubId(part) === progress?.modelId);
  const queued = pkg.parts.some((part) => queuedIds.includes(hubId(part)));
  const total = pkg.parts.reduce((sum, part) => sum + part.bytes, 0);
  return (
    <li className="rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1 break-words">
          <p className="font-medium">{pkg.name}</p>
          <p className="text-zinc-500">{pkg.repo}</p>
          {pkg.folder === "" ? null : <p className="text-zinc-500">{pkg.folder}</p>}
          <p className="text-zinc-500">{formatBytes(total)}</p>
          <ul className="mt-2 flex flex-col gap-1">
            {pkg.parts.map((part) => (
              <li key={part.filename}>{partLine(part, t)}</li>
            ))}
          </ul>
          <p className="text-zinc-500">{pkg.license || t("models.licenseUnknown")}</p>
          <p>{t("models.unmeasured")}</p>
          {blocked ? <p>{t("models.gatedToken")}</p> : null}
        </div>
        {active ? null : queued ? (
          <p className="text-zinc-500">{t("models.queued")}</p>
        ) : (
          <Button
            aria-label={`${t("models.download")} ${pkg.name}`}
            disabled={blocked}
            onClick={onChoose}
          >
            {t("models.download")}
          </Button>
        )}
      </div>
      {active && progress ? <DownloadBar progress={progress} /> : null}
    </li>
  );
}

function hubId(part: HubPart): string {
  return `${part.repo}/${part.filename}`;
}

function packageKey(pkg: HubPackage): string {
  return `${pkg.repo}/${pkg.opener}/${pkg.marker}/${pkg.name}`;
}

function partLine(part: HubPart, t: TFunction): string {
  const role = t(`models.roles.${part.role}`);
  const size = formatBytes(part.bytes);
  if (part.shardIndex !== null && part.shardCount !== null) {
    return t("models.partShard", {
      role,
      index: part.shardIndex,
      total: part.shardCount,
      size,
    });
  }
  return t("models.partLine", { role, size });
}

function HubStatus({
  packages,
  progress,
  failure,
}: {
  packages: HubPackage[];
  progress: DownloadProgress | null;
  failure: string;
}) {
  const current =
    packages.find((pkg) => pkg.parts.some((part) => hubId(part) === progress?.modelId)) ?? null;
  if (!failure && !current) {
    return null;
  }
  return (
    <div
      aria-live="polite"
      className="sticky top-0 z-10 flex flex-col gap-1 rounded-md border border-zinc-300 bg-zinc-50 p-3 dark:border-zinc-700 dark:bg-zinc-950"
    >
      {current ? <p className="font-medium">{current.name}</p> : null}
      {failure ? <p className="text-red-700 dark:text-red-300">{failure}</p> : null}
      {current && progress ? <DownloadBar progress={progress} /> : null}
    </div>
  );
}

function DownloadBar({ progress }: { progress: DownloadProgress }) {
  const { t } = useTranslation();
  const percent =
    progress.total <= 0 ? 0 : Math.min(100, Math.round((progress.received / progress.total) * 100));
  const hashing = progress.phase === "verify" && progress.total > 0;
  const verifying = !hashing && progress.total > 0 && progress.received >= progress.total;
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
        {hashing
          ? t("models.verifyingProgress", {
              received: formatBytes(progress.received),
              total: formatBytes(progress.total),
            })
          : verifying
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
