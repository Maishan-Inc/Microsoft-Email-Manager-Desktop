/** 主题：light / dark / system（默认跟随系统），切换 :root[data-theme]，持久化到 localStorage。 */

export type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "mem.theme";

function readSaved(): Theme {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    /* ignore */
  }
  return "system";
}

function systemPrefersDark(): boolean {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    return false;
  }
}

/** 把 mode 解析成实际生效的浅/深。 */
export function resolved(mode: Theme): "light" | "dark" {
  if (mode === "system") return systemPrefersDark() ? "dark" : "light";
  return mode;
}

function apply(mode: Theme) {
  try {
    document.documentElement.setAttribute("data-theme", resolved(mode));
  } catch {
    /* ignore */
  }
}

export const theme = $state<{ mode: Theme }>({ mode: readSaved() });

export function setTheme(mode: Theme) {
  theme.mode = mode;
  try {
    localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    /* ignore */
  }
  apply(mode);
}

// 初次应用 + 跟随系统变化（仅当处于 system 模式时）
apply(theme.mode);
try {
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => {
      if (theme.mode === "system") apply("system");
    });
} catch {
  /* ignore */
}
