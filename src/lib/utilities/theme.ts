import type { ThemePreference } from "@/types/domain";

export type ResolvedTheme = "dark" | "light";

export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === "system") {
    return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  return preference;
}

/** Applies the resolved theme to the document root via `data-theme`, which
 * `src/styles/globals.css` keys every color token off of. */
export function applyTheme(preference: ThemePreference): void {
  const resolved = resolveTheme(preference);
  document.documentElement.dataset.theme = resolved;
}
