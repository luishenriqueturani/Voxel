import { describe, expect, it } from "vitest";
import { maskFromPaint, paintDisk, paintHasInk, strokePoints } from "./mask";

describe("máscara", () => {
  it("pinta um disco, apaga com a borracha e vira branco na máscara", () => {
    const pixels = new Uint8ClampedArray(8 * 8 * 4);
    paintDisk(8, 8, pixels, { x: 3, y: 3 }, 2, false);
    expect(paintHasInk(pixels)).toBe(true);
    const mask = maskFromPaint(8, 8, pixels);
    expect(mask[0]).toBe(0);
    expect(mask[3]).toBe(255);
    const center = (3 * 8 + 3) * 4;
    expect(mask[center]).toBe(255);

    paintDisk(8, 8, pixels, { x: 3, y: 3 }, 4, true);
    expect(paintHasInk(pixels)).toBe(false);
    expect(maskFromPaint(8, 8, pixels)[center]).toBe(0);
  });

  it("liga os pontos do traço para o pincel não pular", () => {
    const points = strokePoints({ x: 0, y: 0 }, { x: 10, y: 0 }, 4);
    expect(points.length).toBeGreaterThan(1);
    expect(points.at(-1)).toEqual({ x: 10, y: 0 });
    expect(strokePoints(null, { x: 1, y: 1 }, 4)).toEqual([{ x: 1, y: 1 }]);
  });
});
