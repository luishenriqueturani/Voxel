import { type PointerEvent as ReactPointerEvent, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { maskFromPaint, paintDisk, paintHasInk, strokePoints } from "../../lib/mask";
import { Button } from "../ui/button";

type MaskEditorProps = {
  src: string;
  onMask: (png: string | null) => void;
  canRevert?: boolean;
  onRevert?: () => void;
};

type Snapshot = Uint8ClampedArray;

function ownedPixels(pixels: Uint8ClampedArray): Snapshot {
  const copy = new Uint8ClampedArray(pixels.length);
  copy.set(pixels);
  return copy;
}

function imageFromPixels(pixels: Uint8ClampedArray, width: number, height: number): ImageData {
  const image = new ImageData(width, height);
  image.data.set(pixels);
  return image;
}

function ignoreRevert() {
  return undefined;
}

export function MaskEditor({
  src,
  onMask,
  canRevert = false,
  onRevert = ignoreRevert,
}: MaskEditorProps) {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const undoRef = useRef<Snapshot[]>([]);
  const sizedSrc = useRef<string | null>(null);
  const lastRef = useRef<{ x: number; y: number } | null>(null);
  const [brush, setBrush] = useState(28);
  const [eraser, setEraser] = useState(false);
  const [canUndo, setCanUndo] = useState(false);

  function context() {
    const canvas = canvasRef.current;
    return canvas === null ? null : canvas.getContext("2d");
  }

  function remember() {
    const canvas = canvasRef.current;
    const ctx = context();
    if (canvas === null || ctx === null) {
      return;
    }
    const current = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    undoRef.current = [...undoRef.current.slice(-23), ownedPixels(current)];
    setCanUndo(true);
  }

  function publish() {
    const canvas = canvasRef.current;
    const ctx = context();
    if (canvas === null || ctx === null) {
      return;
    }
    const paint = ctx.getImageData(0, 0, canvas.width, canvas.height);
    if (!paintHasInk(paint.data)) {
      onMask(null);
      return;
    }
    const mask = document.createElement("canvas");
    mask.width = canvas.width;
    mask.height = canvas.height;
    const maskContext = mask.getContext("2d");
    if (maskContext === null) {
      return;
    }
    const bytes = ownedPixels(maskFromPaint(canvas.width, canvas.height, paint.data));
    maskContext.putImageData(imageFromPixels(bytes, canvas.width, canvas.height), 0, 0);
    onMask(mask.toDataURL("image/png"));
  }

  function stamp(event: ReactPointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    const ctx = context();
    if (canvas === null || ctx === null) {
      return;
    }
    const rect = canvas.getBoundingClientRect();
    const point = {
      x: ((event.clientX - rect.left) / rect.width) * canvas.width,
      y: ((event.clientY - rect.top) / rect.height) * canvas.height,
    };
    const paint = ctx.getImageData(0, 0, canvas.width, canvas.height);
    for (const step of strokePoints(lastRef.current, point, Math.max(1, brush / 3))) {
      paintDisk(canvas.width, canvas.height, paint.data, step, brush / 2, eraser);
    }
    ctx.putImageData(paint, 0, 0);
    lastRef.current = point;
  }

  function onPointerDown(event: ReactPointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (event.button !== 0 || canvas === null || canvas.width === 0 || canvas.height === 0) {
      return;
    }
    remember();
    lastRef.current = null;
    stamp(event);
    event.currentTarget.setPointerCapture?.(event.pointerId);
  }

  function onPointerUp(event: ReactPointerEvent<HTMLCanvasElement>) {
    if (lastRef.current === null) {
      return;
    }
    lastRef.current = null;
    if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    publish();
  }

  function undo() {
    const canvas = canvasRef.current;
    const ctx = context();
    const previous = undoRef.current.pop();
    if (canvas === null || ctx === null || previous === undefined) {
      setCanUndo(false);
      return;
    }
    ctx.putImageData(imageFromPixels(previous, canvas.width, canvas.height), 0, 0);
    setCanUndo(undoRef.current.length > 0);
    publish();
  }

  function clear() {
    const canvas = canvasRef.current;
    const ctx = context();
    if (canvas === null || ctx === null || canvas.width === 0 || canvas.height === 0) {
      onMask(null);
      return;
    }
    const paint = ctx.getImageData(0, 0, canvas.width, canvas.height);
    if (!paintHasInk(paint.data)) {
      onMask(null);
      return;
    }
    remember();
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    onMask(null);
  }

  function undoOrRevert() {
    if (undoRef.current.length > 0) {
      undo();
      return;
    }
    onRevert();
  }

  return (
    <div className="flex h-full w-full min-h-0 gap-4">
      <div className="flex min-h-0 min-w-0 flex-1 items-center justify-center">
        <div className="relative max-h-full max-w-full">
          <img
            alt={t("mask.label")}
            src={src}
            className="block max-h-[70vh] max-w-full object-contain"
            onLoad={(event) => {
              const canvas = canvasRef.current;
              if (canvas === null) {
                return;
              }
              const width = event.currentTarget.naturalWidth || event.currentTarget.width;
              const height = event.currentTarget.naturalHeight || event.currentTarget.height;
              if (width === 0 || height === 0) {
                return;
              }
              if (sizedSrc.current === src && canvas.width === width && canvas.height === height) {
                return;
              }
              sizedSrc.current = src;
              canvas.width = width;
              canvas.height = height;
              undoRef.current = [];
              setCanUndo(false);
              onMask(null);
            }}
          />
          <canvas
            ref={canvasRef}
            aria-label={t("mask.label")}
            className="absolute inset-0 h-full w-full cursor-crosshair touch-none"
            onPointerDown={onPointerDown}
            onPointerMove={(event) => {
              if (lastRef.current !== null) {
                stamp(event);
              }
            }}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
          />
        </div>
      </div>
      <aside className="flex w-52 shrink-0 flex-col gap-2">
        <label className="flex flex-col gap-1 text-sm">
          {t("mask.brush")}
          <input
            aria-label={t("mask.brush")}
            type="range"
            min={4}
            max={96}
            step={2}
            value={brush}
            onChange={(event) => setBrush(Number(event.target.value))}
          />
        </label>
        <Button
          variant="ghost"
          className="w-full"
          aria-pressed={eraser}
          onClick={() => setEraser((current) => !current)}
        >
          {eraser ? t("mask.paint") : t("mask.eraser")}
        </Button>
        <Button
          variant="ghost"
          className="w-full"
          disabled={!canUndo && !canRevert}
          onClick={undoOrRevert}
        >
          {t("mask.undo")}
        </Button>
        <Button variant="ghost" className="w-full" onClick={clear}>
          {t("mask.clear")}
        </Button>
      </aside>
    </div>
  );
}
