import { useSyncExternalStore } from "react"

/**
 * The GTA VI pictures behind the library: Rockstar's official artwork, prepared by
 * assets/gta6-src/compose-backdrops.py. Each one says where its subject stands so the page can
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
  { id: "robbery", name: "Jason and Lucia: Robbery", src: "/gta6/hero-bg.jpg", thumb: "/gta6/art/thumbs/robbery.jpg", aspect: 2707 / 2000, anchor: 0.7067, target: 0.816, ty: 1 },
  art("beach", "Jason and Lucia: Beach", 0.696),
  art("rooftop", "Jason and Lucia: Rooftop", 0.744),
  art("jason-lucia-01", "Jason and Lucia", 0.656, { ty: 0.4 }),
  art("boobie-ike", "Boobie Ike", 0.616, { ty: 0 }),
  art("brian-heder", "Brian Heder", 0.728, { ty: 0 }),
  art("cal-hampton", "Cal Hampton", 0.576, { target: 0.74, ty: 0.1 }),
  art("drequan-priest", "DreQuan Priest", 0.776, { ty: 0.1 }),
  // The postcard's title is on its right edge: keep that edge on the page's.
  art("mount-kalaga", "Mount Kalaga National Park", 1, { aspect: 2768 / 1641, target: 1, ty: 0.5 }),
]

/** "shuffle" picks one at random each time the launcher starts. */
export type BackdropChoice = string | "shuffle"

const KEY = "vilauncher.backdrop"
const listeners = new Set<() => void>()
let choice: BackdropChoice = localStorage.getItem(KEY) ?? "robbery"
if (choice !== "shuffle" && !BACKDROPS.some((b) => b.id === choice)) choice = "robbery"
// This launch's pick when shuffling: fixed until the next start, so the page doesn't jump around.
const shuffled = BACKDROPS[Math.floor(Math.random() * BACKDROPS.length)]

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
  return c === "shuffle" ? shuffled : (BACKDROPS.find((b) => b.id === c) ?? BACKDROPS[0])
}
