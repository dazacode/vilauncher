import { t } from "@/i18n"
import type { Graphics, OutputResolution, Project, StageKind } from "./types"

/** The graphics modes: Series S profile and render scale. Labels: `modeLabel`, `modeDetail`. */
export const MODES = [
  { id: "fast", seriesS: true, scale: 0.5 },
  { id: "balanced", seriesS: true, scale: 0.75 },
  { id: "best", seriesS: false, scale: 1 },
] as const

export type ModeId = (typeof MODES)[number]["id"]

/** A mode's name in the current language ("Best quality"). */
export const modeLabel = (id: ModeId) => t(`format.mode.${id}.label`)

/** What a mode does, in the current language ("Full resolution"). */
export const modeDetail = (id: ModeId) => t(`format.mode.${id}.detail`)

export function graphicsMode(g: Graphics): ModeId | null {
  const m = MODES.find((m) => g.series_s === m.seriesS && (!m.seriesS || Math.abs(g.render_scale - m.scale) < 0.01))
  return m?.id ?? null
}

export function withMode(g: Graphics, id: ModeId): Graphics {
  const m = MODES.find((m) => m.id === id)!
  return { ...g, series_s: m.seriesS, render_scale: m.scale }
}

/** The output resolutions, in order. Labels: `outputLabel`. */
export const OUTPUTS: readonly OutputResolution[] = ["Default", "P720", "P936", "P1080", "P1440", "P2160"]

/** An output resolution as the player reads it: "Game default", "1080p", "2160p (4K)". */
export function outputLabel(id: OutputResolution): string {
  if (id === "Default") return t("format.output.default")
  return id === "P2160" ? "2160p (4K)" : `${id.slice(1)}p`
}

export const gpuText = (gpu: string[]) => gpu.join(" · ")

export function fps(p: Project) {
  return p.fps != null && p.fps >= 0.5 ? Math.round(p.fps) : null
}

export function shortCommit(c: string | null | undefined) {
  return c ? c.slice(0, 7) : null
}

/**
 * A short stage for small places ("Building" for "Building (the first time…)"), from the
 * stage's kind; other stages are shown as the launcher wrote them (already translated).
 */
export function shortStage(kind: StageKind, stage: string) {
  switch (kind) {
    case "downloading":
    case "building":
    case "warming":
    case "installing_tools":
    case "uninstalling":
    case "moving":
    case "importing":
    case "playing":
      return t(`format.stage.${kind}`)
    default:
      return stage
  }
}
