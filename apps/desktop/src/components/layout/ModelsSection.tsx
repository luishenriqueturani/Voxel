import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ListedModel, ListedReason } from "../../native/models";
import { Button } from "../ui/button";

type ModelsSectionProps = {
  hfTokenConfigured: boolean;
  onHfTokenSave: (token: string) => void;
  onHfTokenClear: () => void;
  models: ListedModel[];
  onDownload: (id: string) => void;
  onRemove: (filename: string) => void;
  onOpenFolder: () => void;
  onImport: () => void;
};

export function ModelsSection({
  hfTokenConfigured,
  onHfTokenSave,
  onHfTokenClear,
  models,
  onDownload,
  onRemove,
  onOpenFolder,
  onImport,
}: ModelsSectionProps) {
  const { t } = useTranslation();
  const [tokenDraft, setTokenDraft] = useState("");
  const [showIncompatible, setShowIncompatible] = useState(false);
  const [pending, setPending] = useState<ListedModel | null>(null);
  const visible = models.filter(
    (model) => showIncompatible || model.compatibility !== "incompatible",
  );

  return (
    <div className="flex max-w-xl flex-col gap-4 text-sm">
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
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={showIncompatible}
          onChange={(event) => setShowIncompatible(event.target.checked)}
        />
        {t("models.showIncompatible")}
      </label>
      <ul className="flex flex-col gap-3">
        {visible.map((model) => (
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
                {model.catalog && !model.installed ? (
                  <Button
                    aria-label={`${t("models.download")} ${model.name}`}
                    onClick={() => setPending(model)}
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
          </li>
        ))}
      </ul>
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
  );
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
