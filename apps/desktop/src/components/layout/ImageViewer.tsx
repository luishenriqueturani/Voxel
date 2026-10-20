import { ChevronLeft, ChevronRight } from "lucide-react";
import { type PointerEvent as ReactPointerEvent, useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";
import { MosaicPreview } from "./MosaicPreview";

const MIN_ZOOM = 1;
const MAX_ZOOM = 8;

type ImageViewerProps = {
  src: string | null;
  step: number;
  total: number;
  canSave: boolean;
  canRegenerate: boolean;
  canRemove?: boolean;
  removing?: boolean;
  transparent?: boolean;
  index?: number;
  count?: number;
  onPrevious?: () => void;
  onNext?: () => void;
  onSave: () => void;
  onCopy: () => void;
  onRegenerate: () => void;
  onRemoveBackground?: () => void;
  onSendTo3d?: () => void;
  canUndo?: boolean;
  onUndo?: () => void;
  mosaic?: boolean;
  onToggleMosaic?: () => void;
};

type Pan = { x: number; y: number };

function zoomByWheel(current: number, deltaY: number): number {
  const scaled = current * (deltaY < 0 ? 1.1 : 1 / 1.1);
  const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, scaled));
  return clamped < 1.02 ? MIN_ZOOM : clamped;
}

export function ImageViewer({
  src,
  step,
  total,
  canSave,
  canRegenerate,
  canRemove = false,
  removing = false,
  transparent = false,
  index = 0,
  count = 0,
  onPrevious,
  onNext,
  onSave,
  onCopy,
  onRegenerate,
  onRemoveBackground,
  onSendTo3d,
  canUndo = false,
  onUndo,
  mosaic = false,
  onToggleMosaic,
}: ImageViewerProps) {
  const { t } = useTranslation();
  const stageRef = useRef<HTMLDivElement>(null);
  const zoomRef = useRef(MIN_ZOOM);
  const panRef = useRef<Pan>({ x: 0, y: 0 });
  const dragRef = useRef<Pan | null>(null);
  const [zoom, setZoom] = useState(MIN_ZOOM);
  const [pan, setPan] = useState<Pan>({ x: 0, y: 0 });
  const [panning, setPanning] = useState(false);
  const [shownSrc, setShownSrc] = useState(src);
  const percent = Math.round(zoom * 100);
  zoomRef.current = zoom;
  panRef.current = pan;

  if (src !== shownSrc) {
    setShownSrc(src);
    setZoom(MIN_ZOOM);
    setPan({ x: 0, y: 0 });
  }

  const onWheel = useCallback((event: WheelEvent) => {
    const stage = stageRef.current;
    if (!stage || !event.ctrlKey || event.deltaY === 0) {
      return;
    }
    event.preventDefault();
    const rect = stage.getBoundingClientRect();
    const dx = event.clientX - rect.left - rect.width / 2;
    const dy = event.clientY - rect.top - rect.height / 2;
    const current = zoomRef.current;
    const next = zoomByWheel(current, event.deltaY);
    const factor = next / current;
    const origin = panRef.current;
    setZoom(next);
    setPan(
      next === MIN_ZOOM
        ? { x: 0, y: 0 }
        : {
            x: dx * (1 - factor) + origin.x * factor,
            y: dy * (1 - factor) + origin.y * factor,
          },
    );
  }, []);

  const setStage = useCallback(
    (node: HTMLDivElement | null) => {
      stageRef.current?.removeEventListener("wheel", onWheel);
      stageRef.current = node;
      node?.addEventListener("wheel", onWheel, { passive: false });
    },
    [onWheel],
  );

  function onPointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 1) {
      return;
    }
    event.preventDefault();
    dragRef.current = { x: event.clientX, y: event.clientY };
    event.currentTarget.setPointerCapture?.(event.pointerId);
    setPanning(true);
  }

  function onPointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    if (!drag) {
      return;
    }
    const dx = event.clientX - drag.x;
    const dy = event.clientY - drag.y;
    dragRef.current = { x: event.clientX, y: event.clientY };
    setPan((current) => ({ x: current.x + dx, y: current.y + dy }));
  }

  function endPan() {
    dragRef.current = null;
    setPanning(false);
  }

  return (
    <div className="flex h-full w-full min-h-0 gap-4">
      <div className={`flex min-h-0 min-w-0 flex-1 ${src ? "" : "items-center justify-center"}`}>
        {src && mosaic ? (
          <MosaicPreview src={src} />
        ) : src ? (
          <div
            ref={setStage}
            data-testid="viewer-stage"
            role="application"
            aria-label={t("generation.preview")}
            className={`relative h-full min-h-0 w-full overflow-hidden ${transparent ? "transparency-grid" : ""} ${panning ? "cursor-grabbing" : "cursor-grab"}`}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={endPan}
            onPointerCancel={endPan}
            onMouseDown={(event) => {
              if (event.button === 1) {
                event.preventDefault();
              }
            }}
          >
            <img
              alt={t("generation.preview")}
              draggable={false}
              className="pointer-events-none absolute inset-0 h-full w-full object-contain select-none"
              style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
              src={src}
            />
          </div>
        ) : (
          <p className="text-sm text-zinc-500">{t("main.empty")}</p>
        )}
      </div>
      {src ? (
        <aside className="flex w-52 shrink-0 flex-col gap-2 overflow-y-auto">
          {count > 1 ? (
            <div className="flex items-center justify-between gap-1">
              <Button
                variant="ghost"
                className="px-2"
                aria-label={t("viewer.previous")}
                disabled={index <= 0}
                onClick={onPrevious}
              >
                <ChevronLeft aria-hidden="true" size={16} />
              </Button>
              <span className="text-sm text-zinc-500">
                {t("viewer.position", { current: index + 1, total: count })}
              </span>
              <Button
                variant="ghost"
                className="px-2"
                aria-label={t("viewer.next")}
                disabled={index >= count - 1}
                onClick={onNext}
              >
                <ChevronRight aria-hidden="true" size={16} />
              </Button>
            </div>
          ) : null}
          {total > 0 ? (
            <p className="text-sm text-zinc-500">{t("generation.step", { step, total })}</p>
          ) : null}
          <span className="text-sm text-zinc-500">{t("viewer.zoom", { value: percent })}</span>
          <p className="text-sm text-zinc-500">{t("viewer.gesture")}</p>
          {onToggleMosaic ? (
            <Button
              variant="ghost"
              className="w-full"
              aria-pressed={mosaic}
              onClick={onToggleMosaic}
            >
              {t("viewer.mosaic")}
            </Button>
          ) : null}
          {onUndo ? (
            <Button variant="ghost" className="w-full" disabled={!canUndo} onClick={onUndo}>
              {t("viewer.undo")}
            </Button>
          ) : null}
          <Button
            variant="ghost"
            className="w-full whitespace-normal"
            disabled={!canRemove || removing}
            onClick={onRemoveBackground}
          >
            {removing ? t("viewer.removing") : t("viewer.removeBackground")}
          </Button>
          {onSendTo3d ? (
            <Button variant="ghost" className="w-full whitespace-normal" onClick={onSendTo3d}>
              {t("viewer.sendTo3d")}
            </Button>
          ) : null}
          <Button variant="ghost" className="w-full" disabled={!canSave} onClick={onSave}>
            {t("viewer.save")}
          </Button>
          <Button variant="ghost" className="w-full" disabled={src === null} onClick={onCopy}>
            {t("viewer.copy")}
          </Button>
          <Button
            variant="ghost"
            className="w-full whitespace-normal"
            disabled={!canRegenerate}
            onClick={onRegenerate}
          >
            {t("viewer.regenerate")}
          </Button>
        </aside>
      ) : null}
    </div>
  );
}
