import { useCallback } from "react"

/**
 * The slow, endless drift of the art behind a project, on one shared timer.
 *
 * The art moves a few pixels a second, so 12 steps a second look the same as
 * 60 and cost a fifth of the redraws (every step repaints the hero, its grain
 * and the glass over it). While the window is hidden (minimised, or on another
 * desktop), the art holds still where it is and costs nothing; beside another
 * window it keeps moving. While a game runs, the hero holds it still anyway.
 */
const STEPS_PER_SECOND = 12

/** Where a layer sits: x and y in % of its own size, rotate in degrees. */
export interface Pose {
  x: number
  y: number
  scale: number
  rotate: number
}

/** From one pose to the other and back, eased, every `duration` seconds. */
export interface Drift {
  from: Pose
  to: Pose
  duration: number
}

const layers = new Map<HTMLElement, Drift>()
let timer: ReturnType<typeof setInterval> | undefined
// Milliseconds of drifting so far. It runs only while the timer does, so art that held still
// (window hidden, a game running) goes on from where it stopped instead of jumping ahead.
let clock = 0
let last = 0

function tick() {
  const now = performance.now()
  clock += now - last
  last = now
}

export function poseAt(drift: Drift, ms: number): Pose {
  const phase = ((ms / 1000) % drift.duration) / drift.duration
  const there = phase < 0.5 ? phase * 2 : 2 - phase * 2
  const k = 0.5 - Math.cos(Math.PI * there) / 2
  const mix = (a: number, b: number) => a + (b - a) * k
  const { from, to } = drift
  return { x: mix(from.x, to.x), y: mix(from.y, to.y), scale: mix(from.scale, to.scale), rotate: mix(from.rotate, to.rotate) }
}

export function transformOf(p: Pose) {
  return `translateX(${p.x.toFixed(3)}%) translateY(${p.y.toFixed(3)}%) scale(${p.scale.toFixed(4)}) rotate(${p.rotate.toFixed(3)}deg)`
}

function place(el: HTMLElement, drift: Drift) {
  el.style.transform = transformOf(poseAt(drift, clock))
}

function step() {
  tick()
  layers.forEach((drift, el) => place(el, drift))
}

function shown() {
  return document.visibilityState === "visible"
}

function sync() {
  const run = layers.size > 0 && shown()
  if (run && timer === undefined) {
    last = performance.now()
    step()
    timer = setInterval(step, 1000 / STEPS_PER_SECOND)
  } else if (!run && timer !== undefined) {
    clearInterval(timer)
    timer = undefined
  }
}

if (typeof window !== "undefined") {
  document.addEventListener("visibilitychange", sync)
}

/**
 * A ref that drifts its element; null leaves it where it is. Pass a drift that
 * keeps its identity between renders (useMemo), or the layer restarts its pose.
 */
export function useDrift(drift: Drift | null) {
  return useCallback(
    (el: HTMLElement | null) => {
      if (!el) return
      if (!drift) return
      place(el, drift)
      layers.set(el, drift)
      sync()
      return () => {
        layers.delete(el)
        sync()
      }
    },
    [drift],
  )
}
