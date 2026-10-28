import { describe, expect, it } from "vitest";
import { mergeViews, meshViewsOf, moveView, pickForPath } from "./meshViews";

const views = [
  { path: "/tmp/frente.png", src: "data:a", seed: 1 },
  { path: "/tmp/lado.png", src: "data:b", seed: 2 },
];

describe("pickForPath", () => {
  it("mantém as vistas conhecidas e marca a pedida", () => {
    expect(pickForPath(views, "/tmp/lado.png")).toEqual({
      token: 0,
      views,
      primaryPath: "/tmp/lado.png",
    });
  });

  it("abre uma vista nova quando o caminho não está na lista", () => {
    expect(pickForPath([], "/tmp/c.png")).toEqual({
      token: 0,
      views: [{ path: "/tmp/c.png", src: "", seed: null }],
      primaryPath: "/tmp/c.png",
    });
  });
});

describe("moveView", () => {
  it("troca com o vizinho e ignora a borda", () => {
    expect(moveView(["a", "b", "c"], 2, -1)).toEqual(["a", "c", "b"]);
    expect(moveView(["a", "b"], 0, -1)).toEqual(["a", "b"]);
  });
});

describe("mergeViews", () => {
  it("acrescenta caminhos novos e ignora os repetidos", () => {
    const first = views[0];
    if (first === undefined) {
      throw new Error("vista");
    }
    expect(mergeViews(views, [first, { path: "/tmp/tras.png", src: "", seed: null }])).toEqual([
      ...views,
      { path: "/tmp/tras.png", src: "", seed: null },
    ]);
  });
});

describe("meshViewsOf", () => {
  it("ignora slide sem caminho", () => {
    expect(meshViewsOf([...views, { path: "", src: "", seed: null }])).toEqual(views);
  });
});
