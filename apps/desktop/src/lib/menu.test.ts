import { describe, expect, it } from "vitest";
import en from "../i18n/locales/en.json";
import ptBR from "../i18n/locales/pt-BR.json";
import { actionForMenu } from "./menu";

function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) {
    return [prefix];
  }
  return Object.entries(value).flatMap(([key, nested]) =>
    keys(nested, prefix === "" ? key : `${prefix}.${key}`),
  );
}

describe("traduções", () => {
  it("têm as mesmas chaves em pt-BR e en", () => {
    expect(keys(ptBR).sort()).toEqual(keys(en).sort());
  });
});

describe("actionForMenu", () => {
  it("reconhece as ações que já funcionam", () => {
    expect(actionForMenu("edit.preferences")).toBe("settings");
    expect(actionForMenu("view.theme")).toBe("toggle-theme");
    expect(actionForMenu("generate.run")).toBe("generate");
    expect(actionForMenu("help.redoSetup")).toBe("redo-setup");
    expect(actionForMenu("file.openImage")).toBe("open-image");
    expect(actionForMenu("edit.removeBackground")).toBe("remove-background");
    expect(actionForMenu("view.gallery")).toBe("gallery");
    expect(actionForMenu("view.viewer")).toBe("viewer");
  });

  it("ignora itens ainda sem função", () => {
    expect(actionForMenu("file.newFlow")).toBeNull();
    expect(actionForMenu("help.about")).toBeNull();
  });
});
