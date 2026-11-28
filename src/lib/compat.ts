import { formatList, formatNumber, t } from "@/i18n"
import type { Project, SystemInfo } from "./types"

/**
 * How a build suits the player's PC, from what is known: the cards it was
 * made for, and the memory its author reported it using. Reported use is not
 * a minimum, so the words stay careful: "below what the author reported",
 * never "won't run".
 *
 * The verdicts are the ones "Can my PC run GTA VI?" (portwatch-vi, lib/pc-check.ts)
 * gives a part: yes, close, no. A part is yes at 95% of what is wanted (benchmark
 * suites differ by about that much) and close at 80%. `concern` is the "no".
 *
 * The texts are in the current language: call `assess` while drawing, from a component
 * that uses `useT()`, so they follow a change of language.
 */
export type Level = "good" | "close" | "concern" | "unknown"

export interface Check {
  level: Level
  label: string
  /** What the PC has; null when it isn't known. */
  value: string | null
  note: string
}

export interface Assessment {
  level: Level
  headline: string
  /** One line for a build that doesn't fit (or only just does): what to watch for. Null when it fits. */
  warning: string | null
  gpu: Check
  ram: Check
  vram: Check
  /** Commit limit against the author's peak commit (Windows only; unknown elsewhere). */
  commit: Check
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

/**
 * The PC the fit is judged on: what the launcher found, with the player's own RAM and
 * video memory where they set them under "By PC spec" (Settings are in the filter menu).
 */
export function withSpec(
  system: SystemInfo | null,
  spec: { spec_ram_gb: number | null; spec_vram_gb: number | null } | undefined,
): SystemInfo | null {
  if (!spec || (spec.spec_ram_gb == null && spec.spec_vram_gb == null)) return system
  const base: SystemInfo = system ?? { gpus: [], ram_gb: null, cpu: null, cpu_cores: null }
  const gpus = base.gpus.map((g, i) => (i === 0 && spec.spec_vram_gb != null ? { ...g, vram_gb: spec.spec_vram_gb } : g))
  if (base.gpus.length === 0 && spec.spec_vram_gb != null) gpus.push({ name: "", vendor: "Other", vram_gb: spec.spec_vram_gb })
  return { ...base, gpus, ram_gb: spec.spec_ram_gb ?? base.ram_gb }
}

/**
 * The list's orders, by their index (`list_sort`). 0 keeps the order the launcher was given
 * (VIPortLive's), the rest sort by it. Their names: `sortLabel`.
 */
export const SORTS = ["listing", "name", "fps", "stars", "commits", "personalFps"] as const

export type SortId = (typeof SORTS)[number]

/** An order's name in the current language ("Frame rate"). */
export const sortLabel = (id: SortId) => t(`format.sort.${id}`)

/** A stable sort of a project list by the chosen order; unknown values sort last. */
export function sortProjects(list: Project[], sort: number): Project[] {
  if (sort === 0) return list
  const key: Record<number, (p: Project) => number | string | null> = {
    1: (p) => p.name.toLowerCase(),
    2: (p) => p.fps,
    3: (p) => p.listing.stars,
    4: (p) => p.listing.commits7d,
    5: (p) => p.personal_fps,
  }
  const k = key[sort]
  return [...list].sort((a, b) => {
    const x = k(a)
    const y = k(b)
    if (x == null || y == null) return x == null && y == null ? 0 : x == null ? 1 : -1
    if (typeof x === "string" && typeof y === "string") return x.localeCompare(y)
    return (y as number) - (x as number)
  })
}

/** yes at 95% of what the author used, close at 80%, else no (the same marks as the GTA VI check). */
export function ratio(have: number, used: number): Level {
  return have >= used * 0.95 ? "good" : have >= used * 0.8 ? "close" : "concern"
}

/** A size in GB, written the current language's way: "16 GB", "7.5 GB". */
const gb = (n: number) => `${formatNumber(n, Number.isInteger(n) ? undefined : { minimumFractionDigits: 1, maximumFractionDigits: 1 })} GB`

export function assess(p: Project, system: SystemInfo | null): Assessment {
  const gpu = mainGpu(system)
  const match = matchesCard(p, system)
  const cards = formatList(p.gpu)
  const gpuCheck: Check = {
    label: t("format.compat.gpu"),
    value: gpu?.name ? gpu.name.replace(/^(NVIDIA|AMD|Intel\(R\)) /, "") : null,
    level: match === true ? "good" : match === false ? "concern" : "unknown",
    note:
      match === true
        ? t("format.compat.madeForYours", { vendor: gpu!.vendor })
        : match === false
          ? t("format.compat.madeFor", { cards })
          : p.gpu.length === 0
            ? t("format.compat.noCards")
            : t("format.compat.madeFor", { cards }),
  }
  const memory = (label: string, have: number | null | undefined, used: number | null): Check => {
    const level: Level = have && used ? ratio(have, used) : "unknown"
    return {
      label,
      value: have ? gb(have) : null,
      level,
      note: !used
        ? t("format.compat.noUsage")
        : !have
          ? t("format.compat.reported", { amount: gb(used) })
          : level === "good"
            ? t("format.compat.above", { amount: gb(used) })
            : level === "close"
              ? t("format.compat.close", { amount: gb(used) })
              : t("format.compat.below", { amount: gb(used) }),
    }
  }
  const ram = memory(t("format.compat.ram"), system?.ram_gb, p.reported.ram_gb)
  const vram = memory(t("format.compat.vram"), gpu?.vram_gb, p.reported.vram_gb)
  // The verdict comes from the launcher (system.rs COMMIT_MARGIN_GB decides "short"); this row shows it.
  const commitPeak = p.reported.commit_gb
  const commit: Check = {
    label: t("format.compat.commit"),
    value: system?.commit_limit_gb ? gb(system.commit_limit_gb) : null,
    level: p.commit_short == null ? "unknown" : p.commit_short ? "concern" : "good",
    note:
      commitPeak == null
        ? t("format.compat.noCommitUsage")
        : p.commit_short == null
          ? t("format.compat.commitUnknown")
          : p.commit_short
            ? t("format.compat.commitBelow", { amount: gb(commitPeak) })
            : t("format.compat.commitAbove", { amount: gb(commitPeak) }),
  }
  const checks = [gpuCheck, ram, vram, commit]
  const level: Level = checks.some((c) => c.level === "concern")
    ? "concern"
    : checks.some((c) => c.level === "close")
      ? "close"
      : checks.some((c) => c.level === "good")
        ? "good"
        : "unknown"
  const headline =
    level === "concern"
      ? gpuCheck.level === "concern"
        ? t("format.compat.headline.card", { cards })
        : t("format.compat.headline.memory")
      : level === "close"
        ? t("format.compat.headline.close")
        : level === "good"
          ? t("format.compat.headline.good")
          : t("format.compat.headline.unknown")
  const warning =
    gpuCheck.level === "concern"
      ? t("format.compat.warning.card", { cards, vendor: gpu!.vendor })
      : ram.level === "concern"
        ? t("format.compat.warning.ram", { used: gb(p.reported.ram_gb!), have: gb(system!.ram_gb!) })
        : vram.level === "concern"
          ? t("format.compat.warning.vram", { used: gb(p.reported.vram_gb!), have: gb(gpu!.vram_gb!) })
          : commit.level === "concern"
            ? t("format.compat.warning.commit", { used: gb(commitPeak!), have: gb(system!.commit_limit_gb!) })
            : level === "close"
              ? t("format.compat.warning.close")
              : null
  return { level, headline, warning, gpu: gpuCheck, ram, vram, commit }
}
