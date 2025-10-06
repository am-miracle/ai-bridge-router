import { THEME_STORAGE_KEY, THEME_CLASSES } from "./constants";
import type { Theme } from "@/types";

export function getStoredTheme(): Theme | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(THEME_STORAGE_KEY) as Theme | null;
}

export function setStoredTheme(theme: Theme): void {
  if (typeof window === "undefined") return;
  localStorage.setItem(THEME_STORAGE_KEY, theme);
}

export function getSystemTheme(): Theme {
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function getInitialTheme(): Theme {
  return getStoredTheme() || getSystemTheme();
}

export function applyTheme(theme: Theme): void {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle(THEME_CLASSES.DARK, theme === "dark");
}

export function toggleTheme(currentTheme: Theme): Theme {
  return currentTheme === "light" ? "dark" : "light";
}
