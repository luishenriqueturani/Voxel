import { describe, expect, it } from "vitest";
import { progressFrom } from "./models";

describe("progressFrom", () => {
  it("aceita o evento de download", () => {
    expect(progressFrom({ modelId: "sd15", received: 10, total: 40 })).toEqual({
      modelId: "sd15",
      received: 10,
      total: 40,
    });
  });

  it("ignora um evento incompleto", () => {
    expect(progressFrom({ modelId: "sd15" })).toBeNull();
    expect(progressFrom(null)).toBeNull();
  });
});
