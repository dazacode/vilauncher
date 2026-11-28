import { AlertTriangle, BadgeCheck, PanelLeftClose, PanelLeftOpen, Plus, Search, SlidersHorizontal } from "lucide-react"
import { useEffect, useId, useRef, useState } from "react"
import { Popover } from "radix-ui"
import { ProjectAvatar } from "@/components/art/project-avatar"
import { Button } from "@/components/ui/button"
import { Segmented } from "@/components/ui/segmented"
import { Select } from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { Tooltip } from "@/components/ui/tooltip"
import { LauncherUpdate } from "@/features/shared/launcher-update"
import { useT } from "@/i18n"
import { assess, mainGpu, SORTS, sortLabel, sortProjects, withSpec } from "@/lib/compat"
import { openUrl } from "@/lib/api"
import { fps, gpuText, shortStage } from "@/lib/format"
import { run, useAlias, useStore } from "@/lib/store"
import type { Project, Snapshot } from "@/lib/types"
import { cn } from "@/lib/utils"
import { Brand } from "./brand"
import { JobCard } from "./job-card"
import { Bone, SidebarSkeleton } from "./skeleton"

const ROW = "[data-row]"

/** The black library panel: the VIPortLive mark, search, the project list, and the jobs. */
export function Sidebar() {
  const t = useT()
  const snap = useStore((s) => s.snap)
  const search = useStore((s) => s.search)
  const setSearch = useStore((s) => s.setSearch)
  const open = useStore((s) => s.open)
  const screenshot = useStore((s) => s.screenshot)
  const collapsed = useStore((s) => s.collapsed)
  const setCollapsed = useStore((s) => s.setCollapsed)
  const listRef = useRef<HTMLDivElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  // The row that takes Tab into the list (roving tabindex): the last one focused, else the selection.
  const [cursor, setCursor] = useState<string | null>(null)

  const query = screenshot || collapsed ? "" : search.trim().toLowerCase()
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
      if (i === 0 && searchRef.current) {
        e.preventDefault()
        searchRef.current.focus()
        return
      }
      next = Math.max(i - 1, 0)
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
    <aside className={cn("relative flex h-full shrink-0 flex-col overflow-hidden bg-sidebar transition-[width] duration-200 ease-out", collapsed ? "w-[64px]" : "w-[300px]")}>
      <div data-tauri-drag-region="deep" className={cn("drag flex h-[76px] shrink-0 items-end", collapsed ? "pl-[18px]" : "pl-[28px]")}>
        {collapsed ? <img src="/logo.svg" alt="VILauncher" className="size-[28px] rounded-[7px]" draggable={false} /> : <Brand size="lg" />}
      </div>

      {!collapsed && (
        // Search and its filter as one row, the same height: no title between the brand and the list.
        <div className="flex shrink-0 items-center gap-2 px-[28px] pt-[28px]">
          <label className="relative block min-w-0 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-fg-4" />
            <input
              ref={searchRef}
              value={screenshot ? "" : search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={onSearchKey}
              placeholder={t("shell.search")}
              aria-label={t("shell.search")}
              aria-controls="sidebar-projects"
              spellCheck={false}
              className="h-9 w-full rounded-md border border-line bg-fill-1 pr-3 pl-9 text-secondary text-fg placeholder:text-fg-4 transition-colors hover:border-line-2 focus-visible:border-accent/60 focus-visible:ring-2 focus-visible:ring-accent/25 focus-visible:outline-none"
            />
          </label>
          {snap ? <FilterMenu snap={snap} /> : <Bone className="size-9 shrink-0 rounded-md" />}
        </div>
      )}
      {collapsed && <div className="h-[24px] shrink-0" />}

      <div
        ref={listRef}
        id="sidebar-projects"
        aria-label={t("shell.list.label")}
        aria-busy={loading || undefined}
        onKeyDown={onListKey}
        className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto pt-[8px] pb-[24px] [scrollbar-width:none] [mask-image:linear-gradient(to_bottom,#000_calc(100%-40px),transparent)] [&::-webkit-scrollbar]:hidden"
      >
        {loading && !collapsed && <div className="px-[20px]"><SidebarSkeleton /></div>}
        {snap?.list.status === "error" && !collapsed && (
          <div className="mx-[28px] mt-4 rounded-lg border border-line bg-fill-1 p-3">
            <p className="text-secondary text-fg-2">{screenshot ? t("shell.list.loadFailed") : snap.list.error}</p>
            <Button size="sm" className="mt-2" onClick={() => void run({ type: "refresh_projects" })}>{t("common.retry")}</Button>
          </div>
        )}
        {snap &&
          sections.map((s) => (
            <Section key={s.id} title={t(`shell.list.sections.${s.id}`)} collapsed={collapsed}>
              {s.rows.map((p) => (
                <li key={p.repo}>
                  <Row project={p} snap={snap} collapsed={collapsed} tabbable={p.repo === active} onFocus={() => setCursor(p.repo)} />
                </li>
              ))}
            </Section>
          ))}
        {snap?.list.status === "ready" && order.length === 0 && !collapsed && (
          <div className="px-[28px] pt-6">
            <p className="text-secondary text-fg-2">{query ? t("shell.list.noMatch", { query: search.trim() }) : t("shell.list.noneForCard")}</p>
            {query ? (
              <Button variant="ghost" size="sm" className="mt-2 -ml-3" onClick={() => setSearch("")}>{t("shell.list.clearSearch")}</Button>
            ) : filter !== 0 ? (
              <Button variant="ghost" size="sm" className="mt-2 -ml-3" onClick={() => void run({ type: "set_gpu_filter", value: 0 })}>{t("shell.list.showAllCards")}</Button>
            ) : null}
          </div>
        )}
      </div>

      <div className={cn("shrink-0 border-t border-line pt-[14px] pb-[22px]", collapsed ? "px-[16px]" : "px-[24px]")}>
        {!collapsed && (
          <div className="mb-[14px] space-y-2 empty:hidden">
            <LauncherUpdate />
            <JobCard />
          </div>
        )}
        <div className="flex items-center gap-3">
          <Tooltip label={collapsed ? t("shell.list.unfold") : t("shell.list.fold")} side="right">
            <button
              type="button"
              aria-label={collapsed ? t("shell.list.unfold") : t("shell.list.fold")}
              aria-expanded={!collapsed}
              onClick={() => setCollapsed(!collapsed)}
              className="grid size-[32px] shrink-0 cursor-pointer place-items-center rounded-md text-fg-2 transition-colors hover:bg-fill-2 hover:text-fg"
            >
              {collapsed ? <PanelLeftOpen className="size-[24px]" strokeWidth={1.5} /> : <PanelLeftClose className="size-[24px]" strokeWidth={1.5} />}
            </button>
          </Tooltip>
          {!collapsed && !screenshot && (
            <Button variant="ghost" size="sm" className="min-w-0 flex-1 justify-start gap-2 px-2 text-fg-2 hover:text-fg" onClick={() => open({ kind: "import" })}>
              <Plus className="size-4!" /> {t("shell.list.import")}
            </Button>
          )}
        </div>
      </div>
    </aside>
  )
}

function Section({ title, collapsed, children }: { title: string; collapsed: boolean; children: React.ReactNode }) {
  const id = useId()
  return (
    <section aria-labelledby={id} className={cn(collapsed ? "pt-3 first:pt-0" : "pt-[22px] first:pt-[14px]")}>
      <h2 id={id} className={cn("pb-[8px] pl-[28px] text-[11px] leading-[16px] font-semibold tracking-[0.12em] text-fg-4 uppercase", collapsed && "sr-only")}>
        {title}
      </h2>
      {collapsed && <div aria-hidden className="mx-auto mb-2 h-px w-6 bg-fill-3" />}
      <ul role="list">{children}</ul>
    </section>
  )
}

function Row({ project: p, snap, collapsed, tabbable, onFocus }: { project: Project; snap: Snapshot; collapsed: boolean; tabbable: boolean; onFocus(): void }) {
  const t = useT()
  const alias = useAlias()(p)
  const selected = snap.selected === p.repo
  const setView = useStore((s) => s.setView)
  const job = p.build_job ?? (snap.job.repo === p.repo ? snap.job : null)
  const working = !!job?.running
  let line: React.ReactNode
  let tone = "text-fg-3"
  let warning: string | null = null
  let badge: { label: string; tone: "warn" | "bad" } | null = null
  // The second line: what it is doing, or what it measured and how it fits this PC.
  if (working) {
    line = <><Spinner className="size-3 text-accent-fg" /> {shortStage(job.stage_kind, job.stage)}</>
    tone = "text-accent-fg"
  } else if (p.listing.nuked) {
    line = t("shell.row.takenDown")
    tone = "text-warn"
  } else if (p.kind === "zip") {
    line = t("shell.row.imported")
  } else if (p.custom) {
    line = p.installed ? t("shell.row.installedByLink") : t("shell.row.byLink")
  } else {
    const f = fps(p)
    // A build that may not suit this PC keeps a quiet line; the warning is an icon by the name.
    const verdict = assess(p, withSpec(snap.system, snap.settings))
    if (verdict.level === "concern") warning = verdict.warning ?? t("shell.row.mayNotRun")
    line = [f != null ? `${f} fps` : null, p.gpu.length ? gpuText(p.gpu) : null].filter(Boolean).join(" · ") || t("shell.row.notMeasured")
  }
  // The outlined tag on the right, for what needs the player's eye.
  if (!working && p.listing.nuked) badge = { label: t("shell.row.badgeDown"), tone: "bad" }
  else if (!working && p.update) badge = { label: t("shell.row.badgeUpdate"), tone: "warn" }
  const button = (
    <button
      type="button"
      data-row
      tabIndex={tabbable ? 0 : -1}
      onFocus={onFocus}
      aria-current={selected ? "true" : undefined}
      aria-label={collapsed ? alias.name : undefined}
      onClick={() => {
        // A taken-down project has nothing to install: its page on VIPortLive explains it.
        if (p.listing.nuked) {
          void openUrl(p.listing.id ? `${snap.brand.site}/projects/${p.listing.id}` : snap.brand.site)
          return
        }
        setView("library")
        void run({ type: "select", repo: p.repo })
      }}
      className={cn(
        "group relative flex h-[60px] w-full cursor-pointer items-center text-left outline-none",
        collapsed ? "justify-center" : "gap-[14px] px-[28px]",
        "focus-visible:bg-fill-2 focus-visible:shadow-[inset_0_0_0_2px_var(--color-focus)]",
      )}
    >
      <span className={cn("absolute inset-y-[3px] rounded-lg transition-colors duration-150", collapsed ? "inset-x-[10px]" : "inset-x-[14px]", selected ? "bg-fill-2" : "group-hover:bg-fill-1")} />
      <span className="relative">
        <ProjectAvatar project={p} owner={alias.owner} size={36} />
        {collapsed && working && <span className="absolute -top-0.5 -right-0.5 size-2.5 rounded-full bg-accent ring-2 ring-black" />}
      </span>
      {!collapsed && (
        <>
          <span className="relative min-w-0 flex-1">
            <span className="flex items-center gap-1">
              <Name text={alias.name} className={selected ? "font-semibold text-fg" : "text-fg/85 group-hover:text-fg"} />
              {p.verified && <BadgeCheck aria-label={t("shell.row.verified")} className="size-3.5 shrink-0 text-accent-fg" />}
              {warning && (
                <Tooltip label={warning} side="right">
                  <AlertTriangle aria-label={warning} className="size-3.5 shrink-0 text-warn" />
                </Tooltip>
              )}
              {p.activity?.pending && !working && <span className="ml-1 size-1.5 shrink-0 rounded-full bg-fg-3" aria-label={p.activity.pending} />}
            </span>
            <span className={cn("mt-[3px] flex items-center gap-1 truncate text-[12px] leading-[16px]", tone)}>{line}</span>
          </span>
          {p.personal_fps != null && (
            <span className="relative shrink-0 rounded-md bg-fill-2 px-1.5 py-0.5 text-[11px] tabular-nums text-fg-2" title={t("library.hero.personalFps")}>
              {p.personal_fps.toFixed(1)} fps
            </span>
          )}
          {badge && (
            <span
              className={cn(
                "relative grid h-[18px] shrink-0 place-items-center rounded-[4px] border px-[6px] pl-[calc(6px+0.1em)] text-[9px] leading-none font-bold tracking-[0.1em]",
                badge.tone === "warn" && "border-warn/70 text-warn",
                badge.tone === "bad" && "border-bad/70 text-bad",
              )}
            >
              {badge.label}
            </span>
          )}
        </>
      )}
    </button>
  )
  return collapsed ? <Tooltip label={alias.name} side="right">{button}</Tooltip> : button
}

/**
 * The sidebar's groups: your repos, installed builds, and every other build in the list's order.
 * There is no "recommended" group: fit is judged from memory figures that authors report, which
 * are too new and too uneven to rank builds on. A build that doesn't fit still shows its warning.
 */
function group(snap: Snapshot, query: string, filter: number) {
  const s = snap.settings
  const keep = (p: Project) => {
    const text = !query || p.name.toLowerCase().includes(query) || p.repo.toLowerCase().includes(query)
    const card = (name: string) => p.custom || p.gpu.some((g) => g.toLowerCase() === name)
    const tier = p.custom || s.list_tier === 0 || p.listing.tier === (s.list_tier === 1 ? "tracked" : "watchlist")
    // Taken-down projects are hidden unless the player turns them on.
    const taken = s.show_nuked || !p.listing.nuked
    return text && tier && taken && (filter === 1 ? card("nvidia") : filter === 2 ? card("amd") : true)
  }
  const listed = sortProjects(snap.list.items.filter(keep), s.list_sort)
  return [
    { id: "custom" as const, rows: sortProjects(snap.custom.filter(keep), s.list_sort) },
    { id: "installed" as const, rows: listed.filter((p) => p.installed) },
    { id: "all" as const, rows: listed.filter((p) => !p.installed) },
  ].filter((s) => s.rows.length > 0)
}

/** Filter, sort and the list's options, in one menu. Every choice is kept in the launcher's settings. */
function FilterMenu({ snap }: { snap: Snapshot }) {
  const t = useT()
  const s = snap.settings
  const [open, setOpen] = useState(false)
  const [advanced, setAdvanced] = useState(false)
  const changed = [s.gpu_filter !== 0, s.show_nuked, s.list_sort !== 0, s.list_tier !== 0, s.spec_ram_gb != null || s.spec_vram_gb != null].filter(Boolean).length
  const setList = (next: { show_nuked?: boolean; sort?: number; tier?: number }) =>
    void run({
      type: "set_list_options",
      show_nuked: next.show_nuked ?? s.show_nuked,
      sort: next.sort ?? s.list_sort,
      tier: next.tier ?? s.list_tier,
    })
  const tiers = [
    { value: "0", label: t("shell.filter.tiers.all") },
    { value: "1", label: t("shell.filter.tiers.tracked") },
    { value: "2", label: t("shell.filter.tiers.watchlist") },
  ] as const
  const cards = [
    { value: "0", label: t("shell.filter.allCards") },
    { value: "1", label: "NVIDIA" },
    { value: "2", label: "AMD" },
  ] as const
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button
          type="button"
          aria-label={changed ? t("shell.filter.buttonOn", { count: changed }) : t("shell.filter.button")}
          title={t("shell.filter.button")}
          className={cn(
            "relative grid size-9 shrink-0 cursor-pointer place-items-center rounded-md border border-line bg-fill-1 text-fg-2 transition-colors hover:border-line-2 hover:bg-fill-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/25",
            open && "border-line-2 bg-fill-3 text-fg",
          )}
        >
          <SlidersHorizontal className="size-4" />
          {changed > 0 && (
            <span className="absolute -top-1 -right-1 grid h-4 min-w-4 place-items-center rounded-full bg-accent px-1 text-[10px] leading-none font-semibold text-white">
              {changed}
            </span>
          )}
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="start"
          sideOffset={6}
          collisionPadding={12}
          className="z-50 max-h-[min(640px,80vh)] w-[248px] space-y-4 overflow-y-auto rounded-xl border border-line-2 bg-raised p-3.5 text-body text-fg shadow-[0_12px_36px_rgb(0_0_0/0.5)] outline-none"
        >
          <label className="flex items-center justify-between gap-3">
            <span>{t("shell.filter.showTakenDown")}</span>
            <Switch checked={s.show_nuked} onCheckedChange={(v) => setList({ show_nuked: v })} aria-label={t("shell.filter.showTakenDown")} />
          </label>
          <div className="space-y-1.5">
            <span className="text-caption font-semibold text-fg-2">{t("shell.filter.sort")}</span>
            <Select
              label={t("shell.filter.sort")}
              value={String(s.list_sort)}
              options={SORTS.map((id, i) => ({ value: String(i), label: sortLabel(id) }))}
              onChange={(v) => setList({ sort: Number(v) })}
              className="w-full"
            />
          </div>
          <div className="space-y-1.5">
            <span className="text-caption font-semibold text-fg-2">{t("shell.filter.tier")}</span>
            <Segmented
              label={t("shell.filter.tier")}
              size="sm"
              className="flex w-full"
              value={String(s.list_tier) as "0" | "1" | "2"}
              options={tiers}
              onChange={(v) => setList({ tier: Number(v) })}
            />
          </div>
          <div className="space-y-1.5">
            <span className="text-caption font-semibold text-fg-2">{t("shell.filter.cards")}</span>
            <Segmented
              label={t("shell.filter.cards")}
              size="sm"
              className="flex w-full"
              value={String(s.gpu_filter) as "0" | "1" | "2"}
              options={cards}
              onChange={(v) => void run({ type: "set_gpu_filter", value: Number(v) })}
            />
          </div>
          <div className="space-y-2 border-t border-line pt-3">
            <button
              type="button"
              aria-expanded={advanced}
              onClick={() => setAdvanced(!advanced)}
              className="flex w-full items-center justify-between text-caption font-semibold text-fg-2 hover:text-fg"
            >
              {t("shell.filter.advanced")}
              <span aria-hidden>{advanced ? "−" : "+"}</span>
            </button>
            {advanced && <SpecFields snap={snap} />}
          </div>
          {changed > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="w-full"
              onClick={() => {
                void run({ type: "set_list_options", show_nuked: false, sort: 0, tier: 0 })
                void run({ type: "set_gpu_filter", value: 0 })
                void run({ type: "set_spec", ram_gb: null, vram_gb: null })
              }}
            >
              {t("shell.filter.reset")}
            </Button>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  )
}

/** The player's RAM and video memory, for judging builds by hand. Empty: what the launcher found. */
function SpecFields({ snap }: { snap: Snapshot }) {
  const t = useT()
  const s = snap.settings
  const ram = snap.system?.ram_gb ?? null
  const vram = mainGpu(snap.system)?.vram_gb ?? null
  const save = (ram_gb: number | null, vram_gb: number | null) => void run({ type: "set_spec", ram_gb, vram_gb })
  return (
    <div className="space-y-2">
      <p className="text-caption text-fg-3">{t("shell.filter.specHint")}</p>
      <SpecInput label={t("shell.filter.ram")} value={s.spec_ram_gb} detected={ram} onCommit={(v) => save(v, s.spec_vram_gb)} />
      <SpecInput label={t("shell.filter.vram")} value={s.spec_vram_gb} detected={vram} onCommit={(v) => save(s.spec_ram_gb, v)} />
    </div>
  )
}

function SpecInput({
  label,
  value,
  detected,
  onCommit,
}: {
  label: string
  value: number | null
  detected: number | null
  onCommit(value: number | null): void
}) {
  const t = useT()
  const [text, setText] = useState(value == null ? "" : String(value))
  useEffect(() => setText(value == null ? "" : String(value)), [value])
  const commit = () => {
    const n = Number(text.replace(",", "."))
    onCommit(text.trim() !== "" && n > 0 ? n : null)
  }
  return (
    <label className="flex items-center justify-between gap-3">
      <span>{label}</span>
      <input
        inputMode="decimal"
        value={text}
        placeholder={detected != null ? String(detected) : t("common.unknown")}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit()
        }}
        aria-label={label}
        className="h-8 w-24 rounded-md border border-line bg-fill-1 px-2 text-right text-secondary text-fg placeholder:text-fg-3 focus-visible:border-accent/60 focus-visible:outline-none"
      />
    </label>
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