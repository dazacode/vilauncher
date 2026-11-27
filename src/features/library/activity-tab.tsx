import { AlertCircle, CheckCircle2, Copy, Download, Square } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { copyText, getBackend, saveFile } from "@/lib/api"
import { run, useStore } from "@/lib/store"

export function ActivityTab() {
  const job = useStore((s) => s.snap!.job)
  const [lines, setLines] = useState<string[]>([])
  const [omitted, setOmitted] = useState(0)
  const box = useRef<HTMLPreElement>(null)
  const stick = useRef(true)
  const have = useRef(0)
  useEffect(() => {
    // Only what is new is fetched; a cleared log starts over.
    if (job.log_lines < have.current) {
      have.current = 0
      setLines([])
    }
    if (job.log_lines === have.current) return
    void getBackend().then(async (b) => {
      const view = await b.log(have.current)
      have.current = view.from + view.lines.length
      setOmitted(view.omitted)
      setLines((old) => [...(view.from === 0 ? [] : old), ...view.lines].slice(-4000))
    })
  }, [job.log_lines])
  useEffect(() => {
    if (stick.current && box.current) box.current.scrollTop = box.current.scrollHeight
  }, [lines])
  const status = !job.stage ? "Nothing running" : job.stage
  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        {job.running ? <Spinner className="text-accent-fg" /> : job.failed ? <AlertCircle className="size-4 text-bad" /> : job.stage ? <CheckCircle2 className="size-4 text-ok" /> : null}
        <p className={job.failed ? "text-body text-bad" : "text-body text-fg-2"}>{status}</p>
        <div className="ml-auto flex gap-1">
          {job.stoppable && (
            <Button size="sm" variant="danger" onClick={() => void run({ type: "stop" })}><Square /> Stop</Button>
          )}
          <Button size="sm" variant="ghost" disabled={!lines.length} onClick={() => void copyText(lines.join("\n")).then(() => toast.success("Log copied"))}><Copy /> Copy</Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={!lines.length}
            onClick={async () => {
              const path = await saveFile("Export the activity log", "vilauncher.log", [{ name: "Log", extensions: ["log", "txt"] }])
              if (path) void run({ type: "export_log", path }, "Log exported")
            }}
          >
            <Download /> Export
          </Button>
        </div>
      </div>
      {omitted > 0 && <p className="text-caption text-fg-3">{omitted} earlier lines scrolled out of the log.</p>}
      <pre
        ref={box}
        data-selectable
        onScroll={(e) => {
          const el = e.currentTarget
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24
        }}
        className="h-[420px] overflow-auto rounded-lg border border-line bg-black/35 p-4 font-mono text-caption leading-5 whitespace-pre-wrap text-fg-2"
      >
        {lines.length ? lines.join("\n") : <span className="text-fg-4">The log of installs, builds and games shows here.</span>}
      </pre>
    </div>
  )
}
