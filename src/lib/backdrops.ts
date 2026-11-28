import { useSyncExternalStore } from "react"
import { SHOTS } from "./backdrop-shots"

/**
 * The GTA VI pictures behind the library: Rockstar's official artwork, prepared by
 * assets/gta6-src/compose-backdrops.py, and its screenshots (backdrop-shots.ts, made by compose-screenshots.py). Each one says where its subject stands so the page can
 * keep it right of the title column:
 *
 * - aspect: the file's width / height (it covers the page by height)
 * - anchor: how far across the file the subject stands (0-1)
 * - target: how far across the page the subject should land (0-1); the picture never slides
 *   past its own edges, so a narrow window may leave it short of that
 * - ty: which part stays when a very wide window crops the height (0 top, 1 bottom)
 */
export interface Backdrop {
  id: string
  name: string
  /** For a screenshot: who or where it shows, for the picker's headings. */
  group?: string
  src: string
  thumb: string
  aspect: number
  anchor: number
  target: number
  ty: number
}

const art = (id: string, name: string, anchor: number, more: Partial<Backdrop> = {}): Backdrop => ({
  id,
  name,
  src: `/gta6/art/${id}.jpg`,
  thumb: `/gta6/art/thumbs/${id}.jpg`,
  aspect: 3200 / 1641,
  anchor,
  target: 0.72,
  ty: 0.2,
  ...more,
})

export const BACKDROPS: Backdrop[] = [
  // The artwork's own titles, kept as Rockstar names them (see docs/I18N.md: names stay as they are).
  { id: "robbery", name: "Jason and Lucia: Robbery", src: "/gta6/hero-bg.jpg", thumb: "/gta6/art/thumbs/robbery.jpg", aspect: 2707 / 2000, anchor: 0.7067, target: 0.816, ty: 1 }, // i18n-ignore
  art("beach", "Jason and Lucia: Beach", 0.696), // i18n-ignore
  art("rooftop", "Jason and Lucia: Rooftop", 0.744), // i18n-ignore
  art("jason-lucia-01", "Jason and Lucia", 0.656, { ty: 0.4 }), // i18n-ignore
  art("jason-lucia-02", "Jason and Lucia 02", 0.656, { ty: 0.3 }), // i18n-ignore
  art("lock-and-load", "Jason and Lucia: Lock and Load", 0.464, { ty: 0.3 }), // i18n-ignore
  art("motel", "Jason and Lucia: Motel", 0.56, { ty: 0.3 }), // i18n-ignore
  art("cover-art", "Official Cover Art", 0.576, { ty: 0.5 }), // i18n-ignore
  art("album-cover", "The Album Cover Art", 0.744, { ty: 0.5 }), // i18n-ignore
  art("boobie-ike", "Boobie Ike", 0.616, { ty: 0 }), // i18n-ignore
  art("brian-heder", "Brian Heder", 0.728, { ty: 0 }), // i18n-ignore
  art("cal-hampton", "Cal Hampton", 0.576, { target: 0.74, ty: 0.1 }), // i18n-ignore
  art("drequan-priest", "DreQuan Priest", 0.776, { ty: 0.1 }), // i18n-ignore
  art("raul-bautista", "Raul Bautista", 0.712, { ty: 0.1 }), // i18n-ignore
  art("real-dimez", "Real Dimez", 0.504, { ty: 0.1 }), // i18n-ignore
  // The postcards (one frame for all, straightened in compose-backdrops.py), by where their titles stand.
  art("vice-city", "Vice City", 0.6, { aspect: 2768 / 1641, ty: 0.5 }), // i18n-ignore
  art("leonida-keys", "Leonida Keys", 1, { aspect: 2768 / 1641, target: 1, ty: 0.5 }), // i18n-ignore
  // Titles that run wide: the photo is centred on the page, so the whole title shows.
  art("port-gellhorn", "Port Gellhorn", 0.6, { aspect: 2768 / 1641, target: 0.5, ty: 0.5 }), // i18n-ignore
  art("ambrosia", "Ambrosia", 0.6, { aspect: 2768 / 1641, target: 0.5, ty: 0.5 }), // i18n-ignore
  art("grassrivers", "Grassrivers", 0.6, { aspect: 2768 / 1641, target: 0.5, ty: 0.5 }), // i18n-ignore
  // The postcard's title is on its right edge: keep that edge on the page's.
  art("mount-kalaga", "Mount Kalaga National Park", 1, { aspect: 2768 / 1641, target: 1, ty: 0.5 }), // i18n-ignore
  // The same art with Rockstar's logos on it.
  art("robbery-logos", "Jason and Lucia: Robbery (With Logo)", 0.64, { ty: 0.3 }), // i18n-ignore
  art("beach-logos", "Jason and Lucia: Beach (With Logos)", 0.696), // i18n-ignore
  art("rooftop-logos", "Jason and Lucia: Rooftop (With Logos)", 0.744), // i18n-ignore
  art("jason-lucia-01-logos", "Jason and Lucia (With Logos)", 0.656, { ty: 0.4 }), // i18n-ignore
  art("jason-lucia-02-logos", "Jason and Lucia 02 (With Logos)", 0.656, { ty: 0.3 }), // i18n-ignore
  art("lock-and-load-logos", "Jason and Lucia: Lock and Load (With Logos)", 0.464, { ty: 0.3 }), // i18n-ignore
]

/** Every picture the library can show: the artwork, then the screenshots. */
export const ALL_BACKDROPS: Backdrop[] = [...BACKDROPS, ...SHOTS]

/** "shuffle" picks one at random each time the launcher starts. */
export type BackdropChoice = string | "shuffle"

const KEY = "vilauncher.backdrop"
const listeners = new Set<() => void>()
let choice: BackdropChoice = localStorage.getItem(KEY) ?? "robbery"
if (choice !== "shuffle" && !ALL_BACKDROPS.some((b) => b.id === choice)) choice = "robbery"
// This launch's pick when shuffling: fixed until the next start, so the page doesn't jump around.
const shuffled = ALL_BACKDROPS[Math.floor(Math.random() * ALL_BACKDROPS.length)]

export function setBackdrop(next: BackdropChoice) {
  choice = next
  localStorage.setItem(KEY, next)
  listeners.forEach((l) => l())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** The saved choice: a backdrop id, or "shuffle". */
export function useBackdropChoice(): BackdropChoice {
  return useSyncExternalStore(subscribe, () => choice, () => "robbery")
}

/** The picture to show now. */
export function useBackdrop(): Backdrop {
  const c = useBackdropChoice()
  return c === "shuffle" ? shuffled : (ALL_BACKDROPS.find((b) => b.id === c) ?? BACKDROPS[0])
}
