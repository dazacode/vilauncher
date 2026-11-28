import { toast } from "sonner"
import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import { setLocale, useT } from "@/i18n"
import { act as send, getBackend } from "./api"
import type { Action, PlayChoice, Project, Snapshot } from "./types"

export type View = "library" | "settings"
export type ProjectTab = "overview" | "graphics" | "launch" | "notes" | "activity"

export type Dialog =
  | { kind: "import" }
  | { kind: "uninstall"; repo: string }
  | { kind: "setup"; repo: string }
  | { kind: "saves"; repo: string }
  | { kind: "presets"; repo: string; choice: PlayChoice }
  | { kind: "notice" }
  | null

/** How many lines each job's log has: "" for the launcher's job, else a project's repo for its build. */
export type LogLengths = Record<string, number>

interface State {
  /** The launcher's snapshot, with every `log_lines` set to 0: the lengths are in `logs`. */
  snap: Snapshot | null
  logs: LogLengths
  view: View
  search: string
  tab: ProjectTab
  screenshot: boolean
  dialog: Dialog
  setView(view: View): void
  setSearch(search: string): void
  setTab(tab: ProjectTab): void
  setScreenshot(on: boolean): void
  open(dialog: Dialog): void
  refresh(): Promise<void>
}

export const useStore = create<State>((set, get) => ({
  snap: null,
  logs: {},
  view: "library",
  search: "",
  tab: "overview",
  screenshot: false,
  dialog: null,
  setView: (view) => set({ view }),
  setSearch: (search) => set({ search }),
  setTab: (tab) => set({ tab }),
  setScreenshot: (screenshot) => set({ screenshot }),
  open: (dialog) => set({ dialog }),
  refresh: async () => {
    const fresh = await (await getBackend()).snapshot()
    // The launcher's messages in the snapshot are in its language: the window follows it.
    setLocale(fresh.locale)
    // The logs' lengths grow with every line a build prints; kept apart, only the log on screen
    // follows them. Then parts that did not change keep their old objects, so what shows them does
    // not draw again; a snapshot with no other change (most of them, while a build prints) changes nothing.
    const logs = keepUnchanged(get().logs, takeLogLengths(fresh))
    const snap = keepUnchanged(get().snap, fresh)
    if (snap !== get().snap || logs !== get().logs) set({ snap, logs })
  },
}))

/** Moves every job's `log_lines` out of a fresh snapshot (setting it to 0 there). */
function takeLogLengths(snap: Snapshot): LogLengths {
  const logs: LogLengths = { "": snap.job.log_lines }
  snap.job.log_lines = 0
  for (const p of allProjects(snap)) {
    if (!p.build_job) continue
    logs[p.repo] = p.build_job.log_lines
    p.build_job.log_lines = 0
  }
  return logs
}

/**
 * `next`, with every part that equals the same part of `prev` replaced by
 * `prev`'s own object; `prev` itself when nothing changed. Snapshots are plain
 * JSON, so arrays, plain objects and primitives are all there is.
 */
export function keepUnchanged<T>(prev: T, next: T): T {
  if (Object.is(prev, next)) return prev
  if (typeof prev !== "object" || typeof next !== "object" || prev === null || next === null) return next
  if (Array.isArray(prev) !== Array.isArray(next)) return next
  if (Array.isArray(prev)) {
    const a = next as unknown[]
    let same = a.length === prev.length
    const out = a.map((item, i) => {
      const kept = i < prev.length ? keepUnchanged(prev[i], item) : item
      if (kept !== prev[i]) same = false
      return kept
    })
    return (same ? prev : out) as T
  }
  const p = prev as Record<string, unknown>
  const n = next as Record<string, unknown>
  const keys = Object.keys(n)
  let same = keys.length === Object.keys(p).length
  const out: Record<string, unknown> = {}
  for (const k of keys) {
    out[k] = k in p ? keepUnchanged(p[k], n[k]) : n[k]
    if (out[k] !== p[k]) same = false
  }
  return (same ? prev : out) as T
}

let queued = false
/** Asks for a new snapshot on the next frame, once however many changes came in. */
export function scheduleRefresh() {
  if (queued) return
  queued = true
  requestAnimationFrame(() => {
    queued = false
    void useStore.getState().refresh()
  })
}

/** Sends an action; a refusal shows as a toast and comes back as undefined. */
export async function run<T = unknown>(action: Action, success?: string): Promise<T | undefined> {
  try {
    const value = await send<T>(action)
    if (success) toast.success(success)
    scheduleRefresh()
    return value
  } catch (error) {
    toast.error(String(error))
    scheduleRefresh()
    return undefined
  }
}

/** The launcher's job as the window shows it (its log's length is in the store's `logs`). */
export type JobState = Omit<Snapshot["job"], "log_lines">

/** The current job, drawn again only when something shown about it changes. */
export function useJob(): JobState | null {
  return useStore(
    useShallow((s) => {
      if (!s.snap) return null
      const { log_lines: _, ...job } = s.snap.job
      return job
    }),
  )
}

export function allProjects(snap: Snapshot): Project[] {
  return [...snap.custom, ...snap.list.items]
}

export function useSelected(): Project | null {
  return useStore((s) => {
    if (!s.snap) return null
    const all = allProjects(s.snap)
    return all.find((p) => p.repo === s.snap!.selected) ?? null
  })
}

/** In Screenshot mode, names and links are examples and private text is hidden. */
export function useAlias() {
  const t = useT()
  // Only the projects' order, and only in Screenshot mode: a new snapshot alone draws nothing again.
  const order = useStore((s) => (s.screenshot && s.snap ? allProjects(s.snap).map((p) => p.repo).join("\n") : null))
  return (p: Project) => {
    if (order === null) return { name: p.name, repo: p.repo, owner: p.owner }
    const index = order.split("\n").indexOf(p.repo) + 1
    const n = String(index).padStart(2, "0")
    return { name: t("common.exampleProject", { n }), repo: `example/project-${n}`, owner: null }
  }
}
