import { motion } from "motion/react"
import { useRef } from "react"
import { type Key, useT } from "@/i18n"
import { useStore, type ProjectTab } from "@/lib/store"
import type { Project } from "@/lib/types"
import { cn } from "@/lib/utils"
import { ActivityTab } from "./activity-tab"
import { GraphicsTab } from "./graphics-tab"
import { LaunchTab } from "./launch-tab"
import { NotesTab } from "./notes-tab"
import { OverviewTab } from "./overview-tab"

/** The tabs, in order, with their names' keys. */
const TABS = [
  { id: "overview", labelKey: "library.tabs.overview" },
  { id: "graphics", labelKey: "library.tabs.graphics" },
  { id: "launch", labelKey: "library.tabs.launch" },
  { id: "notes", labelKey: "library.tabs.notes" },
  { id: "activity", labelKey: "library.tabs.activity" },
] as const satisfies readonly { id: ProjectTab; labelKey: Key }[]

const tabId = (id: ProjectTab) => `project-tab-${id}`
const PANEL = "project-tabpanel"

export function ProjectTabs({ project }: { project: Project }) {
  const t = useT()
  const tab = useStore((s) => s.tab)
  const setTab = useStore((s) => s.setTab)
  const running = useStore((s) => s.snap?.job.running)
  const screenshot = useStore((s) => s.screenshot)
  const listRef = useRef<HTMLDivElement>(null)
  const hidden = screenshot && (tab === "launch" || tab === "notes" || tab === "activity")
  // Arrow keys move between tabs and open them (the WAI-ARIA tabs pattern, automatic activation).
  const onKey = (e: React.KeyboardEvent) => {
    const i = TABS.findIndex((x) => x.id === tab)
    const next =
      e.key === "ArrowRight" ? (i + 1) % TABS.length : e.key === "ArrowLeft" ? (i - 1 + TABS.length) % TABS.length : e.key === "Home" ? 0 : e.key === "End" ? TABS.length - 1 : -1
    if (next < 0) return
    e.preventDefault()
    setTab(TABS[next].id)
    listRef.current?.querySelector<HTMLElement>(`#${tabId(TABS[next].id)}`)?.focus()
  }
  return (
    <div className="max-w-[1280px] px-10 pb-16">
      <div ref={listRef} role="tablist" aria-label={t("library.tabs.label")} onKeyDown={onKey} className="flex gap-8 border-b border-line">
        {TABS.map((x) => (
          <button
            key={x.id}
            id={tabId(x.id)}
            type="button"
            role="tab"
            aria-selected={tab === x.id}
            aria-controls={PANEL}
            tabIndex={tab === x.id ? 0 : -1}
            onClick={() => setTab(x.id)}
            className={cn(
              "relative -mb-px flex h-11 cursor-pointer items-center gap-2 text-body font-medium transition-colors duration-150",
              tab === x.id ? "text-fg" : "text-fg-3 hover:text-fg-2",
            )}
          >
            {t(x.labelKey)}
            {x.id === "activity" && running && <span className="size-1.5 rounded-full bg-accent" aria-label={t("library.tabs.running")} />}
            {tab === x.id && <motion.span layoutId="tab-line" className="absolute inset-x-0 bottom-0 h-0.5 rounded-full bg-accent" transition={{ type: "spring", duration: 0.3, bounce: 0.1 }} />}
          </button>
        ))}
      </div>
      <motion.div
        key={`${project.repo}-${tab}`}
        id={PANEL}
        role="tabpanel"
        aria-labelledby={tabId(tab)}
        initial={{ opacity: 0, y: 4 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.18 }}
        className="pt-8"
      >
        {hidden ? (
          <p className="text-body text-fg-3">{t("library.tabs.hidden")}</p>
        ) : tab === "overview" ? (
          <OverviewTab project={project} />
        ) : tab === "graphics" ? (
          <GraphicsTab project={project} />
        ) : tab === "launch" ? (
          <LaunchTab project={project} />
        ) : tab === "notes" ? (
          <NotesTab project={project} />
        ) : (
          <ActivityTab project={project} />
        )}
      </motion.div>
    </div>
  )
}
