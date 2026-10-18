export const LANGUAGE_KEY = "voxel.language";
export const THEME_KEY = "voxel.theme";

export type Language = "pt-BR" | "en";
export type Theme = "light" | "dark";

export function resolveLanguage(stored: string | null, navigatorLanguage: string): Language {
  if (stored === "pt-BR" || stored === "en") {
    return stored;
  }
  return navigatorLanguage.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export function resolveTheme(stored: string | null, prefersDark: boolean): Theme {
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  return prefersDark ? "dark" : "light";
}

export function applyTheme(theme: Theme): void {
  document.documentElement.classList.toggle("dark", theme === "dark");
  document.documentElement.style.colorScheme = theme;
  localStorage.setItem(THEME_KEY, theme);
}
