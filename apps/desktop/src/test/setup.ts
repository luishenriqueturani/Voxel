import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";
import i18n from "../i18n";

beforeEach(async () => {
  localStorage.clear();
  document.documentElement.classList.remove("dark");
  document.documentElement.style.colorScheme = "";
  document.documentElement.lang = "pt-BR";
  await i18n.changeLanguage("pt-BR");
});

afterEach(() => {
  cleanup();
});
