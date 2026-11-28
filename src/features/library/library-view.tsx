import { BadgeCheck, ChevronDown, Copy, ExternalLink, FolderOpen, RefreshCw, RotateCcw, ShieldCheck, Trash2, Undo2, Wrench, X } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { toast } from "sonner"
import { Aurora } from "@/components/art/aurora"
import { ProjectAvatar } from "@/components/art/project-avatar"
import { Bone } from "@/components/shell/skeleton"
import { Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "@/components/ui/menu"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip } from "@/components/ui/tooltip"
import { Trans, useT } from "@/i18n"
import { copyText, openUrl, pickFolder } from "@/lib/api"
import { useBackdrop } from "@/lib/backdrops"
import { fps, gpuText, hideHome, shortCommit } from "@/lib/format"
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
 * The page frame: the art fills the page and stays put, with the project's column over its left
 * side. Scrolling moves the column up with the details sheet below it, so both stay readable,
 * while the art dims behind them. A "Details" hint at the bottom says there is more.
 */
function Stage({ seed, below, children }: { seed: string; below?: React.ReactNode; children: React.ReactNode }) {
  const t = useT()
  const pictures = useStore((s) => (s.screenshot ? NONE : (s.snap?.backgrounds ?? NONE)))
  const playing = useStore((s) => !!s.snap?.job.running && s.snap.job.stage_kind === "playing")
  // Once the page scrolls, the nav and window controls get a ground of their own.
  const [scrolled, setScrolled] = useState(false)
  const column = useRef<HTMLDivElement>(null)
  // The sheet settles like a drawer: let go partway and it glides open (or shut, going back up).
  const settle = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const last = useRef({ top: 0, down: true })
  useEffect(() => () => clearTimeout(settle.current), [])
  // Set on the page straight from the scroll, not through React: 0 at rest, 1 a page down.
  const onScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const box = e.currentTarget
    box.style.setProperty("--cover", String(Math.min(1, box.scrollTop / Math.max(1, box.clientHeight))))
    // The hero shrinks and fades as the page scrolls: the logo, summary and downloads line first, then the
    // whole column. Once it is mostly gone it takes no clicks or focus, so nothing unseen can be pressed.
    const squish = Math.min(1, box.scrollTop / Math.max(1, box.clientHeight * 0.8))
    box.style.setProperty("--squish", String(squish))
    if (column.current) column.current.inert = squish > 0.45
    const top = box.scrollTop
    if (top !== last.current.top) last.current = { top, down: top > last.current.top }
    clearTimeout(settle.current)
    settle.current = setTimeout(() => {
      const sheet = document.getElementById("library-details")
      if (!sheet) return
      const open = sheet.offsetTop - 80
      const now = box.scrollTop
      if (now <= 0 || now >= open - 1) return
      // A small pull opens it; going back up, a small push closes it.
      const opening = last.current.down ? now > open * 0.12 : now > open * 0.88
      const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches
      box.scrollTo({ top: opening ? open : 0, behavior: still ? "auto" : "smooth" })
    }, 140)
    setScrolled(box.scrollTop > 24)
  }
  return (
    <div
      className="h-full overflow-x-hidden overflow-y-auto bg-page [--cover:0] [--squish:0] [container-type:size] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      onScroll={onScroll}
    >
      <div
        aria-hidden
        className={cn(
          "pointer-events-none sticky top-0 z-20 -mb-[80px] h-[80px] border-b border-line bg-ground/80 backdrop-blur-xl transition-opacity duration-200",
          scrolled ? "opacity-100" : "opacity-0",
        )}
      />
      {/* The art, pinned to the page: framed for the whole window (src/lib/backdrops.ts). */}
      <div aria-hidden className="sticky top-0 -mb-[100cqh] h-[100cqh] overflow-hidden">
        <div className="absolute inset-0 origin-center bg-[#1a1230] [container-type:size] [transform:translateY(calc(var(--cover)*-48px))_scale(calc(1+var(--cover)*0.08))]">
          {pictures.length ? <Aurora seed={seed} still={playing} pictures={pictures} art={null} /> : <GtaArt />}
        </div>
        <div className="absolute inset-0 bg-ground opacity-[calc(var(--cover)*0.6)]" />
      </div>
      <section className="relative flex min-h-[max(620px,100cqh)] flex-col justify-end">
        <div
          ref={column}
          className="box-content w-[min(600px,calc(100%-96px))] origin-bottom-left px-[56px] pt-[96px] opacity-[calc(1-var(--squish)*1.6)] [transform:scale(calc(1-var(--squish)*0.25))]"
          style={{ paddingBottom: below ? "calc(96px - var(--squish) * 64px)" : 48 }}
        >
          {children}
        </div>
        {below && (
          <button
            type="button"
            onClick={showDetails}
            className="absolute bottom-[22px] left-1/2 flex -translate-x-1/2 cursor-pointer flex-col items-center gap-[2px] rounded-md px-3 py-1 text-[11px] leading-4 font-semibold tracking-[0.16em] text-white/55 uppercase opacity-[calc(1-var(--cover)*5)] transition-colors [text-shadow:0_1px_8px_rgb(20_10_35/0.6)] hover:text-white"
          >
            {t("library.hero.details")}
            <ChevronDown aria-hidden className="size-4 animate-[hint-nudge_2.4s_ease-in-out_infinite] motion-reduce:animate-none" />
          </button>
        )}
      </section>
      {below && (
        <div
          id="library-details"
          className="relative z-10 mx-[calc(max(0,1-var(--cover)*1.1)*14px)] min-h-[calc(100cqh-80px)] scroll-mt-[80px] rounded-t-[calc(max(0,1-var(--cover)*1.1)*24px)] border-t border-line bg-page px-4 pt-[18px] shadow-[0_-24px_60px_rgb(0_0_0/0.5)] backdrop-blur-2xl"
        >
          {below}
        </div>
      )}
    </div>
  )
}

function showDetails() {
  document.getElementById("library-details")?.scrollIntoView({ behavior: "smooth", block: "start" })
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
  return (
    <div className="opacity-[calc(1-var(--squish,0)*2.2)]">
      {/* i18n-ignore: the game's name */}
      <img src="/gta6/logo.svg" alt="Grand Theft Auto VI" className="h-[76px] w-[100px] drop-shadow-[0_6px_18px_rgb(30_10_50/0.45)]" draggable={false} />
    </div>
  )
}

/** The build's name, who made it, and its first sentence: what the player reads first. */
function Identity({ project: p }: { project: Project }) {
  const t = useT()
  const alias = useAlias()(p)
  const screenshot = useStore((s) => s.screenshot)
  const lead = screenshot ? null : leadSentence(p.summary)
  return (
    <div className="mt-[24px]">
      <div className="flex items-center gap-[14px]">
        <ProjectAvatar project={p} owner={alias.owner} size={48} className="shadow-[0_8px_20px_rgb(0_0_0/0.45)]" />
        <h1 className="min-w-0 truncate font-display text-[40px] leading-[46px] font-bold tracking-[-0.03em] text-white [text-shadow:0_2px_20px_rgb(20_10_35/0.55)]">{alias.name}</h1>
        {p.verified && (
          <Tooltip label={t("library.hero.verifiedBy")}>
            <BadgeCheck aria-label={t("library.hero.verified")} tabIndex={0} className="size-6 shrink-0 text-accent-art outline-none" />
          </Tooltip>
        )}
      </div>
      {lead && (
        <p
          className="mt-[14px] max-w-[460px] text-[15px] leading-[22px] text-pretty text-white/80 opacity-[calc(1-var(--squish,0)*2.2)] [text-shadow:0_1px_10px_rgb(20_10_35/0.7)]"
          data-selectable
        >
          {lead}
        </p>
      )}
    </div>
  )
}

/** What the tracker measured, as a row of figures: only what is known. */
function Stats({ project: p }: { project: Project }) {
  const t = useT()
  const f = fps(p)
  const stats: { value: string; label: string; tip?: string }[] = []
  if (p.playtime) stats.push({ value: p.playtime, label: t("library.hero.playtime") })
  if (p.personal_fps != null) stats.push({ value: `${p.personal_fps.toFixed(1)} fps`, label: t("library.hero.personalFps") })
  if (f != null) stats.push({ value: `${f} fps`, label: t("library.hero.average") })
  if (p.render != null && (p.render > 0 || f != null))
    stats.push({ value: `${Math.round(p.render * 100)}%`, label: t("library.hero.accuracy"), tip: t("library.hero.accuracyTip") })
  if (p.gpu.length) stats.push({ value: gpuText(p.gpu), label: t("library.hero.madeFor") })
  if (!stats.length) return null
  return (
    <dl className="mt-[28px] flex flex-wrap items-stretch gap-y-3 [text-shadow:0_1px_10px_rgb(20_10_35/0.6)]">
      {stats.map((s, i) => {
        const body = (
          <div className={cn("flex flex-col-reverse", i > 0 && "border-l border-white/20 pl-[22px]", i < stats.length - 1 && "pr-[22px]", s.tip && "cursor-help rounded-xs")} tabIndex={s.tip ? 0 : undefined}>
            <dt className="mt-[2px] text-[12.5px] leading-4 text-white/60">{s.label}</dt>
            <dd className="font-display text-[20px] leading-[24px] font-semibold tracking-[-0.01em] text-white tabular-nums">{s.value}</dd>
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
  const t = useT()
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
  const link = (label: React.ReactNode, onClick: () => void) => (
    <button type="button" onClick={onClick} className="cursor-pointer text-accent-art underline-offset-2 hover:underline">
      {label}
    </button>
  )
  const job = p.build_job ?? (snap.job.repo === p.repo ? snap.job : null)
  const failed = !!job?.failed

  let label: React.ReactNode
  let onClick: (() => void) | undefined
  let tone: Tone
  let status: React.ReactNode
  switch (state.kind) {
    case "install":
      label = t("library.action.download")
      onClick = () => void run({ type: "install", repo: p.repo })
      tone = "amber"
      status = p.kind === "git" ? t("library.status.notInstalledGit") : t("library.status.notInstalled")
      break
    case "busy":
      label = (
        <span className="flex items-center justify-center gap-3">
          <Spinner className="size-5" /> {state.label}
        </span>
      )
      tone = "pink"
      status = <>{job?.stage ?? snap.job.stage} · {link(t("library.note.viewLog"), showLog)}</>
      break
    case "waiting":
      label = t("library.action.waiting")
      tone = "amber"
      status = state.other ? t("library.note.waitingFor", { name: state.other }) : t("library.note.waitingForJob")
      break
    case "setup":
      label = t("library.action.setUp")
      onClick = () => open({ kind: "setup", repo: p.repo })
      tone = "amber"
      status = state.reason
      break
    case "update":
      label = t("library.action.update")
      onClick = warning.ask
      tone = "amber"
      status = <Trans k="library.status.update" tags={{ link: (s) => link(s, () => void startPlay(p.repo)) }} />
      break
    case "play":
      label = t("library.action.play")
      onClick = () => void startPlay(p.repo)
      tone = failed ? "red" : "green"
      status = failed ? (
        <>{job?.stage ?? snap.job.stage} · {link(t("library.fix"), () => void run({ type: "fix", repo: p.repo }))} · {link(t("library.note.viewLog"), showLog)}</>
      ) : p.activity?.last_run ? (
        t("library.status.lastPlayed", { time: p.activity.last_run })
      ) : (
        t("library.status.ready")
      )
      break
    case "stop":
      label = t("library.action.stop")
      onClick = () => void run({ type: "stop" })
      tone = "green"
      status = <>{t("library.status.playing")} · {link(t("library.note.viewLog"), showLog)}</>
      break
    case "nothing":
      label = t("library.action.built")
      tone = "grey"
      status = t("library.status.nothing")
      break
  }

  return (
    <>
      <Identity project={p} />
      <div className="mt-[24px] flex items-center gap-[12px]">
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

      <Stats project={p} />
      <DownloadsLine project={p} />
      {warning.dialog}
    </>
  )
}

/**
 * The quiet last line: where the build comes from (and its commit once built), where builds
 * download to, Change, and the way in for a copy the player already has.
 */
function DownloadsLine({ project: p }: { project: Project }) {
  const t = useT()
  const source = p.kind === "zip" ? t("library.hero.imported") : p.custom ? t("library.hero.addedByLink") : "VIPortLive"
  const commit = p.built ? shortCommit(p.built) : null
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  // The player's account name never shows: the home folder reads "/user/...".
  const path = screenshot ? "C:\\Games\\VILauncher" : hideHome(snap.settings.workspace)
  const busy = snap.job.running
  return (
    <p className="mt-[22px] flex flex-wrap items-center gap-x-[16px] gap-y-1 text-[13px] leading-[18px] whitespace-nowrap text-white/60 opacity-[calc(1-var(--squish,0)*2.2)] [text-shadow:0_1px_6px_rgb(20_10_35/0.6)]">
      <span>{commit ? `${source} · ${t("library.identity.build", { commit })}` : source}</span>
      <span aria-hidden className="-mx-[6px] text-white/30">·</span>
      <span>{t("library.downloads.to")}</span>
      <Tooltip label={<span className="font-mono break-all">{path}</span>}>
        <span tabIndex={0} className="-ml-[10px] inline-block max-w-[150px] cursor-default truncate rounded-[2px] align-bottom" aria-label={t("library.downloads.folder", { path })}>
          {path}
        </span>
      </Tooltip>
      <button
        type="button"
        disabled={busy}
        title={busy ? t("library.downloads.wait") : undefined}
        onClick={async () => {
          const dir = await pickFolder(t("library.downloads.pickTitle"))
          if (dir) void run({ type: "choose_install_dir", path: dir })
        }}
        className="cursor-pointer rounded-[2px] text-accent-art underline-offset-2 hover:underline disabled:cursor-default disabled:opacity-50 disabled:hover:no-underline"
      >
        {t("library.downloads.change")}
      </button>
      <button type="button" onClick={() => open({ kind: "import" })} className="cursor-pointer rounded-[2px] text-accent-art underline-offset-2 hover:underline">
        {t("library.downloads.have")}
      </button>
    </p>
  )
}

/** The square button beside the main one: everything else the project can do. */
function Kebab({ project: p }: { project: Project }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  const busy = snap.job.running || snap.active_builds > 0
  const warning = useUpdateWarning(p.repo)
  return (
    <Menu>
      <MenuTrigger asChild>
        <button
          type="button"
          aria-label={t("library.hero.more")}
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
        {p.playtime && (
          <>
            <MenuItem disabled={busy} onSelect={() => void run({ type: "reset_personal_fps", repo: p.repo })}>
              {t("library.hero.resetPersonalFps")}
            </MenuItem>
            <MenuItem onSelect={() => void run({ type: "set_include_exact60_fps", include: !snap.settings.include_exact_60_fps })}>
              {t(snap.settings.include_exact_60_fps ? "library.hero.exclude60Fps" : "library.hero.include60Fps")}
            </MenuItem>
            <MenuSeparator />
          </>
        )}
        {p.installed && p.kind === "git" && (
          <MenuItem disabled={p.checking} onSelect={() => void run({ type: "check_repo", repo: p.repo })}>
            <RefreshCw /> {t("library.menu.checkUpdates")}
          </MenuItem>
        )}
        {p.installed && (
          <MenuItem disabled={busy} onSelect={() => void run({ type: "fix", repo: p.repo })}>
            <ShieldCheck /> {t("library.fix")}
          </MenuItem>
        )}
        {p.installed && !p.standalone && p.kind !== "local" && (
          <MenuItem disabled={busy || !snap.files.ok} onSelect={() => void run({ type: "warm", repo: p.repo })}>
            <Wrench /> {t("library.menu.warm")}
          </MenuItem>
        )}
        {p.previous_build !== null && (
          <MenuItem disabled={busy} onSelect={() => void run({ type: "rollback", repo: p.repo })}>
            <Undo2 /> {p.previous_build ? t("library.note.restoreCommit", { commit: shortCommit(p.previous_build)! }) : t("library.note.restore")}
          </MenuItem>
        )}
        {p.installed && p.kind === "git" && (
          <MenuItem disabled={snap.job.running || !!p.build_job?.running} onSelect={warning.ask}>
            <RotateCcw /> {t("library.menu.reinstall")}
          </MenuItem>
        )}
        {!screenshot && (
          <MenuItem disabled={!p.installed} onSelect={() => void run({ type: "open", what: "project_folder", repo: p.repo })}>
            <FolderOpen /> {t("library.menu.showInFolder")}
          </MenuItem>
        )}
        {p.web_url && !screenshot && (
          <>
            <MenuItem onSelect={() => void openUrl(p.web_url!).catch((e) => toast.error(String(e)))}>
              <ExternalLink /> {t("library.hero.openRepo")}
            </MenuItem>
            <MenuItem onSelect={() => copyText(p.web_url!).then(() => toast.success(t("library.menu.linkCopied"))).catch((e) => toast.error(String(e)))}>
              <Copy /> {t("library.menu.copyLink")}
            </MenuItem>
          </>
        )}
        {(p.installed && p.kind !== "local") || p.custom ? <MenuSeparator /> : null}
        {p.installed && p.kind !== "local" && (
          <MenuItem tone="danger" disabled={busy} onSelect={() => open({ kind: "uninstall", repo: p.repo })}>
            <Trash2 /> {t("library.menu.uninstall")}
          </MenuItem>
        )}
        {p.custom && p.kind === "git" && (
          <MenuItem tone="danger" onSelect={() => void run({ type: "remove_custom", repo: p.repo }, t("library.menu.removed"))}>
            <X /> {t("library.menu.remove")}
          </MenuItem>
        )}
      </MenuContent>
      {warning.dialog}
    </Menu>
  )
}

function ListError({ snap }: { snap: Snapshot }) {
  const t = useT()
  const screenshot = useStore((s) => s.screenshot)
  return (
    <>
      <h1 className="mt-[22px] font-display text-[28px] leading-[34px] font-bold tracking-[-0.02em] text-white">{t("library.listError.title")}</h1>
      <div className="mt-[20px]">
        <button type="button" onClick={() => void run({ type: "refresh_projects" })} className={cn(BIG, BIG_PRIMARY)}>
          {t("common.retry")}
        </button>
      </div>
      <p className="mt-[18px] text-[12px] leading-[15px] font-semibold tracking-[0.16em] text-white/65 uppercase">{t("library.listError.source")}</p>
      <p className="mt-[7px] flex items-start gap-[9px] text-[14px] leading-[18px] font-bold text-white">
        <span className={cn("mt-[5.5px] size-[7px] shrink-0 rounded-full", DOT.red)} aria-hidden />
        <span role="alert">{t("library.listError.body")}</span>
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
  const t = useT()
  return (
    <div aria-busy="true" aria-label={t("library.loading")}>
      <div className="mt-[24px] flex items-center gap-[14px]">
        <Bone className="size-12 rounded-[12px]" />
        <Bone className="h-9 w-[240px] rounded-md" />
      </div>
      <Bone className="mt-[14px] h-[14px] w-[480px] max-w-full" />
      <Bone className="mt-[8px] h-[14px] w-[320px]" />
      <div className="mt-[24px] flex gap-[12px]">
        <Bone className="h-[54px] w-[220px] rounded-lg" />
        <Bone className="size-[54px] rounded-lg" />
      </div>
      <Bone className="mt-[16px] h-[14px] w-[280px]" />
      <div className="mt-[28px] flex gap-[22px]">
        {[64, 52, 56].map((w) => (
          <div key={w} className="space-y-2">
            <Bone className="h-5" style={{ width: w }} />
            <Bone className="h-3 w-14" />
          </div>
        ))}
      </div>
      <Bone className="mt-[22px] h-[14px] w-[360px]" />
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