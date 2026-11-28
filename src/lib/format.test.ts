import { beforeAll, describe, expect, it } from "vitest"
import { setLocale } from "@/i18n"
import { fps, graphicsMode, gpuText, MODES, outputLabel, shortCommit, shortStage, withMode } from "./format"
import type { Graphics, Project } from "./types"

beforeAll(() => setLocale("en"))

const graphics = (over: Partial<Graphics> = {}): Graphics =>
  ({ series_s: true, render_scale: 0.5, ...over }) as Graphics

describe("graphics modes", () => {
  it("reads each mode back from the settings it writes", () => {
    for (const mode of MODES) {
      expect(graphicsMode(withMode(graphics(), mode.id))).toBe(mode.id)
    }
  })

  it("writes the Series S profile and scale for a mode", () => {
    expect(withMode(graphics(), "fast")).toMatchObject({ series_s: true, render_scale: 0.5 })
    expect(withMode(graphics(), "best")).toMatchObject({ series_s: false, render_scale: 1 })
  })

  it("is null for a scale that matches no mode", () => {
    expect(graphicsMode(graphics({ series_s: true, render_scale: 0.6 }))).toBeNull()
  })

  it("matches Best quality whatever the scale, since it has no Series S profile", () => {
    expect(graphicsMode(graphics({ series_s: false, render_scale: 0.3 }))).toBe("best")
  })
})

describe("outputLabel", () => {
  it("names the game default", () => {
    expect(outputLabel("Default")).toBe("Game default")
  })

  it("shows a resolution in pixels, with 4K named", () => {
    expect(outputLabel("P1080")).toBe("1080p")
    expect(outputLabel("P2160")).toBe("2160p (4K)")
  })
})

describe("fps", () => {
  const project = (value: number | null) => ({ fps: value }) as Project

  it("rounds a known frame rate", () => {
    expect(fps(project(59.6))).toBe(60)
  })

  it("is null when the rate is unknown or below half a frame", () => {
    expect(fps(project(null))).toBeNull()
    expect(fps(project(0.4))).toBeNull()
  })
})

describe("shortCommit", () => {
  it("keeps seven characters of a commit", () => {
    expect(shortCommit("0123456789abcdef")).toBe("0123456")
  })

  it("is null without a commit", () => {
    expect(shortCommit(null)).toBeNull()
    expect(shortCommit(undefined)).toBeNull()
    expect(shortCommit("")).toBeNull()
  })
})

describe("shortStage", () => {
  it("shortens a known stage to its short wording", () => {
    expect(shortStage("building", "Building (the first time, this can take a while)")).toBe("Building")
  })

  it("shows any other stage as the launcher wrote it", () => {
    expect(shortStage("other", "Some stage the launcher wrote")).toBe("Some stage the launcher wrote")
  })
})

describe("gpuText", () => {
  it("joins the cards with a middle dot", () => {
    expect(gpuText(["NVIDIA", "AMD"])).toBe("NVIDIA · AMD")
    expect(gpuText([])).toBe("")
  })
})
