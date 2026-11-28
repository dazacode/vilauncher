import { AlertCircle, CheckCircle2, Copy, Download, Square, Trash2, Wrench } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { useT } from "@/i18n"
import { copyText, getBackend, openUrl, saveFile } from "@/lib/api"
import { run, useStore } from "@/lib/store"
import type { Project, ReportView } from "@/lib/types"

/**
 * After a failed job: a GitHub issue for the launcher or the build's repo, with the log filled in.
 * Nothing is sent by the launcher; the player reads and submits the issue on GitHub.
 */
function ReportBar({ project }: { project: Project }) {
  const t = useT()
  const [busy, setBusy] = useState(false)
  const fetchReport = async () => {
    setBusy(true)
    try {
      return await run<ReportView>({ type: "report", repo: project.repo })
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="space-y-2 rounded-lg border border-line bg-fill-1 p-3">
      <p className="text-caption text-fg-2">{t("library.report.text")}</p>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={async () => {
            const r = await fetchReport()
            if (r) void openUrl(r.launcher_url)
          }}
        >
          {t("library.report.to", { name: "VILauncher" })}
        </Button>
        {project.repo.split("/").length === 2 && !project.repo.includes("://") && (
          <Button
            size="sm"
            variant="ghost"
            disabled={busy}
            onClick={async () => {
              const r = await fetchReport()
              if (r?.build_url) void openUrl(r.build_url)
            }}
          >
            {t("library.report.to", { name: project.name })}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={async () => {
            const r = await fetchReport()
            if (r) void copyText(r.text).then(() => toast.success(t("library.report.copied")))
          }}
        >
          <Copy /> {t("library.report.copy")}
        </Button>
      </div>
    </div>
  )
}

export function ActivityTab({ project }: { project: Project }) {
  const t = useT()
  const snap = useStore((s) => s.snap!)
  const job = project.build_job ?? (snap.job.repo === project.repo ? snap.job : null)
  const logLines = job?.log_lines ?? 0
  const logKey = `${project.repo}:${job?.generation ?? 0}`
  const [lines, setLines] = useState<string[]>([])
  const [omitted, setOmitted] = useState(0)
  const box = useRef<HTMLPreElement>(null)
  const stick = useRef(true)
  const have = useRef({ key: "", count: 0 })
  useEffect(() => {
    if (have.current.key !== logKey || logLines < have.current.count) {
      have.current = { key: logKey, count: 0 }
      setLines([])
      setOmitted(0)
    }
    if (logLines === have.current.count) return
    let cancelled = false
    void getBackend().then(async (b) => {
      const view = await b.log(have.current.count, project.repo)
      if (cancelled) return
      have.current.count = view.from + view.lines.length
      setOmitted(view.omitted)
      setLines((old) => [...(view.from === 0 ? [] : old), ...view.lines].slice(-4000))
    })
    return () => { cancelled = true }
  }, [logLines, logKey, project.repo])
  useEffect(() => {
    if (stick.current && box.current) box.current.scrollTop = box.current.scrollHeight
  }, [lines])
  const status = job?.stage || t("library.activity.nothing")
  return (
    <div className="space-y-4">
      {project.activity?.warm_up && <p className="text-caption text-fg-3">{t("library.activity.warmUp")}</p>}
      <div className="flex items-center gap-3">
        {job?.running ? <Spinner className="text-accent-fg" /> : job?.failed ? <AlertCircle className="size-4 text-bad" /> : job?.stage ? <CheckCircle2 className="size-4 text-ok" /> : null}
        <p className={job?.failed ? "text-body text-bad" : "text-body text-fg-2"}>{status}</p>
        <div className="ml-auto flex gap-1">
          {!snap.job.running && !snap.active_builds && project.installed && (
            <Button size="sm" variant="ghost" title={t("library.activity.fixTitle")} onClick={() => void run({ type: "fix", repo: project.repo })}>
              <Wrench /> {t("library.fix")}
            </Button>
          )}
          {!snap.job.running && !snap.active_builds && project.installed && (
            <Button size="sm" variant="ghost" title={t("library.activity.clearCacheTitle")} onClick={() => void run({ type: "clear_shader_cache", repo: project.repo })}>
              <Trash2 /> {t("library.activity.clearCache")}
            </Button>
          )}
          {job?.stoppable && (
            <Button size="sm" variant="danger" onClick={() => void run({ type: "stop" })}><Square /> {t("library.action.stop")}</Button>
          )}
          <Button size="sm" variant="ghost" disabled={!lines.length} onClick={() => void copyText(lines.join("\n")).then(() => toast.success(t("library.activity.logCopied")))}><Copy /> {t("common.copy")}</Button>
          <Button
            size="sm"
            variant="ghost"
            title={t("library.activity.summaryTitle")}
            onClick={async () => {
              const text = await run<string>({ type: "diagnostic_summary", repo: project.repo })
              if (text) void copyText(text).then(() => toast.success(t("library.activity.summaryCopied")))
            }}
          >
            <Copy /> {t("library.activity.summary")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={!lines.length}
            onClick={async () => {
              // i18n-ignore: the file's default name
              const path = await saveFile(t("library.activity.exportTitle"), "vilauncher.log", [{ name: t("library.activity.exportFilter"), extensions: ["log", "txt"] }])
              if (path) void run({ type: "export_log", path, repo: project.repo }, t("library.activity.exported"))
            }}
          >
            <Download /> {t("library.activity.export")}
          </Button>
        </div>
      </div>
      {job?.failed && !job.running && <ReportBar project={project} />}
      {omitted > 0 && <p className="text-caption text-fg-3">{t("library.activity.omitted", { count: omitted })}</p>}
      <p className="text-caption text-fg-3">{t("library.activity.fixExplain")}</p>
      <pre
        ref={box}
        data-selectable
        onScroll={(e) => {
          const el = e.currentTarget
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24
        }}
        className="h-[420px] overflow-auto rounded-lg border border-line bg-black/35 p-4 font-mono text-caption leading-5 whitespace-pre-wrap text-fg-2"
      >
        {lines.length ? lines.join("\n") : <span className="text-fg-4">{t("library.activity.empty")}</span>}
      </pre>
    </div>
  )
}
