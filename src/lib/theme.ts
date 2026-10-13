import type { Theme } from "./types";

/** "system" removes the override so the CSS media query decides. */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") delete root.dataset.theme;
  else root.dataset.theme = theme;
}
