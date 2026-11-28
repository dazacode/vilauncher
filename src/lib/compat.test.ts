import { beforeAll, describe, expect, it } from "vitest"
import { setLocale } from "@/i18n"
import { assess, matchesCard, mainGpu, ratio, sortProjects, withSpec } from "./compat"
import type { Project, SystemInfo } from "./types"

// The texts follow the language; pin English so the checks read plainly.
beforeAll(() => setLocale("en"))

const gpu = (vendor: SystemInfo["gpus"][number]["vendor"], vram_gb: number | null, name = `${vendor} card`) => ({
  name,
  vendor,
  vram_gb,
})

const system = (over: Partial<SystemInfo> = {}): SystemInfo => ({
  gpus: [gpu("NVIDIA", 8, "NVIDIA GeForce RTX 3060")],
  ram_gb: 16,
  cpu: "Test CPU",
  cpu_cores: 8,
  ...over,
})

function project(over: Partial<Project> = {}): Project {
  return {
    name: "Test",
    repo: "example/test",
    owner: null,
    gpu: ["NVIDIA"],
    fps: null,
    personal_fps: null,
    playtime: null,
    build_job: null,
    render: null,
    verified: false,
    summary: "",
    highlights: [],
    blockers: [],
    branch: null,
    branch_chosen: false,
    advice: [],
    listing: { id: null, nuked: null, tier: null, stars: null, commits7d: null },
    reported: { ram_gb: null, vram_gb: null, commit_gb: null },
    commit_short: null,
    custom: false,
    kind: "git",
    binary: null,
    web_url: null,
    installed: false,
    checking: false,
    ...over,
  } as Project
}

describe("mainGpu", () => {
  it("is null without a system or any card", () => {
    expect(mainGpu(null)).toBeNull()
    expect(mainGpu(system({ gpus: [] }))).toBeNull()
  })

  it("prefers a discrete NVIDIA or AMD card over integrated graphics", () => {
    const main = mainGpu(system({ gpus: [gpu("Intel", 2), gpu("AMD", 4), gpu("NVIDIA", 6)] }))
    expect(main?.vendor).toBe("NVIDIA")
    expect(mainGpu(system({ gpus: [gpu("Intel", 2), gpu("AMD", 4)] }))?.vendor).toBe("AMD")
  })

  it("takes the card with more video memory when two share a vendor", () => {
    const main = mainGpu(system({ gpus: [gpu("NVIDIA", 4, "small"), gpu("NVIDIA", 12, "big")] }))
    expect(main?.name).toBe("big")
  })
})

describe("matchesCard", () => {
  it("is null when the card or the project's cards are unknown", () => {
    expect(matchesCard(project(), null)).toBeNull()
    expect(matchesCard(project({ gpu: [] }), system())).toBeNull()
    expect(matchesCard(project(), system({ gpus: [gpu("Apple", 8)] }))).toBeNull()
  })

  it("is true when the project was made for the player's vendor", () => {
    expect(matchesCard(project({ gpu: ["AMD", "NVIDIA"] }), system())).toBe(true)
  })

  it("is false when the project was made for another vendor", () => {
    expect(matchesCard(project({ gpu: ["AMD"] }), system())).toBe(false)
  })
})

describe("withSpec", () => {
  it("returns the system unchanged when no spec is set", () => {
    const base = system()
    expect(withSpec(base, undefined)).toBe(base)
    expect(withSpec(base, { spec_ram_gb: null, spec_vram_gb: null })).toBe(base)
  })

  it("applies the player's RAM and the first card's video memory", () => {
    const out = withSpec(system({ gpus: [gpu("NVIDIA", 8), gpu("Intel", 2)] }), { spec_ram_gb: 32, spec_vram_gb: 12 })
    expect(out?.ram_gb).toBe(32)
    expect(out?.gpus[0].vram_gb).toBe(12)
    expect(out?.gpus[1].vram_gb).toBe(2)
  })

  it("keeps the detected RAM when only video memory is set", () => {
    const out = withSpec(system({ ram_gb: 16 }), { spec_ram_gb: null, spec_vram_gb: 6 })
    expect(out?.ram_gb).toBe(16)
  })

  it("adds a card when none was found but video memory is set", () => {
    const out = withSpec(system({ gpus: [], ram_gb: null }), { spec_ram_gb: null, spec_vram_gb: 6 })
    expect(out?.gpus).toEqual([{ name: "", vendor: "Other", vram_gb: 6 }])
  })

  it("builds a system from the spec alone when none was found", () => {
    const out = withSpec(null, { spec_ram_gb: 8, spec_vram_gb: null })
    expect(out?.ram_gb).toBe(8)
    expect(out?.gpus).toEqual([])
  })
})

describe("sortProjects", () => {
  const list = [
    project({ name: "beta", fps: 30 }),
    project({ name: "Alpha", fps: null }),
    project({ name: "gamma", fps: 90 }),
  ]

  it("keeps the listing order for sort 0", () => {
    expect(sortProjects(list, 0)).toBe(list)
  })

  it("sorts by name, ignoring case", () => {
    expect(sortProjects(list, 1).map((p) => p.name)).toEqual(["Alpha", "beta", "gamma"])
  })

  it("sorts by frame rate, highest first, with unknown values last", () => {
    expect(sortProjects(list, 2).map((p) => p.name)).toEqual(["gamma", "beta", "Alpha"])
  })

  it("does not change the list it was given", () => {
    const copy = [...list]
    sortProjects(list, 2)
    expect(list).toEqual(copy)
  })
})

describe("ratio", () => {
  it("is good at 95% of what was used, close at 80%, else a concern", () => {
    expect(ratio(9.5, 10)).toBe("good")
    expect(ratio(9.49, 10)).toBe("close")
    expect(ratio(8, 10)).toBe("close")
    expect(ratio(7.99, 10)).toBe("concern")
  })
})

describe("assess", () => {
  it("is unknown when the player's PC is not known", () => {
    const a = assess(project(), null)
    expect(a.level).toBe("unknown")
    expect(a.warning).toBeNull()
  })

  it("is good when the card and memory suit what the author used", () => {
    const a = assess(
      project({ reported: { ram_gb: 8, vram_gb: 6, commit_gb: null }, commit_short: false }),
      system({ ram_gb: 16, commit_limit_gb: 24 }),
    )
    expect(a.gpu.level).toBe("good")
    expect(a.ram.level).toBe("good")
    expect(a.vram.level).toBe("good")
    expect(a.level).toBe("good")
    expect(a.warning).toBeNull()
  })

  it("is a concern with a warning when the card was made for another vendor", () => {
    const a = assess(project({ gpu: ["AMD"] }), system())
    expect(a.gpu.level).toBe("concern")
    expect(a.level).toBe("concern")
    expect(a.warning).not.toBeNull()
  })

  it("is a concern about RAM when the PC has less than the author used", () => {
    const a = assess(project({ reported: { ram_gb: 32, vram_gb: null, commit_gb: null } }), system({ ram_gb: 16 }))
    expect(a.ram.level).toBe("concern")
    expect(a.warning).toContain("16")
  })

  it("is close when memory is just short of what the author used", () => {
    const a = assess(project({ reported: { ram_gb: 10, vram_gb: null, commit_gb: null } }), system({ ram_gb: 9 }))
    expect(a.ram.level).toBe("close")
    expect(a.level).toBe("close")
    expect(a.warning).not.toBeNull()
  })
})
