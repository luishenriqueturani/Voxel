import { describe, expect, it } from "vitest";
import { environmentLabel } from "./pythonStatus";

const t = (key: string, options?: { detail: string }) =>
  options ? `${key}:${options.detail}` : key;

const empty = {
  phase: "absent",
  percent: null,
  receivedBytes: null,
  totalBytes: null,
  backend: null,
  detail: null,
};

describe("rótulo do ambiente 3D", () => {
  it("mostra o percentual e o tamanho enquanto baixa", () => {
    expect(
      environmentLabel(
        {
          ...empty,
          phase: "preparing",
          percent: 45,
          receivedBytes: 1.2 * 1024 ** 3,
          totalBytes: 2.6 * 1024 ** 3,
        },
        t,
      ),
    ).toBe("status.envPreparing:45% · 1.2/2.6 GiB");
  });

  it("distingue ausente, teste, pronto e falha", () => {
    expect(environmentLabel(undefined, t)).toBe("status.unknown");
    expect(environmentLabel(empty, t)).toBe("status.envAbsent");
    expect(environmentLabel({ ...empty, phase: "testing" }, t)).toBe("status.envTesting");
    expect(environmentLabel({ ...empty, phase: "ready" }, t)).toBe("status.envReady");
    expect(environmentLabel({ ...empty, phase: "failed" }, t)).toBe("status.envFailed");
    expect(environmentLabel({ ...empty, phase: "preparing" }, t)).toBe("status.envPreparingPlain");
  });
});
