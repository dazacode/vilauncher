import type { Project, SystemInfo } from "./types"

/**
 * How a project suits the player's PC, from what is known: the cards it was
 * made for, and the memory its author reported it using. Reported use is not
 * a minimum, so the words stay careful: "below what the author reported",
 * never "won't run".
 */
export type Level = "good" | "concern" | "unknown"

export interface Check {
  level: Level
  label: string
  value: string
  note: string
}

export interface Assessment {
  level: Level
  headline: string
  gpu: Check
  ram: Check
  vram: Check
}

type Gpu = SystemInfo["gpus"][number]

/** The card that plays games: a discrete NVIDIA or AMD one before integrated graphics. */
export function mainGpu(system: SystemInfo | null): Gpu | null {
  if (!system || system.gpus.length === 0) return null
  const rank = (g: Gpu) => (g.vendor === "NVIDIA" ? 0 : g.vendor === "AMD" ? 1 : g.vendor === "Intel" ? 2 : 3)
  return [...system.gpus].sort((a, b) => rank(a) - rank(b) || (b.vram_gb ?? 0) - (a.vram_gb ?? 0))[0]
}

/** Whether `p` was made for the player's card: null when either is unknown. */
export function matchesCard(p: Project, system: SystemInfo | null): boolean | null {
  const gpu = mainGpu(system)
  if (!gpu || p.gpu.length === 0 || (gpu.vendor !== "NVIDIA" && gpu.vendor !== "AMD")) return null
  return p.gpu.some((g) => g.toLowerCase() === gpu.vendor.toLowerCase())
}

const gb = (n: number) => `${Number.isInteger(n) ? n : n.toFixed(1)} GB`

export function assess(p: Project, system: SystemInfo | null): Assessment {
  const gpu = mainGpu(system)
  const match = matchesCard(p, system)
  const gpuCheck: Check = {
    label: "Graphics card",
    value: gpu ? gpu.name.replace(/^(NVIDIA|AMD|Intel\(R\)) /, "") : "Unknown",
    level: match === true ? "good" : match === false ? "concern" : "unknown",
    note:
      match === true
        ? `Made for ${gpu!.vendor} cards, like yours`
        : match === false
          ? `Made for ${p.gpu.join(" and ")} cards`
          : p.gpu.length === 0
            ? "The author hasn't said which cards it runs on"
            : `Made for ${p.gpu.join(" and ")} cards`,
  }
  const memory = (label: string, have: number | null | undefined, used: number | null): Check => ({
    label,
    value: have ? gb(have) : "Unknown",
    level: have && used ? (have >= used ? "good" : "concern") : "unknown",
    note: !used ? "No usage reported" : !have ? `The author reported about ${gb(used)}` : have >= used ? `Above the ${gb(used)} the author reported` : `Below the ${gb(used)} the author reported`,
  })
  const ram = memory("System memory", system?.ram_gb, p.reported.ram_gb)
  const vram = memory("Video memory", gpu?.vram_gb, p.reported.vram_gb)
  const checks = [gpuCheck, ram, vram]
  const level: Level = checks.some((c) => c.level === "concern") ? "concern" : checks.some((c) => c.level === "good") ? "good" : "unknown"
  const headline =
    level === "concern"
      ? gpuCheck.level === "concern"
        ? `Made for ${p.gpu.join(" and ")}: it may not start, or run slowly, on your card.`
        : "It may run slowly on your PC: it used more memory on the author's."
      : level === "good"
        ? "A good match for your PC, from what its author reported."
        : "Not enough is known yet to compare it with your PC."
  return { level, headline, gpu: gpuCheck, ram, vram }
}
