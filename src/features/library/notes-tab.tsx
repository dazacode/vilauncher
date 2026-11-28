import { Textarea } from "@/components/ui/input"
import { useT } from "@/i18n"
import { useDraft } from "@/lib/hooks"
import { run } from "@/lib/store"
import type { Project } from "@/lib/types"

export function NotesTab({ project: p }: { project: Project }) {
  const t = useT()
  const [notes, setNotes] = useDraft(p.launch.notes, (v) => void run({ type: "set_project_notes", repo: p.repo, notes: v }))
  return (
    <div className="max-w-[760px]">
      <p className="mb-3 text-secondary text-fg-3">{t("library.notes.hint")}</p>
      <Textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={12} placeholder={t("library.notes.placeholder")} aria-label={t("library.tabs.notes")} className="min-h-64 text-body" />
    </div>
  )
}
