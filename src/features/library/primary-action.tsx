import { Download, Play, RefreshCw, Wrench } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { shortStage } from "@/lib/format"
import { allProjects, run, useAlias, useStore } from "@/lib/store"
import type { PlayNext, Project, Snapshot } from "@/lib/types"

export type ActionState =
  | { kind: "busy"; label: string }
  | { kind: "waiting"; other: string }
  | { kind: "setup"; reason: string }
  | { kind: "install" }
  | { kind: "update" }
  | { kind: "play" }
  | { kind: "nothing" }

/** What the main button does for `p` now: never off without a way forward. */
export function actionState(p: Project, snap: Snapshot, name: (p: Project) => string): ActionState {
  const job = snap.job
  if (job.running && job.repo === p.repo) return { kind: "busy", label: shortStage(job.stage) }
  if (job.running) {
    const other = job.repo ? allProjects(snap).find((q) => q.repo === job.repo) : undefined
    return { kind: "waiting", other: other ? name(other) : "the current job" }
  }
  const needsTools = !p.installed && p.kind === "git" && !!snap.tools && !snap.tools.ready
  const needsFiles = !p.standalone && !snap.files.ok
  if (!p.installed) {
    if (needsTools) return { kind: "setup", reason: `Installing needs ${snap.tools?.missing.join(", ")}.` }
    return p.kind === "git" ? { kind: "install" } : { kind: "nothing" }
  }
  if (p.update) return { kind: "update" }
  if (!p.runnable) return { kind: "nothing" }
  if (needsFiles) return { kind: "setup", reason: "Playing needs your own game files." }
  return { kind: "play" }
}

export async function startPlay(repo: string) {
  const next = await run<PlayNext>({ type: "play", repo })
  if (!next) return
  const open = useStore.getState().open
  if (next.next === "presets") open({ kind: "presets", repo, choice: next.choice })
  else if (next.next === "saves") open({ kind: "saves", repo })
  else toast("Starting…")
}

export function PrimaryAction({ project: p }: { project: Project }) {
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const alias = useAlias()
  const state = actionState(p, snap, (q) => alias(q).name)
  switch (state.kind) {
    case "busy":
      return (
        <Button variant="primary" size="lg" aria-busy aria-disabled tabIndex={-1} className="pointer-events-none relative overflow-hidden">
          <span className="absolute inset-0 animate-[busy_1.6s_linear_infinite] bg-[linear-gradient(110deg,transparent_30%,rgb(255_255_255/0.18)_50%,transparent_70%)] bg-[length:200%_100%]" />
          <Spinner className="relative text-white" /> <span className="relative">{state.label}</span>
        </Button>
      )
    case "waiting":
      return (
        <Button variant="primary" size="lg" disabled>
          <Spinner className="text-white" /> Waiting
        </Button>
      )
    case "setup":
      return (
        <Button variant="primary" size="lg" onClick={() => open({ kind: "setup", repo: p.repo })}>
          <Wrench /> Set up
        </Button>
      )
    case "install":
      return (
        <Button variant="primary" size="lg" onClick={() => void run({ type: "install", repo: p.repo })}>
          <Download /> Install
        </Button>
      )
    case "update":
      return (
        <Button variant="primary" size="lg" onClick={() => void run({ type: "install", repo: p.repo })}>
          <RefreshCw /> Update
        </Button>
      )
    case "play":
      return (
        <Button variant="primary" size="lg" onClick={() => void startPlay(p.repo)}>
          <Play className="fill-current" /> Play
        </Button>
      )
    case "nothing":
      return (
        <Button variant="secondary" size="lg" disabled>
          Built
        </Button>
      )
  }
}

/** One quiet line under the main button: what it will do, or what it waits for. */
export function ActionNote({ project: p }: { project: Project }) {
  const snap = useStore((s) => s.snap)!
  const setTab = useStore((s) => s.setTab)
  const alias = useAlias()
  const state = actionState(p, snap, (q) => alias(q).name)
  const link = (label: string, onClick: () => void) => (
    <button type="button" onClick={onClick} className="cursor-pointer font-medium text-fg-2 underline-offset-2 hover:text-fg hover:underline">
      {label}
    </button>
  )
  let text: React.ReactNode
  switch (state.kind) {
    case "busy":
      text = <>{snap.job.stage} · {link("View log", () => setTab("activity"))}</>
      break
    case "waiting":
      text = <>Waiting for {state.other} to finish.</>
      break
    case "setup":
      text = state.reason
      break
    case "install":
      text = "Downloads and builds it on this PC once. The first build takes several minutes."
      break
    case "update":
      text = <>A new version is out. {link("Play this version instead", () => void startPlay(p.repo))}</>
      break
    case "play":
      text = p.activity ? [p.activity.updated_ago, p.activity.last_run ? `Last played ${p.activity.last_run}` : p.activity.pending].filter(Boolean).join(" · ") : p.standalone ? "Starts the installed program." : "Starts your installed build with your own game files."
      break
    case "nothing":
      text = "This Rust workspace has no program to run."
      break
  }
  return <p className="text-body text-fg-3">{text}</p>
}
