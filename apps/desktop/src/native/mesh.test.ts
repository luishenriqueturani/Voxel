import { describe, expect, it, vi } from "vitest";
import { meshUrl } from "./mesh";

const state = vi.hoisted(() => ({ inside: false }));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => state.inside,
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

describe("meshUrl", () => {
  it("abre um endereço remoto sem o Tauri", () => {
    state.inside = false;
    expect(meshUrl("https://exemplo.test/a.glb")).toBe("https://exemplo.test/a.glb");
    expect(meshUrl("blob:https://exemplo.test/abc")).toBe("blob:https://exemplo.test/abc");
    expect(meshUrl("/home/luis/modelo.glb")).toBeNull();
  });

  it("converte o caminho do disco quando está no aplicativo", () => {
    state.inside = true;
    expect(meshUrl("/home/luis/modelo.glb")).toBe("asset:///home/luis/modelo.glb");
  });
});
