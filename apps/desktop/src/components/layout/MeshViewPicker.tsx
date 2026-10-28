import { ChevronDown, ChevronUp } from "lucide-react";
import { type DragEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { imagePaths } from "../../lib/mesh";
import { type MeshView, moveView, viewName, viewSrc } from "../../lib/meshViews";
import { Button } from "../ui/button";

type MeshViewPickerProps = {
  views: MeshView[];
  primaryPath: string;
  canAddCurrent?: boolean;
  onAddPaths?: (paths: string[]) => void;
  onAddFromDisk?: () => void;
  onAddCurrent?: () => void;
  onConfirm: (views: MeshView[], primaryPath: string) => void;
  onCancel: () => void;
};

function ignorePaths(_paths: string[]) {
  return undefined;
}

function ignoreClick() {
  return undefined;
}

export function MeshViewPicker({
  views,
  primaryPath,
  canAddCurrent = false,
  onAddPaths = ignorePaths,
  onAddFromDisk = ignoreClick,
  onAddCurrent = ignoreClick,
  onConfirm,
  onCancel,
}: MeshViewPickerProps) {
  const { t } = useTranslation();
  const [order, setOrder] = useState(views);
  const [included, setIncluded] = useState(views.map((view) => view.path));
  const [primary, setPrimary] = useState(primaryPath);
  const known = useRef(new Set(views.map((view) => view.path)));

  useEffect(() => {
    const fresh = views.filter((view) => !known.current.has(view.path));
    if (fresh.length === 0) {
      return;
    }
    for (const view of fresh) {
      known.current.add(view.path);
    }
    setOrder((current) => [...current, ...fresh]);
    setIncluded((current) => [...current, ...fresh.map((view) => view.path)]);
  }, [views]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onCancel();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  function toggle(path: string) {
    const next = included.includes(path)
      ? included.filter((item) => item !== path)
      : [...included, path];
    setIncluded(next);
    if (!next.includes(primary)) {
      setPrimary(order.find((view) => next.includes(view.path))?.path ?? primary);
    }
  }

  function confirm() {
    const chosen = order.filter((view) => included.includes(view.path));
    const main = chosen.find((view) => view.path === primary);
    if (main === undefined) {
      return;
    }
    onConfirm(chosen, main.path);
  }

  const canConfirm = included.includes(primary);

  function dropImages(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    const images = imagePaths(event.dataTransfer.getData("text/plain").split(/\s+/));
    if (images.length > 0) {
      onAddPaths(images);
    }
  }

  return (
    <div className="absolute inset-0 z-20 flex items-center justify-center bg-zinc-950/40 p-6">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="mesh-pick-title"
        className="flex max-h-full w-full max-w-lg flex-col gap-3 overflow-hidden rounded-lg border border-zinc-300 bg-white p-4 shadow-lg dark:border-zinc-700 dark:bg-zinc-900"
        onDragOver={(event) => {
          event.preventDefault();
        }}
        onDrop={dropImages}
      >
        <div className="flex flex-col gap-1">
          <h2 id="mesh-pick-title" className="text-base font-medium">
            {t("meshPick.title")}
          </h2>
          <p className="text-sm text-zinc-500">{t("meshPick.hint")}</p>
          <p className="text-sm text-zinc-500">{t("meshPick.limit")}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="ghost" onClick={onAddFromDisk}>
            {t("meshPick.fromDisk")}
          </Button>
          <Button variant="ghost" disabled={!canAddCurrent} onClick={onAddCurrent}>
            {t("meshPick.fromGeneration")}
          </Button>
        </div>
        <ul className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
          {order.map((view, index) => {
            const name = viewName(view.path);
            const src = viewSrc(view.src);
            return (
              <li
                key={view.path}
                className="flex items-center gap-2 rounded-md border border-zinc-200 p-2 dark:border-zinc-800"
              >
                <input
                  type="checkbox"
                  aria-label={t("meshPick.include", { name })}
                  checked={included.includes(view.path)}
                  onChange={() => toggle(view.path)}
                />
                {src ? (
                  <img alt="" src={src} className="h-12 w-12 shrink-0 rounded object-cover" />
                ) : (
                  <span className="h-12 w-12 shrink-0 rounded bg-zinc-200 dark:bg-zinc-800" />
                )}
                <span className="min-w-0 flex-1 truncate text-sm">{name}</span>
                <input
                  type="radio"
                  name="mesh-primary"
                  aria-label={t("meshPick.primary", { name })}
                  checked={primary === view.path}
                  disabled={!included.includes(view.path)}
                  onChange={() => setPrimary(view.path)}
                />
                <Button
                  variant="ghost"
                  className="px-2"
                  aria-label={t("meshPick.up", { name })}
                  disabled={index === 0}
                  onClick={() => setOrder((current) => moveView(current, index, -1))}
                >
                  <ChevronUp aria-hidden="true" size={16} />
                </Button>
                <Button
                  variant="ghost"
                  className="px-2"
                  aria-label={t("meshPick.down", { name })}
                  disabled={index === order.length - 1}
                  onClick={() => setOrder((current) => moveView(current, index, 1))}
                >
                  <ChevronDown aria-hidden="true" size={16} />
                </Button>
              </li>
            );
          })}
        </ul>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel}>
            {t("meshPick.cancel")}
          </Button>
          <Button disabled={!canConfirm} onClick={confirm}>
            {t("meshPick.confirm")}
          </Button>
        </div>
      </div>
    </div>
  );
}
