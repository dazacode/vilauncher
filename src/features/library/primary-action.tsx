import { Download, Play, RefreshCw, Square, Wrench } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { formatList, t, Trans, useT } from "@/i18n"
import { shortCommit, shortStage } from "@/lib/format"
import { allProjects, run, useAlias, useStore } from "@/lib/store"
import type { PlayNext, Project, Snapshot } from "@/lib/types"
import { UpdateButton } from "./update-button"

export type ActionState =
  | { kind: "busy"; label: string }
  /** `other`: the name of the project the job is for; null when it isn't one in the list. */
  | { kind: "waiting"; other: string | null }
  | { kind: "setup"; reason: string }
  | { kind: "install" }
  | { kind: "update" }
  | { kind: "play" }
  | { kind: "stop" }
  | { kind: "nothing" }

/** What the main button does for `p` now: never off without a way forward. Texts are in the current language. */
export function actionState(p: Project, snap: Snapshot, name: (p: Project) => string): ActionState {
  const job = snap.job
  // While the game runs, the main button stops it, as Steam does. "Playing" is the stage a game run sets.
  if (job.running && job.repo === p.repo && job.stoppable && job.stage_kind === "playing") return { kind: "stop" }
  if (job.running && job.repo === p.repo) return { kind: "busy", label: shortStage(job.stage_kind, job.stage) }
  if (job.running) {
    const other = job.repo ? allProjects(snap).find((q) => q.repo === job.repo) : undefined
    return { kind: "waiting", other: other ? name(other) : null }
  }
  const needsTools = !p.installed && p.kind === "git" && !!snap.tools && !snap.tools.ready
  const needsFiles = !p.standalone && !snap.files.ok
  if (!p.installed) {
    if (needsTools) return { kind: "setup", reason: t("library.action.needsTools", { tools: formatList(snap.tools?.missing ?? []) }) }
    return p.kind === "git" ? { kind: "install" } : { kind: "nothing" }
  }
  if (p.update) return { kind: "update" }
  if (!p.runnable) return { kind: "nothing" }
  if (needsFiles) return { kind: "setup", reason: t("library.action.needsFiles") }
  return { kind: "play" }
}

export async function startPlay(repo: string) {
  const next = await run<PlayNext>({ type: "play", repo })
  if (!next) return
  const open = useStore.getState().open
  if (next.next === "presets") open({ kind: "presets", repo, choice: next.choice })
  else if (next.next === "saves") open({ kind: "saves", repo })
  else toast(t("library.action.starting"))
}

export function PrimaryAction({ project: p }: { project: Project }) {
  const t = useT()
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
          <Spinner className="text-white" /> {t("library.action.waiting")}
        </Button>
      )
    case "setup":
      return (
        <Button variant="primary" size="lg" onClick={() => open({ kind: "setup", repo: p.repo })}>
          <Wrench /> {t("library.action.setUp")}
        </Button>
      )
    case "install":
      return (
        <Button variant="primary" size="lg" onClick={() => void run({ type: "install", repo: p.repo })}>
          <Download /> {t("library.action.install")}
        </Button>
      )
    case "update":
      return (
        <UpdateButton repo={p.repo} variant="primary" size="lg">
          <RefreshCw /> {t("library.action.update")}
        </UpdateButton>
      )
    case "play":
      return (
        <Button variant="primary" size="lg" onClick={() => void startPlay(p.repo)}>
          <Play className="fill-current" /> {t("library.action.play")}
        </Button>
      )
    case "stop":
      return (
        <Button variant="danger" size="lg" onClick={() => void run({ type: "stop" })}>
          <Square /> {t("library.action.stop")}
        </Button>
      )
    case "nothing":
      return (
        <Button variant="secondary" size="lg" disabled>
          {t("library.action.built")}
        </Button>
      )
  }
}

/** One quiet line under the main button: what it will do, or what it waits for. */
export function ActionNote({ project: p }: { project: Project }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const setTab = useStore((s) => s.setTab)
  const alias = useAlias()
  const state = actionState(p, snap, (q) => alias(q).name)
  const link = (label: React.ReactNode, onClick: () => void) => (
    <button type="button" onClick={onClick} className="cursor-pointer font-medium text-fg-2 underline-offset-2 hover:text-fg hover:underline">
      {label}
    </button>
  )
  let text: React.ReactNode
  switch (state.kind) {
    case "busy":
      text = <>{snap.job.stage} · {link(t("library.note.viewLog"), () => setTab("activity"))}</>
      break
    case "waiting":
      text = state.other ? t("library.note.waitingFor", { name: state.other }) : t("library.note.waitingForJob")
      break
    case "setup":
      text = state.reason
      break
    case "install":
      text = t("library.note.install")
      break
    case "update":
      text = <Trans k="library.note.update" tags={{ link: (label) => link(label, () => void startPlay(p.repo)) }} />
      break
    case "play":
      text =
        snap.job.failed && snap.job.repo === p.repo ? (
          <>
            {snap.job.stage} · {link(t("library.fix"), () => void run({ type: "fix", repo: p.repo }))} · {link(t("library.note.viewLog"), () => setTab("activity"))}
          </>
        ) : p.activity ? (
          [p.activity.updated_ago, p.activity.last_run ? t("library.note.lastPlayed", { time: p.activity.last_run }) : p.activity.pending].filter(Boolean).join(" · ")
        ) : p.standalone ? (
          t("library.note.standalone")
        ) : (
          t("library.note.play")
        )
      break
    case "stop":
      text = <>{t("library.note.stop")} · {link(t("library.note.viewLog"), () => setTab("activity"))}</>
      break
    case "nothing":
      text = t("library.note.nothing")
      break
  }
  // Going back to the build before the last update: offered next to the update and play notes.
  const restore =
    p.previous_build !== null && (state.kind === "update" || state.kind === "play") ? (
      <p className="text-caption text-fg-3">
        {link(
          p.previous_build ? t("library.note.restoreCommit", { commit: shortCommit(p.previous_build)! }) : t("library.note.restore"),
          () => void run({ type: "rollback", repo: p.repo }),
        )}
      </p>
    ) : null
  return (
    <>
      <p className="text-body text-fg-3">{text}</p>
      {restore}
    </>
  )
}
