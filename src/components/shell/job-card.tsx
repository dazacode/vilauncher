import { AlertCircle, CheckCircle2, Square, X } from "lucide-react"
import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { useEffect, useMemo, useState } from "react"
import { Tooltip } from "@/components/ui/tooltip"
import { useT } from "@/i18n"
import { run, useAlias, useJob, useStore } from "@/lib/store"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/lib/utils"

/** What the launcher is doing now, in the corner of the sidebar, like a download. */
export function JobCard() {
  const t = useT()
  // The projects and the job, without the log's length: the card is not drawn again for each line a build prints.
  const snap = useStore(useShallow((s) => (s.snap ? { list: s.snap.list, custom: s.snap.custom, selected: s.snap.selected } : null)))
  const mainJob = useJob()
  const setTab = useStore((s) => s.setTab)
  const setView = useStore((s) => s.setView)
  const alias = useAlias()
  const still = useReducedMotion()
  const [dismissed, setDismissed] = useState<string | null>(null)
  const projects = useMemo(() => (snap ? [...snap.custom, ...snap.list.items] : []), [snap?.custom, snap?.list])
  const builds = projects.filter((project) => project.build_job?.running)
  const activeBuild = builds.find((project) => project.repo === snap?.selected) ?? builds[0]
  const job = activeBuild?.build_job ?? mainJob
  const key = job ? `${job.repo}|${job.stage}` : ""
  useEffect(() => {
    if (job?.running) setDismissed(null)
  }, [job?.running])
  const show = !!job && !!job.stage && dismissed !== key && job.stage_kind !== "playing"
  const project = job?.repo ? projects.find((p) => p.repo === job.repo) : undefined
  const title = project ? alias(project).name : "VILauncher"
  return (
    <AnimatePresence initial={false}>
      {show && job && (
        // Grows in from nothing, so the list above shortens smoothly instead of jumping.
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          exit={{ opacity: 0, height: 0 }}
          transition={{ duration: still ? 0 : 0.22, ease: [0.22, 1, 0.36, 1] }}
          className="overflow-hidden"
        >
          <div role="status" aria-live="polite" className="rounded-lg border border-line bg-fill-2 p-3">
            <div className="flex items-center gap-2">
              {job.running ? (
                <span className="grid size-4 shrink-0 place-items-center"><span className="size-2 rounded-full bg-accent" /></span>
              ) : job.failed ? (
                <AlertCircle className="size-4 shrink-0 text-bad" />
              ) : (
                <CheckCircle2 className="size-4 shrink-0 text-ok" />
              )}
              <p className="min-w-0 flex-1 truncate text-secondary font-medium text-fg">{title}</p>
              <button
                type="button"
                onClick={() => {
                  if (job.repo) void run({ type: "select", repo: job.repo })
                  setView("library")
                  setTab("activity")
                }}
                className={cn(small, "px-2 text-caption font-medium text-fg-2")}
              >
                {t("shell.job.log")}
              </button>
              {job.stoppable && (
                <Tooltip label={t("common.stop")} side="top">
                  <button type="button" aria-label={t("common.stop")} onClick={() => void run({ type: "stop" })} className={cn(small, "w-6 text-fg-2 hover:text-bad")}>
                    <Square className="size-3" />
                  </button>
                </Tooltip>
              )}
              {!job.running && (
                <button type="button" aria-label={t("shell.job.dismiss")} onClick={() => setDismissed(key)} className={cn(small, "w-6 text-fg-3")}>
                  <X className="size-3.5" />
                </button>
              )}
            </div>
            <p className="mt-1 line-clamp-2 pl-6 text-caption text-fg-3">{job.stage}</p>
            {builds.length > 1 && (
              <ul className="mt-1 pl-6 text-caption text-fg-3">
                {builds.filter((p) => p.repo !== job.repo).map((p) => (
                  <li key={p.repo}>
                    <button type="button" className="w-full truncate text-left hover:underline" onClick={() => {
                      void run({ type: "select", repo: p.repo })
                      setView("library")
                      setTab("activity")
                    }}>{alias(p).name} · {p.build_job?.stage}</button>
                  </li>
                ))}
              </ul>
            )}
            {job.running && (
              <div className="mt-2 ml-6 h-1 overflow-hidden rounded-full bg-fill-3">
                {still ? (
                  <div className="h-full w-full rounded-full bg-accent/50" />
                ) : (
                  <motion.div className="h-full w-1/3 rounded-full bg-accent" animate={{ x: ["-100%", "300%"] }} transition={{ duration: 1.6, repeat: Infinity, ease: "easeInOut" }} />
                )}
              </div>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

const small = "grid h-6 shrink-0 cursor-pointer place-items-center rounded-sm transition-colors duration-150 hover:bg-fill-3 hover:text-fg active:bg-fill-4"
