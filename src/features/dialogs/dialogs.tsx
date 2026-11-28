import { AlertTriangle, Check, CheckCircle2, Download, FileArchive, FolderOpen, Link2, Plus, Search, Star, Trash2 } from "lucide-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Dialog } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Segmented } from "@/components/ui/segmented"
import { Spinner } from "@/components/ui/spinner"
import { startPlay } from "@/features/library/primary-action"
import { ResponsibilityDialog } from "@/features/shared/responsibility"
import { formatList, useT } from "@/i18n"
import { pickFile, pickFolder, saveFile } from "@/lib/api"
import { allProjects, run, useAlias, useStore } from "@/lib/store"
import type { PlayChoice, PlayNext, SaveOption, SavesList } from "@/lib/types"
import { cn } from "@/lib/utils"
import { NoticeDialog } from "./notice-dialog"

/** Every dialog, opened from the store or by the launcher itself (a failed import, a move). */
export function Dialogs() {
  const dialog = useStore((s) => s.dialog)
  const open = useStore((s) => s.open)
  const snap = useStore((s) => s.snap)
  const close = () => open(null)
  if (!snap) return null
  return (
    <>
      <ImportDialog open={dialog?.kind === "import"} onClose={close} />
      {dialog?.kind === "uninstall" && <UninstallDialog repo={dialog.repo} onClose={close} />}
      {dialog?.kind === "setup" && <SetupDialog repo={dialog.repo} onClose={close} />}
      {dialog?.kind === "saves" && <SavesDialog repo={dialog.repo} onClose={close} />}
      {dialog?.kind === "presets" && <PresetsDialog repo={dialog.repo} choice={dialog.choice} onClose={close} />}
      <MoveDialog />
      <ImportErrorDialog />
      <NoticeDialog />
    </>
  )
}

function ImportDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  const t = useT()
  const busy = useStore((s) => s.snap!.job.running)
  const [mode, setMode] = useState<"link" | "zip">("link")
  const [link, setLink] = useState("")
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    if (open) {
      setLink("")
      setError(null)
    }
  }, [open])
  const acknowledged = useStore((s) => s.snap!.settings.acknowledged)
  const [askImport, setAskImport] = useState(false)
  const add = async () => {
    try {
      const { act } = await import("@/lib/api")
      await act({ type: "add_repo_link", link })
      toast.success(t("dialogs.import.added"))
      onClose()
    } catch (e) {
      setError(String(e))
    }
  }
  // The first repo import asks the player to take responsibility; later ones go straight through.
  const submit = () => (acknowledged.includes("import-repo") ? void add() : setAskImport(true))
  const [zipPath, setZipPath] = useState<string | null>(null)
  const importZip = (path: string) => {
    void run({ type: "import_build_zip", path })
    onClose()
  }
  return (
    <>
    <Dialog
      open={open}
      onOpenChange={(o) => !o && onClose()}
      title={t("dialogs.import.title")}
      description={t("dialogs.import.description")}
      footer={
        mode === "link" ? (
          <>
            <Button variant="primary" disabled={!link.trim()} onClick={submit}><Plus /> {t("dialogs.import.add")}</Button>
            <Button variant="ghost" onClick={onClose}>{t("common.cancel")}</Button>
          </>
        ) : (
          <>
            <Button
              variant="primary"
              disabled={busy}
              onClick={async () => {
                const path = await pickFile(t("dialogs.import.pickZip"), [{ name: t("dialogs.import.zip"), extensions: ["zip"] }])
                if (!path) return
                // A ZIP holds a program that runs on this PC, so it gets the same warning as a repo.
                if (acknowledged.includes("import-repo")) importZip(path)
                else {
                  setZipPath(path)
                  setAskImport(true)
                }
              }}
            >
              <FileArchive /> {t("dialogs.import.chooseZip")}
            </Button>
            <Button variant="ghost" onClick={onClose}>{t("common.cancel")}</Button>
          </>
        )
      }
    >
      <Segmented
        label={t("dialogs.import.what")}
        className="flex w-full"
        value={mode}
        options={[{ value: "link", label: t("dialogs.import.link") }, { value: "zip", label: t("dialogs.import.zip") }] as const}
        onChange={(v) => setMode(v)}
      />
      {mode === "link" ? (
        <div className="mt-5 space-y-2">
          <label className="relative block">
            <Link2 className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-fg-4" />
            <Input
              autoFocus
              value={link}
              onChange={(e) => {
                setLink(e.target.value)
                setError(null)
              }}
              onKeyDown={(e) => e.key === "Enter" && link.trim() && submit()}
              // i18n-ignore: an example address
              placeholder="https://github.com/owner/project"
              aria-label={t("dialogs.import.link")}
              className="h-10 pl-9"
            />
          </label>
          {error && <p className="text-secondary text-bad">{error}</p>}
          <p className="text-caption text-fg-3">{t("dialogs.import.hosts")}</p>
        </div>
      ) : (
        <p className="mt-5 text-body text-fg-2">{t("dialogs.import.zipHint")}</p>
      )}
      <div className="mt-5 flex items-start gap-3 rounded-lg border border-warn/20 bg-warn/8 px-3.5 py-3">
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warn" />
        <p className="text-secondary text-fg-2">{t("dialogs.import.trust")}</p>
      </div>
    </Dialog>
    <ResponsibilityDialog
      open={askImport}
      onOpenChange={(open) => {
        setAskImport(open)
        if (!open) setZipPath(null)
      }}
      id="import-repo"
      onAccept={() => {
        const pending = zipPath
        setZipPath(null)
        if (pending) importZip(pending)
        else void add()
      }}
    />
    </>
  )
}

function UninstallDialog({ repo, onClose }: { repo: string; onClose(): void }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const alias = useAlias()
  const p = allProjects(snap).find((q) => q.repo === repo)
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      title={p ? t("dialogs.uninstall.title", { name: alias(p).name }) : t("dialogs.uninstall.titleUnknown")}
      description={t("dialogs.uninstall.description")}
      footer={
        <>
          <Button variant="danger-solid" onClick={() => { void run({ type: "uninstall", repo }); onClose() }}><Trash2 /> {t("common.uninstall")}</Button>
          <Button variant="ghost" onClick={onClose}>{t("common.cancel")}</Button>
        </>
      }
    />
  )
}

function MoveDialog() {
  const t = useT()
  const pending = useStore((s) => s.snap!.pending_move)
  if (!pending) return null
  const answer = (a: "move" | "keep" | "cancel") => void run({ type: "confirm_move", answer: a })
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && answer("cancel")}
      title={t("dialogs.move.title")}
      description={t("dialogs.move.description")}
      footer={
        <>
          <Button variant="primary" onClick={() => answer("move")}>{t("dialogs.move.move")}</Button>
          <Button onClick={() => answer("keep")}>{t("dialogs.move.keep")}</Button>
          <Button variant="ghost" onClick={() => answer("cancel")}>{t("common.cancel")}</Button>
        </>
      }
    >
      <dl className="space-y-2 rounded-lg border border-line bg-fill-1 p-3 font-mono text-caption">
        <div className="flex gap-3"><dt className="w-10 text-fg-3">{t("dialogs.move.from")}</dt><dd className="break-all text-fg-2">{pending[0]}</dd></div>
        <div className="flex gap-3"><dt className="w-10 text-fg-3">{t("dialogs.move.to")}</dt><dd className="break-all text-fg">{pending[1]}</dd></div>
      </dl>
    </Dialog>
  )
}

function ImportErrorDialog() {
  const t = useT()
  const error = useStore((s) => s.snap!.job.import_error)
  if (!error) return null
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && void run({ type: "dismiss_import_error" })}
      title={t("dialogs.importError")}
      description={error}
      footer={<Button variant="primary" onClick={() => void run({ type: "dismiss_import_error" })}>{t("common.ok")}</Button>}
    />
  )
}

/** Everything a project needs before Install or Play, each with its fix, in order. */
function SetupDialog({ repo, onClose }: { repo: string; onClose(): void }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const alias = useAlias()
  const p = allProjects(snap).find((q) => q.repo === repo)
  if (!p) return null
  const tools = snap.tools
  const toolsBusy = snap.job.running && !snap.job.repo
  const needTools = !p.installed && p.kind === "git"
  const needFiles = !p.standalone
  const toolsOk = !needTools || !!tools?.ready
  const filesOk = !needFiles || snap.files.ok
  const ready = toolsOk && filesOk
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      eyebrow={alias(p).name}
      title={p.installed ? t("dialogs.setup.titlePlay") : t("dialogs.setup.titleInstall")}
      description={ready ? t("dialogs.setup.ready") : t("dialogs.setup.needed")}
      footer={
        <>
          <Button
            variant="primary"
            disabled={!ready || snap.job.running}
            onClick={() => {
              onClose()
              if (p.installed) void startPlay(p.repo)
              else void run({ type: "install", repo: p.repo })
            }}
          >
            {p.installed ? t("common.play") : t("common.install")}
          </Button>
          <Button variant="ghost" onClick={onClose}>{ready ? t("dialogs.setup.notNow") : t("common.later")}</Button>
        </>
      }
    >
      <ol className="space-y-2">
        {needTools && (
          <Step
            n={1}
            done={toolsOk}
            title={t("dialogs.setup.tools.title")}
            detail={!tools ? t("dialogs.setup.tools.checking") : tools.ready ? t("dialogs.setup.tools.ready") : toolsBusy ? snap.job.stage : t("dialogs.setup.tools.missing", { tools: formatList(tools.missing) })}
          >
            {tools && !tools.ready && (
              <Button size="sm" disabled={snap.job.running} onClick={() => void run({ type: "install_tools" })}>
                {toolsBusy ? <Spinner className="text-white" /> : <Download />} {toolsBusy ? t("dialogs.setup.tools.installing") : t("common.install")}
              </Button>
            )}
          </Step>
        )}
        {needFiles && (
          <Step
            n={needTools ? 2 : 1}
            done={filesOk}
            title={t("dialogs.setup.files.title")}
            detail={snap.files.ok ? t("dialogs.setup.files.found") : snap.files.unplugged ? t("dialogs.setup.files.unplugged") : snap.files.find_error ?? (snap.files.chosen ? t("dialogs.setup.files.missing", { files: formatList(snap.files.missing) }) : t("dialogs.setup.files.hint"))}
          >
            {!filesOk && (
              <>
                <Button size="sm" variant="ghost" disabled={snap.files.searching} onClick={() => void run({ type: "find_game_files" })}>
                  {snap.files.searching ? <Spinner /> : <Search />} {t("common.find")}
                </Button>
                <Button size="sm" onClick={async () => { const d = await pickFolder(t("dialogs.setup.files.pick")); if (d) void run({ type: "set_game_dir", path: d }) }}>
                  <FolderOpen /> {t("common.choose")}
                </Button>
              </>
            )}
          </Step>
        )}
      </ol>
    </Dialog>
  )
}

function Step({ n, done, title, detail, children }: { n: number; done: boolean; title: string; detail: string; children?: React.ReactNode }) {
  return (
    <li className="flex items-center gap-3.5 rounded-lg border border-line bg-fill-1 p-3.5">
      <span className={cn("grid size-7 shrink-0 place-items-center rounded-full text-secondary font-semibold", done ? "bg-ok/15 text-ok" : "bg-fill-3 text-fg-2")}>
        {done ? <Check className="size-4" /> : n}
      </span>
      <div className="min-w-0 flex-1">
        <p className="text-body font-medium text-fg">{title}</p>
        <p className="text-secondary text-fg-3">{detail}</p>
      </div>
      <div className="flex shrink-0 gap-1.5">{children}</div>
    </li>
  )
}

function PresetsDialog({ repo, choice, onClose }: { repo: string; choice: PlayChoice; onClose(): void }) {
  const open = useStore((s) => s.open)
  const t = useT()
  const [picked, setPicked] = useState<number | null>(choice.recommended)
  const rows = [{ title: t("dialogs.presets.current"), detail: t("dialogs.presets.currentDetail") }, ...choice.options]
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      title={t("dialogs.presets.title")}
      description={t("dialogs.presets.description")}
      footer={
        <>
          <Button
            variant="primary"
            onClick={async () => {
              const next = await run<PlayNext>({ type: "confirm_preset", repo, index: picked })
              if (!next) return
              if (next.next === "saves") open({ kind: "saves", repo })
              else onClose()
            }}
          >
            {t("common.continue")}
          </Button>
          <Button variant="ghost" onClick={onClose}>{t("common.cancel")}</Button>
        </>
      }
    >
      <div role="radiogroup" className="space-y-1.5">
        {rows.map((r, i) => {
          const index = i === 0 ? null : i - 1
          return <Choice key={i} selected={picked === index} onSelect={() => setPicked(index)} title={r.title} detail={r.detail} />
        })}
      </div>
    </Dialog>
  )
}

function Choice({ selected, onSelect, title, detail, icon }: { selected: boolean; onSelect(): void; title: string; detail?: string; icon?: React.ReactNode }) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className={cn(
        "flex w-full cursor-pointer items-center gap-3 rounded-lg border px-3.5 py-3 text-left transition-colors duration-150",
        selected ? "border-accent/50 bg-accent-soft" : "border-line bg-fill-1 hover:bg-fill-2",
      )}
    >
      <span className={cn("grid size-4 shrink-0 place-items-center rounded-full border", selected ? "border-accent bg-accent" : "border-fg-4")}>
        {selected && <span className="size-1.5 rounded-full bg-white" />}
      </span>
      {icon}
      <span className="min-w-0 flex-1">
        <span className="block truncate text-body font-medium text-fg">{title}</span>
        {detail && <span className="block truncate text-secondary text-fg-3">{detail}</span>}
      </span>
    </button>
  )
}

type SaveTab = "this_build" | "other_builds" | "favorites"

function SavesDialog({ repo, onClose }: { repo: string; onClose(): void }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const screenshot = useStore((s) => s.screenshot)
  const alias = useAlias()
  const p = allProjects(snap).find((q) => q.repo === repo)
  const [list, setList] = useState<SavesList | null>(null)
  const [tab, setTab] = useState<SaveTab>("this_build")
  const [index, setIndex] = useState(0)
  const [naming, setNaming] = useState<string | null>(null)
  const [deleting, setDeleting] = useState(false)
  const load = async (keep?: { tab: SaveTab; title?: string }) => {
    const l = await run<SavesList>({ type: "saves_list", repo })
    if (!l) return
    setList(l)
    if (keep) {
      setTab(keep.tab)
      setIndex(Math.max(0, keep.title ? l[keep.tab].findIndex((o) => o.title === keep.title) : 0))
    } else {
      setIndex(l.has_current ? 0 : 1)
    }
  }
  useEffect(() => {
    void load()
  }, [repo])
  const options: SaveOption[] = list?.[tab] ?? []
  const option = options[index]
  const exportable = option?.path ?? option?.source ?? null
  const label = option?.kind === "continue" ? t("common.continue") : option?.kind === "fresh" ? t("dialogs.saves.fresh") : t("dialogs.saves.copy")
  const empty = tab === "other_builds" ? t("dialogs.saves.emptyOther") : t("dialogs.saves.emptyFavorites")
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      eyebrow={p ? alias(p).name : undefined}
      title={t("dialogs.saves.title")}
      description={t("dialogs.saves.description")}
      className="w-[min(560px,calc(100vw-48px))]"
      footer={
        deleting ? (
          <>
            <Button
              variant="danger-solid"
              onClick={async () => {
                if (exportable) await run({ type: "delete_save", path: exportable }, t("dialogs.saves.deleted"))
                setDeleting(false)
                void load({ tab })
              }}
            >
              <Trash2 /> {t("dialogs.saves.deleteSave")}
            </Button>
            <Button variant="ghost" onClick={() => setDeleting(false)}>{t("dialogs.saves.keep")}</Button>
          </>
        ) : naming != null ? (
          <>
            <Button
              variant="primary"
              disabled={!naming.trim() || !option?.source}
              onClick={async () => {
                const ok = await run({ type: "checkpoint", name: naming, source: option!.source! })
                if (ok !== undefined) {
                  toast.success(t("dialogs.saves.favorited"))
                  setNaming(null)
                  void load({ tab: "favorites", title: naming.trim() })
                }
              }}
            >
              <Star /> {t("dialogs.saves.saveCheckpoint")}
            </Button>
            <Button variant="ghost" onClick={() => setNaming(null)}>{t("common.back")}</Button>
          </>
        ) : (
          <>
            <Button
              variant="primary"
              disabled={!option || !snap.files.ok || snap.job.running}
              onClick={() => {
                if (!option) return
                const choice = option.kind === "copy" ? { kind: "copy" as const, path: option.path! } : { kind: option.kind }
                void run({ type: "start_with_save", repo, choice })
                onClose()
              }}
            >
              {label}
            </Button>
            <Button variant="ghost" onClick={onClose}>{t("common.cancel")}</Button>
            <div className="mr-auto flex gap-1">
              {!screenshot && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={async () => {
                    const path = await pickFile(t("dialogs.saves.importZip"), [{ name: t("dialogs.saves.importType"), extensions: ["zip"] }])
                    if (path) {
                      const name = await run<string>({ type: "import_save_zip", path })
                      if (name) {
                        toast.success(t("dialogs.saves.imported", { name }))
                        void load({ tab: "favorites", title: name })
                      }
                    }
                  }}
                >
                  <Download /> {t("common.import")}
                </Button>
              )}
            </div>
          </>
        )
      }
    >
      <Segmented
        label={t("dialogs.saves.tabs.label")}
        className="flex w-full"
        value={tab}
        options={[{ value: "this_build", label: t("dialogs.saves.tabs.thisBuild") }, { value: "other_builds", label: t("dialogs.saves.tabs.otherBuilds") }, { value: "favorites", label: t("dialogs.saves.tabs.favorites") }] as const}
        onChange={(next) => {
          setTab(next)
          setIndex(0)
          setNaming(null)
          setDeleting(false)
        }}
      />
      <div role="radiogroup" className="mt-4 max-h-64 space-y-1.5 overflow-y-auto">
        {!list && <p className="flex items-center gap-2 text-secondary text-fg-3"><Spinner /> {t("dialogs.saves.looking")}</p>}
        {list && options.length === 0 && <p className="py-6 text-center text-secondary text-fg-3">{empty}</p>}
        {options.map((o, i) => (
          <Choice
            key={`${tab}-${i}`}
            selected={i === index}
            onSelect={() => {
              setIndex(i)
              setNaming(null)
              setDeleting(false)
            }}
            title={screenshot && tab !== "this_build" ? t("dialogs.saves.example", { n: String(i + 1).padStart(2, "0") }) : o.title}
            detail={o.detail}
            icon={tab === "favorites" ? <Star className="size-4 shrink-0 text-fg-3" /> : undefined}
          />
        ))}
      </div>
      {naming != null ? (
        <div className="mt-4 space-y-1.5">
          <Input autoFocus value={naming} onChange={(e) => setNaming(e.target.value)} placeholder={t("dialogs.saves.checkpointExample")} aria-label={t("dialogs.saves.checkpointName")} />
          <p className="text-caption text-fg-3">{t("dialogs.saves.checkpointRule")}</p>
        </div>
      ) : deleting ? (
        <p className="mt-4 text-secondary text-fg-2">{t("dialogs.saves.confirmDelete", { name: option?.title ?? "" })}</p>
      ) : (
        option && !screenshot && (
          <div className="mt-3 flex flex-wrap gap-1">
            {option.source && <Button size="sm" variant="ghost" onClick={() => setNaming("")}><Star /> {t("dialogs.saves.favorite")}</Button>}
            {exportable && (
              <Button
                size="sm"
                variant="ghost"
                onClick={async () => {
                  // i18n-ignore: the suggested file name stays as it is
                  const path = await saveFile(t("dialogs.saves.exportTitle"), "VILauncher Save.zip", [{ name: t("dialogs.saves.exportType"), extensions: ["zip"] }])
                  if (path) void run({ type: "export_save", source: exportable, path }, t("dialogs.saves.exported"))
                }}
              >
                <Download /> {t("common.export")}
              </Button>
            )}
            {exportable && <Button size="sm" variant="ghost" className="text-bad hover:text-bad" onClick={() => setDeleting(true)}><Trash2 /> {t("common.delete")}</Button>}
          </div>
        )
      )}
      {!snap.files.ok && (
        <p className="mt-3 flex items-center gap-2 text-secondary text-warn"><CheckCircle2 className="size-3.5" /> {t("dialogs.saves.needFiles")}</p>
      )}
    </Dialog>
  )
}
