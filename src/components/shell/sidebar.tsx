import { BadgeCheck, LayoutGrid, Plus, Search, Settings2 } from "lucide-react"
import { motion } from "motion/react"
import { useEffect, useId, useRef, useState } from "react"
import { ProjectAvatar } from "@/components/art/project-avatar"
import { Button } from "@/components/ui/button"
import { Segmented } from "@/components/ui/segmented"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip } from "@/components/ui/tooltip"
import { matchesCard } from "@/lib/compat"
import { fps, gpuText, shortStage } from "@/lib/format"
import { run, useAlias, useStore } from "@/lib/store"
import type { Project, Snapshot } from "@/lib/types"
import { cn } from "@/lib/utils"
import { Brand } from "./brand"
import { JobCard } from "./job-card"
import { Bone, SidebarSkeleton } from "./skeleton"

const FILTERS = [
  { value: "0", label: "All" },
  { value: "1", label: "NVIDIA" },
  { value: "2", label: "AMD" },
] as const

const ROW = "[data-row]"

export function Sidebar() {
  const snap = useStore((s) => s.snap)
  const view = useStore((s) => s.view)
  const setView = useStore((s) => s.setView)
  const search = useStore((s) => s.search)
  const setSearch = useStore((s) => s.setSearch)
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  const listRef = useRef<HTMLDivElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  // The row that takes Tab into the list (roving tabindex): the last one focused, else the selection.
  const [cursor, setCursor] = useState<string | null>(null)

  const query = screenshot ? "" : search.trim().toLowerCase()
  const filter = snap?.settings.gpu_filter ?? 0
  const loading = !snap || snap.list.status === "loading"
  const sections = snap ? group(snap, query, filter) : []
  const order = sections.flatMap((s) => s.rows.map((p) => p.repo))
  const active = cursor && order.includes(cursor) ? cursor : snap?.selected && order.includes(snap.selected) ? snap.selected : order[0]

  const rows = () => [...(listRef.current?.querySelectorAll<HTMLElement>(ROW) ?? [])]
  const onListKey = (e: React.KeyboardEvent) => {
    const all = rows()
    const i = all.indexOf(document.activeElement as HTMLElement)
    if (i < 0) return
    let next: number
    if (e.key === "ArrowDown") next = Math.min(i + 1, all.length - 1)
    else if (e.key === "ArrowUp") {
      if (i === 0) {
        e.preventDefault()
        searchRef.current?.focus()
        return
      }
      next = i - 1
    } else if (e.key === "Home") next = 0
    else if (e.key === "End") next = all.length - 1
    else return
    e.preventDefault()
    all[next]?.focus()
  }
  const onSearchKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      const target = listRef.current?.querySelector<HTMLElement>(`${ROW}[tabindex="0"]`) ?? rows()[0]
      if (target) {
        e.preventDefault()
        target.focus()
      }
    } else if (e.key === "Escape" && search) {
      e.preventDefault()
      e.stopPropagation()
      setSearch("")
    }
  }

  return (
    <aside className="flex w-[264px] shrink-0 flex-col border-r border-line bg-[rgb(10_9_14/0.55)]">
      <div className="drag flex h-12 shrink-0 items-center px-4">
        <Brand />
      </div>
      <nav className="space-y-1 px-2 pt-1" aria-label="Pages">
        <NavItem icon={<LayoutGrid />} label="Library" active={view === "library"} onClick={() => setView("library")} />
        <NavItem icon={<Settings2 />} label="Settings" active={view === "settings"} onClick={() => setView("settings")} />
      </nav>
      <div className="space-y-2 px-2 pt-4">
        <label className="relative block">
          <Search className="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-fg-3" />
          <input
            ref={searchRef}
            value={screenshot ? "" : search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={onSearchKey}
            placeholder="Search library"
            aria-label="Search library"
            aria-controls="sidebar-projects"
            spellCheck={false}
            className="h-8 w-full rounded-md border border-line bg-fill-1 pr-3 pl-9 text-secondary text-fg placeholder:text-fg-3 transition-colors hover:border-line-2 focus-visible:border-accent/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/25"
          />
        </label>
        {snap ? (
          <Segmented
            label="Graphics cards"
            size="sm"
            className="flex w-full"
            value={String(filter) as "0" | "1" | "2"}
            options={FILTERS}
            onChange={(v) => void run({ type: "set_gpu_filter", value: Number(v) })}
          />
        ) : (
          <Bone className="h-8 w-full rounded-md" />
        )}
      </div>
      <div
        ref={listRef}
        id="sidebar-projects"
        aria-label="Projects"
        aria-busy={loading || undefined}
        onKeyDown={onListKey}
        className="min-h-0 flex-1 overflow-y-auto px-2 pb-4"
      >
        {loading && <SidebarSkeleton />}
        {snap?.list.status === "error" && (
          <div className="mt-4 rounded-lg border border-line bg-fill-1 p-3">
            <p className="text-secondary text-fg-2">{screenshot ? "The project list could not load." : snap.list.error}</p>
            <Button size="sm" className="mt-2" onClick={() => void run({ type: "refresh_projects" })}>Try again</Button>
          </div>
        )}
        {snap &&
          sections.map((s) => (
            <Section key={s.title} title={s.title}>
              {s.rows.map((p) => (
                <li key={p.repo}>
                  <Row project={p} snap={snap} tabbable={p.repo === active} onFocus={() => setCursor(p.repo)} />
                </li>
              ))}
            </Section>
          ))}
        {snap?.list.status === "ready" && order.length === 0 && (
          <div className="px-2 pt-6">
            <p className="text-secondary text-fg-2">{query ? `No project matches “${search.trim()}”.` : "No projects for this card yet."}</p>
            {query ? (
              <Button variant="ghost" size="sm" className="mt-2 -ml-3" onClick={() => setSearch("")}>Clear search</Button>
            ) : filter !== 0 ? (
              <Button variant="ghost" size="sm" className="mt-2 -ml-3" onClick={() => void run({ type: "set_gpu_filter", value: 0 })}>Show all cards</Button>
            ) : null}
          </div>
        )}
      </div>
      <div className="space-y-2 border-t border-line px-2 py-3">
        <JobCard />
        {!screenshot && (
          <Button variant="ghost" size="sm" className="w-full justify-start gap-3 px-2" onClick={() => open({ kind: "import" })}>
            <Plus className="size-4!" /> Import a repo or build
          </Button>
        )}
      </div>
    </aside>
  )
}

/**
 * The sidebar's groups. Recommended: the first three not-installed projects
 * (in VIPortLive's order) made for the player's card, else simply the first three.
 */
function group(snap: Snapshot, query: string, filter: number) {
  const keep = (p: Project) => {
    const text = !query || p.name.toLowerCase().includes(query) || p.repo.toLowerCase().includes(query)
    const card = (name: string) => p.custom || p.gpu.some((g) => g.toLowerCase() === name)
    return text && (filter === 1 ? card("nvidia") : filter === 2 ? card("amd") : true)
  }
  const listed = snap.list.items.filter(keep)
  const rest = listed.filter((p) => !p.installed)
  const suited = rest.filter((p) => matchesCard(p, snap.system) === true)
  const recommended = (suited.length ? suited : rest).slice(0, 3)
  return [
    { title: "Your repos", rows: snap.custom.filter(keep) },
    { title: "Installed", rows: listed.filter((p) => p.installed) },
    { title: "Recommended", rows: recommended },
    { title: "All projects", rows: rest.filter((p) => !recommended.includes(p)) },
  ].filter((s) => s.rows.length > 0)
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  const id = useId()
  return (
    <section aria-labelledby={id} className="pt-6 first:pt-4">
      <h2 id={id} className="px-2 pb-1 text-caption font-semibold tracking-[0.01em] text-fg-2">
        {title}
      </h2>
      <ul role="list">{children}</ul>
    </section>
  )
}

function NavItem({ icon, label, active, onClick }: { icon: React.ReactNode; label: string; active: boolean; onClick(): void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex h-8 w-full cursor-pointer items-center gap-3 rounded-md px-2 text-body font-medium transition-colors duration-150 [&_svg]:size-4",
        active ? "bg-fill-3 text-fg [&_svg]:text-accent-fg" : "text-fg-2 hover:bg-fill-2 hover:text-fg active:bg-fill-3 [&_svg]:text-fg-3",
      )}
    >
      {icon}
      {label}
    </button>
  )
}

function Row({ project: p, snap, tabbable, onFocus }: { project: Project; snap: Snapshot; tabbable: boolean; onFocus(): void }) {
  const alias = useAlias()(p)
  const selected = snap.selected === p.repo
  const setView = useStore((s) => s.setView)
  const working = snap.job.running && snap.job.repo === p.repo
  let line: React.ReactNode
  let tone = "text-fg-3"
  if (working) {
    line = <><Spinner className="size-3 text-accent-fg" /> {shortStage(snap.job.stage)}</>
    tone = "text-accent-fg"
  } else if (p.update) {
    line = "Update available"
    tone = "text-warn"
  } else if (p.kind === "zip") {
    line = "Imported build"
  } else if (p.custom) {
    line = p.installed ? "Installed · added by link" : "Added by link"
  } else {
    const f = fps(p)
    // A build made for another card says so, quietly, where its cards would be.
    const card = matchesCard(p, snap.system) === false ? `Made for ${p.gpu.join(" and ")}` : p.gpu.length ? gpuText(p.gpu) : null
    line = [f != null ? `${f} fps` : null, card].filter(Boolean).join(" · ") || "Not measured yet"
  }
  return (
    <button
      type="button"
      data-row
      tabIndex={tabbable ? 0 : -1}
      onFocus={onFocus}
      aria-current={selected ? "true" : undefined}
      onClick={() => {
        setView("library")
        void run({ type: "select", repo: p.repo })
      }}
      className="group relative flex h-[52px] w-full cursor-pointer items-center gap-3 rounded-md px-2 text-left focus-visible:outline-offset-0"
    >
      {selected && (
        <motion.span layoutId="sidebar-pick" className="absolute inset-0 rounded-md bg-fill-3" transition={{ type: "spring", duration: 0.28, bounce: 0.1 }} />
      )}
      <span className="absolute inset-0 rounded-md transition-colors duration-150 group-hover:bg-fill-1 group-active:bg-fill-2" />
      <ProjectAvatar project={p} owner={alias.owner} size={32} className="relative" />
      <span className="relative min-w-0 flex-1">
        <span className="flex items-center gap-1">
          <Name text={alias.name} className={selected ? "text-fg" : "text-fg/90"} />
          {p.verified && <BadgeCheck aria-label="Verified" className="size-3.5 shrink-0 text-fg-3" />}
          {p.activity?.pending && !working && <span className="ml-1 size-1.5 shrink-0 rounded-full bg-fg-2" aria-label={p.activity.pending} />}
        </span>
        <span className={cn("flex items-center gap-1 truncate text-caption", tone)}>{line}</span>
      </span>
    </button>
  )
}

/** A project name that shows itself whole in a tooltip when the row cuts it short. */
function Name({ text, className }: { text: string; className?: string }) {
  const [el, setEl] = useState<HTMLSpanElement | null>(null)
  const [cut, setCut] = useState(false)
  useEffect(() => {
    if (!el) return
    const check = () => setCut(el.scrollWidth > el.clientWidth + 1)
    check()
    const ro = new ResizeObserver(check)
    ro.observe(el)
    return () => ro.disconnect()
  }, [el, text])
  const name = (
    <span ref={setEl} className={cn("truncate text-body font-medium", className)}>
      {text}
    </span>
  )
  return cut ? <Tooltip label={text} side="right">{name}</Tooltip> : name
}
