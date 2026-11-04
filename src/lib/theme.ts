// Theme choice: "auto" follows the Windows setting, the others force light or dark tokens (src/theme.css).
export type Theme = "auto" | "light" | "dark";

const KEY = "cs-theme";

export function loadTheme(): Theme {
  try {
    const saved = localStorage.getItem(KEY);
    return saved === "light" || saved === "dark" ? saved : "auto";
  } catch {
    return "auto";
  }
}

export function applyTheme(theme: Theme) {
  if (theme === "auto") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", theme);
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // the choice still applies until the app is closed
  }
}
