import { useSyncExternalStore } from "react"
import { t } from "@/i18n"
import { inTauri, setWindowLook } from "@/lib/api"

/**
 * The launcher's look. Two built-in themes (Dark and Light, and a hidden one), and up to three of
 * the player's own: a name, a dark or light base, an accent, a background, how much of the system's
 * glass shows through, and how round the corners are. Everything else is worked out from those
 * (palette below), kept readable whatever is picked. Themes are kept in this PC's local storage, and
 * travel as share codes ("VITHEME1:…"). The built-in colours live in src/styles.css under
 * `data-theme`; a custom theme sets the same tokens on <html> itself.
 *
 * The window follows the theme too (setWindowLook): Mica on Windows 11 in the theme's light or dark,
 * Acrylic tinted with its background on Windows 10, the window material on macOS, off when Flat.
 * Linux has no system glass, so there the glass only makes panels over the art see-through.
 */
export type Base = "dark" | "light"

export interface ThemeSpec {
  name: string
  base: Base
  /** #rrggbb */
  accent: string
  /** #rrggbb */
  ground: string
  /** 0 flat … 100 as much glass as the system has. */
  glass: number
  /** Corner roundness in %, 0 square … 150. */
  roundness: number
}

export interface CustomTheme extends ThemeSpec {
  id: string
}

export const MAX_CUSTOM = 3
export type BuiltIn = "dark" | "light" | "paws"

/** What the built-in looks are, as specs: where a new theme starts from. */
export const BUILT_IN: Record<BuiltIn, ThemeSpec> = {
  dark: { name: "", base: "dark", accent: "#e0338a", ground: "#0c0b10", glass: 60, roundness: 100 },
  light: { name: "", base: "light", accent: "#b3236a", ground: "#f5f4f7", glass: 30, roundness: 100 },
  paws: { name: "", base: "dark", accent: "#e9925a", ground: "#1c1512", glass: 40, roundness: 100 },
}

/** The built-in looks a player can pick ("paws" only once unlocked). */
export const THEMES = [{ id: "dark" }, { id: "light" }] as const

/** A built-in look's name in the current language ("Dark"). */
export const themeLabel = (theme: BuiltIn) => t(`format.theme.${theme}`)

interface State {
  /** A built-in id, or a custom theme's id. */
  active: string
  custom: CustomTheme[]
}

const KEY = "vilauncher.themes"
const OLD_KEY = "vilauncher.theme"
const PAWS_KEY = "vilauncher.paws"

// --- Colour ------------------------------------------------------------------------------------

type RGB = [number, number, number]

const HEX = /^#[0-9a-f]{6}$/i
const hexRgb = (hex: string): RGB => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as RGB
const rgbHex = (c: RGB) => "#" + c.map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0")).join("")
const mix = (a: RGB, b: RGB, k: number): RGB => [0, 1, 2].map((i) => a[i] + (b[i] - a[i]) * k) as RGB
const rgba = (c: RGB, alpha: number) => `rgb(${c.map(Math.round).join(" ")} / ${Math.round(alpha * 1000) / 1000})`

function toHsl([r, g, b]: RGB): RGB {
  r /= 255; g /= 255; b /= 255
  const max = Math.max(r, g, b), min = Math.min(r, g, b)
  const l = (max + min) / 2
  if (max === min) return [0, 0, l]
  const d = max - min
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4
  return [h / 6, s, l]
}

function fromHsl([h, s, l]: RGB): RGB {
  if (s === 0) return [l * 255, l * 255, l * 255]
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q
  const hue = (t: number) => {
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1 / 6) return p + (q - p) * 6 * t
    if (t < 1 / 2) return q
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6
    return p
  }
  return [hue(h + 1 / 3) * 255, hue(h) * 255, hue(h - 1 / 3) * 255]
}

/** The colour with its lightness kept between lo and hi (0-1). */
const clampLight = (c: RGB, lo: number, hi: number): RGB => {
  const [h, s, l] = toHsl(c)
  return fromHsl([h, s, Math.min(hi, Math.max(lo, l))])
}

/** The background as the base allows it: dark stays dark and light stays light, so text reads. */
export const usableGround = (spec: ThemeSpec) => rgbHex(spec.base === "dark" ? clampLight(hexRgb(spec.ground), 0, 0.2) : clampLight(hexRgb(spec.ground), 0.86, 1))
/** The accent as buttons can carry it: white text on it stays readable. */
export const usableAccent = (spec: ThemeSpec) => rgbHex(clampLight(hexRgb(spec.accent), 0.32, 0.6))

/** Every colour token of a custom theme, from its few choices. */
export function palette(spec: ThemeSpec): Record<string, string> {
  const dark = spec.base === "dark"
  const g = spec.glass / 100
  const ground = hexRgb(usableGround(spec))
  const accent = hexRgb(usableAccent(spec))
  const fg: RGB = dark ? mix([255, 255, 255], ground, 0.06) : mix([0, 0, 0], ground, 0.12)
  const ink = dark ? [0.7, 0.56, 0.36] : [0.72, 0.58, 0.4]
  const fills = dark ? [0.04, 0.07, 0.11, 0.16] : [0.03, 0.06, 0.09, 0.13]
  const raised = dark ? mix(ground, [255, 255, 255], 0.07) : mix(ground, [255, 255, 255], 0.85)
  const accentFg = dark ? mix(accent, [255, 255, 255], 0.45) : mix(accent, [0, 0, 0], 0.12)
  const sidebar = dark ? mix(ground, [0, 0, 0], 0.45) : mix(ground, [255, 255, 255], 0.5)
  const vars: Record<string, string> = {
    "--color-ground": rgbHex(ground),
    "--color-page": rgba(ground, 1 - g * 0.35),
    "--color-sidebar": rgba(sidebar, 1 - g * (dark ? 0.5 : 0.4)),
    "--color-fg": rgbHex(fg),
    "--color-fg-2": rgba(fg, ink[0]),
    "--color-fg-3": rgba(fg, ink[1]),
    "--color-fg-4": rgba(fg, ink[2]),
    "--color-fill-1": rgba(fg, fills[0]),
    "--color-fill-2": rgba(fg, fills[1]),
    "--color-fill-3": rgba(fg, fills[2]),
    "--color-fill-4": rgba(fg, fills[3]),
    "--color-line": rgba(fg, dark ? 0.08 : 0.09),
    "--color-line-2": rgba(fg, dark ? 0.13 : 0.16),
    "--color-accent": rgbHex(accent),
    "--color-accent-hover": rgbHex(mix(accent, [255, 255, 255], 0.1)),
    "--color-accent-soft": rgba(accent, dark ? 0.16 : 0.12),
    "--color-accent-fg": rgbHex(accentFg),
    "--color-accent-art": rgbHex(mix(accent, [255, 255, 255], 0.45)),
    "--color-accent-selection": rgba(accent, dark ? 0.35 : 0.22),
    "--color-focus": rgba(accentFg, 0.75),
    "--color-raised": rgbHex(raised),
    "--color-glass": rgba(raised, 0.97 - g * 0.12),
    "--color-glass-solid": rgba(raised, 0.96 - g * 0.1),
    "--color-glass-thin": rgba(fg, dark ? 0.06 : 0.03),
    "--color-ok": dark ? "#3ccf8e" : "#15803d",
    "--color-warn": dark ? "#f4b13e" : "#b45309",
    "--color-bad": dark ? "#ff6369" : "#b91c1c",
  }
  const r = spec.roundness / 100
  for (const [name, px] of [["xs", 4], ["sm", 6], ["md", 8], ["lg", 12], ["xl", 16]] as const) vars[`--radius-${name}`] = `${Math.round(px * r * 10) / 10}px`
  return vars
}

// --- Share codes -------------------------------------------------------------------------------

const PREFIX = "VITHEME1:"

/** A theme as a code to send to someone. */
export function shareCode(spec: ThemeSpec): string {
  const data = { n: spec.name, b: spec.base, a: spec.accent, g: spec.ground, gl: spec.glass, r: spec.roundness }
  const bytes = new TextEncoder().encode(JSON.stringify(data))
  const text = btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "")
  return PREFIX + text
}

/** The theme in a share code, or null when the code is not one. Only known fields, checked and clamped. */
export function readShareCode(code: string): ThemeSpec | null {
  const text = code.trim()
  if (!text.startsWith(PREFIX) || text.length > 600) return null
  try {
    const b64 = text.slice(PREFIX.length).replace(/-/g, "+").replace(/_/g, "/")
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
    const d = JSON.parse(new TextDecoder().decode(bytes)) as Record<string, unknown>
    if (typeof d !== "object" || d === null) return null
    const { n, b, a, g, gl, r } = d
    if ((b !== "dark" && b !== "light") || typeof a !== "string" || !HEX.test(a) || typeof g !== "string" || !HEX.test(g)) return null
    if (typeof gl !== "number" || typeof r !== "number" || !Number.isFinite(gl) || !Number.isFinite(r)) return null
    return {
      name: cleanName(typeof n === "string" ? n : ""),
      base: b,
      accent: a.toLowerCase(),
      ground: g.toLowerCase(),
      glass: Math.round(Math.min(100, Math.max(0, gl))),
      roundness: Math.round(Math.min(150, Math.max(0, r))),
    }
  } catch {
    return null
  }
}

/** A name as it is kept: no control characters, at most 32 characters. */
export const cleanName = (name: string) => name.replace(/[\u0000-\u001f\u007f]/g, "").trim().slice(0, 32)

// --- State -------------------------------------------------------------------------------------

const valid = (c: unknown): c is CustomTheme => {
  const x = c as CustomTheme
  return !!x && typeof x.id === "string" && typeof x.name === "string" && (x.base === "dark" || x.base === "light") && HEX.test(x.accent) && HEX.test(x.ground) && typeof x.glass === "number" && typeof x.roundness === "number"
}

function read(): State {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as State | null
    if (saved && Array.isArray(saved.custom)) {
      const custom = saved.custom.filter(valid).slice(0, MAX_CUSTOM)
      return { active: typeof saved.active === "string" ? saved.active : "dark", custom }
    }
  } catch {
    // A broken entry starts over from Dark.
  }
  // The looks before themes could be made: Calm becomes a theme of the player's own, as it was.
  const old = localStorage.getItem(OLD_KEY)
  if (old === "calm") {
    const calm: CustomTheme = { id: newId(), name: t("format.theme.calm"), base: "dark", accent: "#5b9aa5", ground: "#101417", glass: 40, roundness: 100 }
    return { active: calm.id, custom: [calm] }
  }
  return { active: old === "light" || old === "paws" ? old : "dark", custom: [] }
}

function newId() {
  return Math.random().toString(36).slice(2, 10)
}

let state: State = read()
const listeners = new Set<() => void>()

/** The look in effect: a built-in id or a custom theme, and its spec. */
export interface Look {
  id: string
  builtIn: BuiltIn | null
  spec: ThemeSpec
}

function resolve(s: State): Look {
  const custom = s.custom.find((c) => c.id === s.active)
  if (custom) return { id: custom.id, builtIn: null, spec: custom }
  const builtIn: BuiltIn = s.active === "light" ? "light" : s.active === "paws" && pawsUnlocked() ? "paws" : "dark"
  return { id: builtIn, builtIn, spec: BUILT_IN[builtIn] }
}

let look: Look = resolve(state)
let applied: string[] = []
let nativeTimer: ReturnType<typeof setTimeout> | undefined

function apply() {
  look = resolve(state)
  const root = document.documentElement
  for (const name of applied) root.style.removeProperty(name)
  applied = []
  root.dataset.theme = look.builtIn ?? look.spec.base
  root.style.colorScheme = look.spec.base
  if (look.spec.glass === 0) root.dataset.flat = ""
  else delete root.dataset.flat
  if (!look.builtIn) {
    for (const [name, value] of Object.entries(palette(look.spec))) {
      root.style.setProperty(name, value)
      applied.push(name)
    }
  }
  // The window's own glass follows, once a slider stops moving.
  if (inTauri) {
    clearTimeout(nativeTimer)
    const { base, glass } = look.spec
    const tint = hexRgb(look.builtIn ? look.spec.ground : usableGround(look.spec))
    nativeTimer = setTimeout(() => void setWindowLook(base === "dark", glass > 0, tint), 120)
  }
  listeners.forEach((listener) => listener())
}

function save(next: State) {
  state = next
  localStorage.setItem(KEY, JSON.stringify(state))
  apply()
}

apply()

/** Switches to a built-in look or a custom theme by id. */
export function setTheme(id: string) {
  save({ ...state, active: id })
}

/** A new custom theme from spec, switched to; null when all slots are taken. */
export function addTheme(spec: ThemeSpec): string | null {
  if (state.custom.length >= MAX_CUSTOM) return null
  const theme: CustomTheme = { ...spec, name: cleanName(spec.name), id: newId() }
  save({ active: theme.id, custom: [...state.custom, theme] })
  return theme.id
}

/** Changes a custom theme (the name is cleaned, numbers kept in range). */
export function updateTheme(id: string, patch: Partial<ThemeSpec>) {
  const custom = state.custom.map((c) => {
    if (c.id !== id) return c
    const next = { ...c, ...patch }
    if (patch.name !== undefined) next.name = cleanName(patch.name)
    next.glass = Math.round(Math.min(100, Math.max(0, next.glass)))
    next.roundness = Math.round(Math.min(150, Math.max(0, next.roundness)))
    return next
  })
  save({ ...state, custom })
}

/** Deletes a custom theme; the look goes back to the theme's base if it was in use. */
export function deleteTheme(id: string) {
  const gone = state.custom.find((c) => c.id === id)
  const custom = state.custom.filter((c) => c.id !== id)
  save({ active: state.active === id ? (gone?.base ?? "dark") : state.active, custom })
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

/** The look in effect, updated when it changes. */
export function useLook(): Look {
  return useSyncExternalStore(subscribe, () => look, () => look)
}

/** The player's own themes. */
export function useCustomThemes(): CustomTheme[] {
  return useSyncExternalStore(subscribe, () => state.custom, () => state.custom)
}

/** The built-in look in effect, or "custom". */
export function useTheme(): BuiltIn | "custom" {
  return useSyncExternalStore(subscribe, () => look.builtIn ?? "custom", () => "dark")
}
