export type Point = { x: number; y: number };

export function strokePoints(from: Point | null, to: Point, spacing: number): Point[] {
  if (from === null) {
    return [to];
  }
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const distance = Math.hypot(dx, dy);
  const step = Math.max(1, spacing);
  const count = Math.max(1, Math.ceil(distance / step));
  const points: Point[] = [];
  for (let index = 1; index <= count; index += 1) {
    const t = index / count;
    points.push({ x: from.x + dx * t, y: from.y + dy * t });
  }
  return points;
}

export function paintDisk(
  width: number,
  height: number,
  pixels: Uint8ClampedArray,
  point: Point,
  radius: number,
  erase: boolean,
): void {
  const reach = Math.max(1, radius);
  const reachSquared = reach * reach;
  const left = Math.max(0, Math.floor(point.x - reach));
  const top = Math.max(0, Math.floor(point.y - reach));
  const right = Math.min(width - 1, Math.ceil(point.x + reach));
  const bottom = Math.min(height - 1, Math.ceil(point.y + reach));
  for (let y = top; y <= bottom; y += 1) {
    for (let x = left; x <= right; x += 1) {
      const dx = x - point.x;
      const dy = y - point.y;
      if (dx * dx + dy * dy > reachSquared) {
        continue;
      }
      const offset = (y * width + x) * 4;
      if (erase) {
        pixels[offset + 3] = 0;
        continue;
      }
      pixels[offset] = 255;
      pixels[offset + 1] = 40;
      pixels[offset + 2] = 40;
      pixels[offset + 3] = 140;
    }
  }
}

export function paintHasInk(pixels: Uint8ClampedArray): boolean {
  for (let index = 3; index < pixels.length; index += 4) {
    if ((pixels[index] ?? 0) > 0) {
      return true;
    }
  }
  return false;
}

export function maskFromPaint(
  width: number,
  height: number,
  paint: Uint8ClampedArray,
): Uint8ClampedArray {
  const mask = new Uint8ClampedArray(width * height * 4);
  const last = Math.min(paint.length, mask.length);
  for (let index = 0; index < last; index += 4) {
    const ink = (paint[index + 3] ?? 0) > 0;
    const value = ink ? 255 : 0;
    mask[index] = value;
    mask[index + 1] = value;
    mask[index + 2] = value;
    mask[index + 3] = 255;
  }
  return mask;
}
