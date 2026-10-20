import { describe, expect, it } from "vitest";
import { paramsFromHistory } from "./historyParams";

describe("paramsFromHistory", () => {
  it("recupera a proporção que cabe no quadro gravado", () => {
    expect(
      paramsFromHistory({ width: 1024, height: 576, steps: 20, cfg: 8, seed: 4 }, 2).aspect,
    ).toBe("16:9");
    expect(paramsFromHistory({ width: 512, height: 320, steps: 25, cfg: 7, seed: 1 }, 1)).toEqual(
      expect.objectContaining({ aspect: "3:2", resolution: 512, seed: "1", count: 1 }),
    );
    expect(
      paramsFromHistory({ width: 448, height: 1024, steps: 25, cfg: 7, seed: 9 }, 1).aspect,
    ).toBe("9:21");
  });
});
