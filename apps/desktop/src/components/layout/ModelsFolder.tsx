import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { modelsDirectory, pickModelsDirectory, setModelsDirectory } from "../../native/models";
import { Button } from "../ui/button";

type ModelsFolderProps = {
  onChanged?: () => void;
  onError: (error: unknown) => void;
};

export function ModelsFolder({ onChanged, onError }: ModelsFolderProps) {
  const { t } = useTranslation();
  const [path, setPath] = useState("");
  const [pending, setPending] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void modelsDirectory()
      .then((loaded) => {
        if (!cancelled && loaded !== null) {
          setPath(loaded);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          onError(error);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [onError]);

  async function apply(destination: string, moveFiles: boolean) {
    setBusy(true);
    try {
      const next = await setModelsDirectory(destination, moveFiles);
      if (next !== null) {
        setPath(next);
        onChanged?.();
      }
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
      setPending(null);
    }
  }

  return (
    <div className="flex max-w-xl flex-col gap-2">
      <p className="text-sm font-medium">{t("models.folder")}</p>
      <p className="break-all text-sm text-zinc-500">
        {path === "" ? t("models.folderDefault") : path}
      </p>
      <p className="text-sm text-zinc-500">{t("models.folderHint")}</p>
      <Button
        variant="ghost"
        className="self-start"
        disabled={busy}
        onClick={() => {
          void pickModelsDirectory(t("models.chooseFolder")).then((chosen) => {
            if (chosen === null || chosen === path) {
              return;
            }
            setPending(chosen);
          });
        }}
      >
        {t("models.chooseFolder")}
      </Button>
      {pending === null ? null : (
        <div className="flex flex-col gap-2 rounded-md border border-zinc-300 p-3 dark:border-zinc-700">
          <p>{t("models.moveLead", { path: pending })}</p>
          <div className="flex flex-wrap gap-2">
            <Button disabled={busy} onClick={() => void apply(pending, true)}>
              {t("models.moveYes")}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => void apply(pending, false)}>
              {t("models.moveNo")}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => setPending(null)}>
              {t("models.cancel")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
