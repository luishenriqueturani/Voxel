import { describe, expect, it } from "vitest";
import { applyTheme, resolveLanguage, resolveTheme } from "./preferences";

describe("resolveLanguage", () => {
  it("prioriza o valor salvo", () => {
    expect(resolveLanguage("en", "pt-BR")).toBe("en");
    expect(resolveLanguage("pt-BR", "en-US")).toBe("pt-BR");
  });

  it("usa o idioma do navegador quando não há valor salvo", () => {
    expect(resolveLanguage(null, "pt-BR")).toBe("pt-BR");
    expect(resolveLanguage("outro", "en-US")).toBe("en");
  });
});

describe("resolveTheme", () => {
  it("prioriza o valor salvo", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("cai para a preferência do sistema", () => {
    expect(resolveTheme(null, true)).toBe("dark");
    expect(resolveTheme("outro", false)).toBe("light");
  });
});

describe("applyTheme", () => {
  it("aplica a classe, o color-scheme e persiste", () => {
    applyTheme("dark");

    expect(document.documentElement).toHaveClass("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
    expect(localStorage.getItem("voxel.theme")).toBe("dark");

    applyTheme("light");

    expect(document.documentElement).not.toHaveClass("dark");
    expect(localStorage.getItem("voxel.theme")).toBe("light");
  });
});
