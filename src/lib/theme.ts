import { useSyncExternalStore } from "react"

/**
 * The launcher's look. The choice is kept in the browser's local storage, so it needs no settings
 * change in the backend. Every colour lives in src/styles.css under a `data-theme` attribute.
 */
export const THEMES = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "calm", label: "Calm" },
] as const

export type Theme = (typeof THEMES)[number]["id"] | "paws"

const THEME_KEY = "vilauncher.theme"
const PAWS_KEY = "vilauncher.paws"

const readTheme = (): Theme => {
  const saved = localStorage.getItem(THEME_KEY)
  if (saved === "paws" && localStorage.getItem(PAWS_KEY) === "1") return "paws"
  return THEMES.some((t) => t.id === saved) ? (saved as Theme) : "dark"
}

let current: Theme = readTheme()
const listeners = new Set<() => void>()

function apply() {
  document.documentElement.dataset.theme = current
  listeners.forEach((listener) => listener())
}

apply()

export function setTheme(theme: Theme) {
  current = theme
  localStorage.setItem(THEME_KEY, theme)
  apply()
}

/** The hidden theme. Seven clicks on the logo unlock it; it then appears in Settings → Appearance. */
export function unlockPaws() {
  localStorage.setItem(PAWS_KEY, "1")
  setTheme("paws")
}

export function pawsUnlocked() {
  return localStorage.getItem(PAWS_KEY) === "1"
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** The current theme, updated when it changes. */
export function useTheme(): Theme {
  return useSyncExternalStore(subscribe, () => current, () => "dark")
}
