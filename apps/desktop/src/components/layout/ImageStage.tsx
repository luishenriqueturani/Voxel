import {
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

const MIN_ZOOM = 1;
const MAX_ZOOM = 8;

type Pan = { x: number; y: number };

type ImageStageProps = {
  src: string;
  alt: string;
  transparent?: boolean;
  compareSrc?: string | null;
  hint?: boolean;
  onPercent?: (percent: number) => void;
  children?: ReactNode;
};

export function zoomByWheel(current: number, deltaY: number): number {
  const scaled = current * (deltaY < 0 ? 1.1 : 1 / 1.1);
  const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, scaled));
  return clamped < 1.02 ? MIN_ZOOM : clamped;
}

export function ImageStage({
  src,
  alt,
  transparent = false,
  compareSrc = null,
  hint = false,
  onPercent,
  children,
}: ImageStageProps) {
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
  const transform = `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`;
  zoomRef.current = zoom;
  panRef.current = pan;

  if (src !== shownSrc) {
    setShownSrc(src);
    setZoom(MIN_ZOOM);
    setPan({ x: 0, y: 0 });
  }

  useEffect(() => {
    onPercent?.(percent);
  }, [onPercent, percent]);

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
    <div className="flex h-full min-h-0 w-full flex-col">
      <div
        ref={setStage}
        data-testid="viewer-stage"
        role="application"
        aria-label={t("generation.preview")}
        className={`relative min-h-0 w-full flex-1 overflow-hidden ${transparent ? "transparency-grid" : ""} ${panning ? "cursor-grabbing" : "cursor-grab"}`}
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
        {compareSrc ? (
          <div className="absolute inset-0 grid grid-cols-2">
            <figure className="relative min-h-0 min-w-0">
              <figcaption className="absolute top-2 left-2 z-10 rounded bg-black/60 px-2 py-1 text-xs text-white">
                {t("viewer.comparePrevious")}
              </figcaption>
              <img
                alt={t("viewer.comparePrevious")}
                draggable={false}
                className="pointer-events-none h-full w-full object-contain select-none"
                style={{ transform }}
                src={compareSrc}
              />
            </figure>
            <figure className="relative min-h-0 min-w-0 border-l border-zinc-700">
              <figcaption className="absolute top-2 left-2 z-10 rounded bg-black/60 px-2 py-1 text-xs text-white">
                {t("viewer.compareCurrent")}
              </figcaption>
              <img
                alt={t("viewer.compareCurrent")}
                draggable={false}
                className="pointer-events-none h-full w-full object-contain select-none"
                style={{ transform }}
                src={src}
              />
            </figure>
          </div>
        ) : (
          <img
            alt={alt}
            draggable={false}
            className="pointer-events-none absolute inset-0 h-full w-full object-contain select-none"
            style={{ transform }}
            src={src}
          />
        )}
        {children}
      </div>
      {hint ? (
        <p className="shrink-0 pt-2 text-xs text-zinc-500">
          {t("viewer.zoom", { value: percent })} · {t("viewer.gesture")}
        </p>
      ) : null}
    </div>
  );
}
