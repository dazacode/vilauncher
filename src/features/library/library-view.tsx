import { BadgeCheck, Copy, ExternalLink, FolderOpen, RefreshCw, RotateCcw, ShieldCheck, Trash2, Undo2, Wrench, X } from "lucide-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"
import { Aurora } from "@/components/art/aurora"
import { ProjectAvatar } from "@/components/art/project-avatar"
import { Bone } from "@/components/shell/skeleton"
import { Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "@/components/ui/menu"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip } from "@/components/ui/tooltip"
import { copyText, openUrl, pickFolder } from "@/lib/api"
import { useBackdrop } from "@/lib/backdrops"
import { fps, gpuText, shortCommit } from "@/lib/format"
import { allProjects, run, useAlias, useSelected, useStore } from "@/lib/store"
import type { Project, Snapshot } from "@/lib/types"
import { cn } from "@/lib/utils"
import { leadSentence } from "./hero"
import { actionState, startPlay } from "./primary-action"
import { ProjectTabs } from "./project-tabs"
import { useUpdateWarning } from "./update-button"

/**
 * The library: the selected project over the art, with the one action that matters, and the
 * build's details (overview, graphics, launch options, notes, activity) right under it.
 */
export function LibraryView() {
  const snap = useStore((s) => s.snap)!
  const project = useSelected()
  // Nothing picked: an installed project, else the best listed one.
  useEffect(() => {
    if (project) return
    const all = allProjects(snap)
    const pick = all.find((p) => p.installed) ?? snap.list.items[0] ?? all[0]
    if (pick) void run({ type: "select", repo: pick.repo })
  }, [project, snap])

  return (
    <Stage seed={project?.repo ?? "library"} below={project && <ProjectTabs project={project} />}>
      <GtaTitle />
      {project ? (
        <ProjectControls key={project.repo} project={project} />
      ) : snap.list.status === "error" ? (
        <ListError snap={snap} />
      ) : (
        <ControlsSkeleton />
      )}
    </Stage>
  )
}

const NONE: string[] = []

/**
 * The page frame: a banner of art (the player's own pictures when they set some) with the
 * project's column over its left side, then whatever goes below it, on the plain ground the
 * art fades into. The whole page scrolls under the nav.
 */
function Stage({ seed, below, children }: { seed: string; below?: React.ReactNode; children: React.ReactNode }) {
  const pictures = useStore((s) => (s.screenshot ? NONE : (s.snap?.backgrounds ?? NONE)))
  const playing = useStore((s) => !!s.snap?.job.running && s.snap.job.stage === "Playing")
  // Once the page scrolls, the nav and window controls get a ground of their own.
  const [scrolled, setScrolled] = useState(false)
  return (
    <div className="h-full overflow-x-hidden overflow-y-auto bg-[#0b0910]" onScroll={(e) => setScrolled(e.currentTarget.scrollTop > 24)}>
      <div
        aria-hidden
        className={cn(
          "pointer-events-none sticky top-0 z-20 -mb-[80px] h-[80px] border-b border-white/[0.06] bg-[#0b0910]/85 backdrop-blur-xl transition-opacity duration-200",
          scrolled ? "opacity-100" : "opacity-0",
        )}
      />
      <section className="relative isolate flex min-h-[clamp(560px,74vh,720px)] flex-col justify-end overflow-hidden">
        {/* The art is framed for the whole window (src/lib/backdrops.ts), so it keeps that frame and the banner shows its top. */}
        <div className="absolute inset-x-0 top-0 -z-10 h-[max(100%,100vh)] overflow-hidden bg-[#1a1230] [container-type:size]">
          {pictures.length ? <Aurora seed={seed} still={playing} pictures={pictures} art={null} /> : <GtaArt />}
        </div>
        {/* Into the page's ground, so the tabs read as the same page. */}
        <div className="pointer-events-none absolute inset-x-0 bottom-0 -z-10 h-[42%] bg-[linear-gradient(0deg,#0b0910_0%,rgb(11_9_16/0.82)_28%,rgb(11_9_16/0.35)_64%,transparent_100%)]" />
        <div className="w-[min(600px,calc(100%-96px))] px-[56px] pt-[96px] pb-[36px] box-content">{children}</div>
      </section>
      {below && <div id="library-details" className="scroll-mt-[80px] px-4">{below}</div>}
    </div>
  )
}

function GtaArt() {
  const b = useBackdrop()
  const vars = { "--art-aspect": b.aspect, "--art-anchor": b.anchor, "--art-target": b.target, "--art-ty": b.ty } as React.CSSProperties
  return (
    <>
      <img key={b.id} src={b.src} alt="" className="gta-art animate-[fade-in_400ms_ease-out]" style={vars} draggable={false} />
      {/* Darker on the left for the type, and toward the ground. */}
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(90deg,rgb(17_10_32/0.66)_0%,rgb(17_10_32/0.42)_38%,transparent_66%)]" />
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(0deg,rgb(13_9_24/0.55)_0%,rgb(13_9_24/0.22)_20%,transparent_38%)]" />
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[120px] bg-[linear-gradient(180deg,rgb(13_9_24/0.45),transparent)]" />
    </>
  )
}

/** The GTA VI logo, for the game every build on the list plays. */
function GtaTitle() {
  return <img src="/gta6/logo.svg" alt="Grand Theft Auto VI" className="h-[112px] w-[147px] drop-shadow-[0_6px_18px_rgb(30_10_50/0.45)]" draggable={false} />
}

/** Where the build comes from, who made it, its first sentence and what it measured. */
function Identity({ project: p }: { project: Project }) {
  const alias = useAlias()(p)
  const screenshot = useStore((s) => s.screenshot)
  const source = p.kind === "zip" ? "Imported build" : p.custom ? "Added by link" : "VIPortLive"
  const version = p.built ? `build ${shortCommit(p.built)}` : p.installed ? "installed" : "not built yet"
  const lead = screenshot ? null : leadSentence(p.summary)
  return (
    <div className="mt-[26px]">
      <p className="truncate text-[12px] leading-[15px] font-semibold tracking-[0.16em] text-white/65 uppercase [text-shadow:0_1px_6px_rgb(20_10_35/0.5)]">
        {source} · {version}
      </p>
      <div className="mt-[10px] flex items-center gap-[14px]">
        <ProjectAvatar project={p} owner={alias.owner} size={48} className="shadow-[0_8px_20px_rgb(0_0_0/0.45)]" />
        <h1 className="min-w-0 truncate font-display text-[40px] leading-[46px] font-bold tracking-[-0.03em] text-white [text-shadow:0_2px_20px_rgb(20_10_35/0.55)]">{alias.name}</h1>
        {p.verified && (
          <Tooltip label="Verified by VIPortLive">
            <BadgeCheck aria-label="Verified" tabIndex={0} className="size-6 shrink-0 text-accent-fg outline-none" />
          </Tooltip>
        )}
      </div>
      {lead && (
        <p className="mt-[14px] max-w-[460px] text-[15px] leading-[22px] text-pretty text-white/80 [text-shadow:0_1px_10px_rgb(20_10_35/0.7)]" data-selectable>
          {lead}
        </p>
      )}
      <Stats project={p} />
    </div>
  )
}

/** What the tracker measured, as a row of figures: only what is known. */
function Stats({ project: p }: { project: Project }) {
  const f = fps(p)
  const stats: { value: string; label: string; tip?: string }[] = []
  if (f != null) stats.push({ value: `${f} fps`, label: "Average" })
  if (p.render != null && (p.render > 0 || f != null))
    stats.push({ value: `${Math.round(p.render * 100)}%`, label: "Picture accuracy", tip: "How close the picture is to the console version, as rated on VIPortLive." })
  if (p.gpu.length) stats.push({ value: gpuText(p.gpu), label: "Made for" })
  if (!stats.length) return null
  return (
    <dl className="mt-[18px] flex flex-wrap items-stretch gap-y-3 [text-shadow:0_1px_10px_rgb(20_10_35/0.6)]">
      {stats.map((s, i) => {
        const body = (
          <div className={cn("flex flex-col-reverse", i > 0 && "border-l border-white/20 pl-[22px]", i < stats.length - 1 && "pr-[22px]", s.tip && "cursor-help rounded-xs")} tabIndex={s.tip ? 0 : undefined}>
            <dt className="mt-[2px] text-[12.5px] leading-4 text-white/60">{s.label}</dt>
            <dd className="font-display text-[22px] leading-[26px] font-semibold tracking-[-0.01em] text-white tabular-nums">{s.value}</dd>
          </div>
        )
        return s.tip ? <Tooltip key={s.label} label={s.tip}>{body}</Tooltip> : <div key={s.label} className="contents">{body}</div>
      })}
    </dl>
  )
}

type Tone = "amber" | "green" | "pink" | "red" | "grey"
const DOT: Record<Tone, string> = {
  amber: "bg-warn shadow-[0_0_8px_var(--color-warn)]",
  green: "bg-ok shadow-[0_0_8px_var(--color-ok)]",
  pink: "bg-accent shadow-[0_0_8px_var(--color-accent)] animate-pulse",
  red: "bg-bad shadow-[0_0_8px_var(--color-bad)]",
  grey: "bg-white/45",
}

const BIG = "h-[54px] w-[220px] rounded-lg text-[17px] font-bold tracking-[0.08em] uppercase"
/** The main action in VIPortLive pink. */
const BIG_PRIMARY =
  "cursor-pointer bg-accent text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.22),0_10px_30px_-8px_color-mix(in_srgb,var(--color-accent)_70%,transparent)] transition-[background-color,transform,opacity] duration-150 hover:bg-accent-hover active:translate-y-px disabled:cursor-default disabled:opacity-55 disabled:hover:bg-accent disabled:active:translate-y-0"

/** Who made it, the button and kebab, where it comes from, its status and the downloads folder. */
function ProjectControls({ project: p }: { project: Project }) {
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const setTab = useStore((s) => s.setTab)
  const alias = useAlias()
  const state = actionState(p, snap, (q) => alias(q).name)
  const warning = useUpdateWarning(p.repo)
  const showLog = () => {
    setTab("activity")
    document.getElementById("library-details")?.scrollIntoView({ behavior: "smooth", block: "start" })
  }
  const link = (label: string, onClick: () => void) => (
    <button type="button" onClick={onClick} className="cursor-pointer text-accent-fg underline-offset-2 hover:underline">
      {label}
    </button>
  )
  const failed = snap.job.failed && snap.job.repo === p.repo

  let label: React.ReactNode
  let onClick: (() => void) | undefined
  let tone: Tone
  let status: React.ReactNode
  switch (state.kind) {
    case "install":
      label = "Download"
      onClick = () => void run({ type: "install", repo: p.repo })
      tone = "amber"
      status = p.kind === "git" ? "Not installed · download the build to play" : "Not installed"
      break
    case "busy":
      label = (
        <span className="flex items-center justify-center gap-3">
          <Spinner className="size-5" /> {state.label}
        </span>
      )
      tone = "pink"
      status = <>{snap.job.stage} · {link("View log", showLog)}</>
      break
    case "waiting":
      label = "Waiting"
      tone = "amber"
      status = `Waiting for ${state.other} to finish`
      break
    case "setup":
      label = "Set up"
      onClick = () => open({ kind: "setup", repo: p.repo })
      tone = "amber"
      status = state.reason
      break
    case "update":
      label = "Update"
      onClick = warning.ask
      tone = "amber"
      status = <>Update available · {link("play this version", () => void startPlay(p.repo))}</>
      break
    case "play":
      label = "Play"
      onClick = () => void startPlay(p.repo)
      tone = failed ? "red" : "green"
      status = failed ? (
        <>{snap.job.stage} · {link("Fix and verify", () => void run({ type: "fix", repo: p.repo }))} · {link("View log", showLog)}</>
      ) : p.activity?.last_run ? (
        `Installed · last played ${p.activity.last_run}`
      ) : (
        "Installed · ready to play"
      )
      break
    case "stop":
      label = "Stop"
      onClick = () => void run({ type: "stop" })
      tone = "green"
      status = <>Playing · {link("View log", showLog)}</>
      break
    case "nothing":
      label = "Built"
      tone = "grey"
      status = "Built · this workspace has no program to run"
      break
  }

  return (
    <>
      <Identity project={p} />
      <div className="mt-[26px] flex items-center gap-[12px]">
        <button
          type="button"
          disabled={!onClick}
          aria-busy={state.kind === "busy" || undefined}
          onClick={onClick}
          className={cn(BIG, BIG_PRIMARY, state.kind === "stop" && "bg-bad shadow-none hover:bg-bad/90")}
        >
          {label}
        </button>
        <Kebab project={p} />
      </div>

      <p className="mt-[16px] flex min-h-[18px] items-start gap-[9px] text-[14.5px] leading-[18px] font-bold text-white [text-shadow:0_1px_8px_rgb(20_10_35/0.6)]">
        <span className={cn("mt-[5.5px] size-[7px] shrink-0 rounded-full", DOT[tone])} aria-hidden />
        <span role="status" className="min-w-0">{status}</span>
      </p>

      <DownloadsLine />
      {warning.dialog}
    </>
  )
}

/** Where builds download to, with Change and the way in for a copy the player already has. */
function DownloadsLine() {
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  const path = screenshot ? "C:\\Games\\VILauncher" : snap.settings.workspace
  const busy = snap.job.running
  return (
    <p className="mt-[14px] flex items-center gap-x-[16px] text-[13px] leading-[18px] whitespace-nowrap text-white/70 [text-shadow:0_1px_6px_rgb(20_10_35/0.6)]">
      <span>Downloads to</span>
      <Tooltip label={<span className="font-mono break-all">{path}</span>}>
        <span tabIndex={0} className="-ml-[10px] inline-block max-w-[150px] cursor-default truncate rounded-[2px] align-bottom" aria-label={`Download folder: ${path}`}>
          {path}
        </span>
      </Tooltip>
      <button
        type="button"
        disabled={busy}
        title={busy ? "Wait for the current job to finish" : undefined}
        onClick={async () => {
          const dir = await pickFolder("Where builds are downloaded")
          if (dir) void run({ type: "choose_install_dir", path: dir })
        }}
        className="cursor-pointer rounded-[2px] text-accent-fg underline-offset-2 hover:underline disabled:cursor-default disabled:opacity-50 disabled:hover:no-underline"
      >
        Change
      </button>
      <button type="button" onClick={() => open({ kind: "import" })} className="cursor-pointer rounded-[2px] text-accent-fg underline-offset-2 hover:underline">
        Already have it?
      </button>
    </p>
  )
}

/** The square button beside the main one: everything else the project can do. */
function Kebab({ project: p }: { project: Project }) {
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  const busy = snap.job.running
  const warning = useUpdateWarning(p.repo)
  return (
    <Menu>
      <MenuTrigger asChild>
        <button
          type="button"
          aria-label="More options"
          className="grid size-[54px] cursor-pointer place-items-center rounded-lg border border-white/15 bg-black/55 text-white shadow-[0_2px_10px_rgb(0_0_0/0.3)] backdrop-blur-md transition-colors hover:bg-black/70 data-[state=open]:border-accent/60 data-[state=open]:bg-black/70"
        >
          <svg viewBox="0 0 6 26" className="h-[26px] w-[6px]" fill="currentColor" aria-hidden>
            <circle cx="3" cy="3" r="2.7" />
            <circle cx="3" cy="13" r="2.7" />
            <circle cx="3" cy="23" r="2.7" />
          </svg>
        </button>
      </MenuTrigger>
      <MenuContent className="font-sans">
        {p.installed && p.kind === "git" && (
          <MenuItem disabled={p.checking} onSelect={() => void run({ type: "check_repo", repo: p.repo })}>
            <RefreshCw /> Check for updates
          </MenuItem>
        )}
        {p.installed && (
          <MenuItem disabled={busy} onSelect={() => void run({ type: "fix", repo: p.repo })}>
            <ShieldCheck /> Fix and verify
          </MenuItem>
        )}
        {p.installed && !p.standalone && p.kind !== "local" && (
          <MenuItem disabled={busy || !snap.files.ok} onSelect={() => void run({ type: "warm", repo: p.repo })}>
            <Wrench /> Warm up shaders
          </MenuItem>
        )}
        {p.previous_build !== null && (
          <MenuItem disabled={busy} onSelect={() => void run({ type: "rollback", repo: p.repo })}>
            <Undo2 /> {p.previous_build ? `Restore previous build (${shortCommit(p.previous_build)})` : "Restore previous build"}
          </MenuItem>
        )}
        {p.installed && p.kind === "git" && (
          <MenuItem disabled={busy} onSelect={warning.ask}>
            <RotateCcw /> Reinstall
          </MenuItem>
        )}
        {!screenshot && (
          <MenuItem disabled={!p.installed} onSelect={() => void run({ type: "open", what: "project_folder", repo: p.repo })}>
            <FolderOpen /> Show in folder
          </MenuItem>
        )}
        {p.web_url && !screenshot && (
          <>
            <MenuItem onSelect={() => void openUrl(p.web_url!).catch((e) => toast.error(String(e)))}>
              <ExternalLink /> Open repository
            </MenuItem>
            <MenuItem onSelect={() => copyText(p.web_url!).then(() => toast.success("Link copied")).catch((e) => toast.error(String(e)))}>
              <Copy /> Copy link
            </MenuItem>
          </>
        )}
        {(p.installed && p.kind !== "local") || p.custom ? <MenuSeparator /> : null}
        {p.installed && p.kind !== "local" && (
          <MenuItem tone="danger" disabled={busy} onSelect={() => open({ kind: "uninstall", repo: p.repo })}>
            <Trash2 /> Uninstall…
          </MenuItem>
        )}
        {p.custom && p.kind === "git" && (
          <MenuItem tone="danger" onSelect={() => void run({ type: "remove_custom", repo: p.repo }, "Removed from your library")}>
            <X /> Remove from library
          </MenuItem>
        )}
      </MenuContent>
      {warning.dialog}
    </Menu>
  )
}

function ListError({ snap }: { snap: Snapshot }) {
  const screenshot = useStore((s) => s.screenshot)
  return (
    <>
      <h1 className="mt-[22px] font-display text-[28px] leading-[34px] font-bold tracking-[-0.02em] text-white">The project list didn't arrive</h1>
      <div className="mt-[20px]">
        <button type="button" onClick={() => void run({ type: "refresh_projects" })} className={cn(BIG, BIG_PRIMARY)}>
          Try again
        </button>
      </div>
      <p className="mt-[18px] text-[12px] leading-[15px] font-semibold tracking-[0.16em] text-white/65 uppercase">viportlive.com · project list</p>
      <p className="mt-[7px] flex items-start gap-[9px] text-[14px] leading-[18px] font-bold text-white">
        <span className={cn("mt-[5.5px] size-[7px] shrink-0 rounded-full", DOT.red)} aria-hidden />
        <span role="alert">VILauncher reads the list of community builds from viportlive.com. Check your connection, then try again.</span>
      </p>
      {!screenshot && snap.list.error && (
        <p className="mt-2 text-[12px] leading-4 text-white/65" data-selectable>
          {snap.list.error}
        </p>
      )}
    </>
  )
}

function ControlsSkeleton() {
  return (
    <div aria-busy="true" aria-label="Loading builds">
      <Bone className="mt-[26px] h-[12px] w-[180px]" />
      <div className="mt-[10px] flex items-center gap-[14px]">
        <Bone className="size-12 rounded-[12px]" />
        <Bone className="h-9 w-[240px] rounded-md" />
      </div>
      <Bone className="mt-[14px] h-[14px] w-[480px] max-w-full" />
      <Bone className="mt-[8px] h-[14px] w-[320px]" />
      <div className="mt-[18px] flex gap-[22px]">
        {[64, 52, 56].map((w) => (
          <div key={w} className="space-y-2">
            <Bone className="h-5" style={{ width: w }} />
            <Bone className="h-3 w-14" />
          </div>
        ))}
      </div>
      <div className="mt-[26px] flex gap-[12px]">
        <Bone className="h-[54px] w-[220px] rounded-lg" />
        <Bone className="size-[54px] rounded-lg" />
      </div>
      <Bone className="mt-[16px] h-[14px] w-[280px]" />
      <Bone className="mt-[14px] h-[14px] w-[250px]" />
    </div>
  )
}

/** The library before the first snapshot: the art and logo at once, the controls as bones. */
export function LibrarySkeleton() {
  return (
    <Stage seed="library">
      <GtaTitle />
      <ControlsSkeleton />
    </Stage>
  )
}
