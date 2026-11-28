import { useCallback, useEffect, useState } from "react"
import { ExternalLink, FolderOpen, RefreshCw, Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Group, Row } from "@/components/ui/group"
import { Input, Textarea } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { GraphicsGroups } from "@/features/library/graphics-tab"
import { ControllerKeysGroup } from "@/features/settings/controller-keys"
import { LanguageSelect } from "@/features/shared/language-select"
import { BackdropPicker } from "./backdrop-picker"
import { ThemeRows } from "./theme-picker"
import { ResponsibilityDialog } from "@/features/shared/responsibility"
import { formatBytes, formatList, Trans, useT } from "@/i18n"
import { openUrl, pickFile, pickFolder } from "@/lib/api"
import { useDraft } from "@/lib/hooks"
import { run, useStore } from "@/lib/store"
import type { StorageView } from "@/lib/types"

export function SettingsView() {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const screenshot = useStore((s) => s.screenshot)
  const setScreenshot = useStore((s) => s.setScreenshot)
  const s = snap.settings
  const [askAgents, setAskAgents] = useState(false)
  // Turning agents on needs the warning once; the switch only moves after Continue.
  const turnOnAgents = () =>
    s.acknowledged.includes("agent-access") ? void run({ type: "set_agent_access", enabled: true }) : setAskAgents(true)
  const [gameDir, setGameDir] = useDraft(s.game_dir, (v) => void run({ type: "set_game_dir", path: v }), 600)
  const [args, setArgs] = useDraft(s.launch_args, (v) => void run({ type: "set_settings", patch: { launch_args: v } }))
  const [env, setEnv] = useDraft(s.extra_env, (v) => void run({ type: "set_settings", patch: { extra_env: v } }))
  const busy = snap.job.running || snap.active_builds > 0
  const u = snap.updater
  return (
    <div className="h-full overflow-y-auto">
      <div data-tauri-drag-region="deep" className="drag h-6" />
      <div className="mx-auto max-w-[760px] space-y-10 px-10 pb-16">
        <header>
          <h1 className="font-display text-page font-bold tracking-[-0.02em] text-fg">{t("settings.title")}</h1>
          <p className="mt-1 text-body text-fg-3">{snap.brand.name} {snap.version}</p>
        </header>

        <Group title={t("settings.appearance.title")} description={t("settings.appearance.description")}>
          <ThemeRows />
          <LanguageRow />
          <BackdropPicker />
        </Group>

        <Group title={t("settings.privacy.title")}>
          <Row label={t("settings.privacy.screenshot")} hint={t("settings.privacy.screenshotHint")}>
            <Switch checked={screenshot} onCheckedChange={setScreenshot} aria-label={t("settings.privacy.screenshot")} />
          </Row>
        </Group>

        {screenshot ? (
          <p className="text-body text-fg-3">{t("settings.privacy.othersHidden")}</p>
        ) : (
          <>
            <Group title={t("settings.gameFiles.title")} description={t("settings.gameFiles.description")}>
              <Row stacked label={t("settings.gameFiles.folder")}>
                <Input value={gameDir} onChange={(e) => setGameDir(e.target.value)} placeholder={t("settings.gameFiles.placeholder")} className="font-mono text-secondary" aria-label={t("settings.gameFiles.label")} />
                <Button size="md" onClick={async () => { const d = await pickFolder(t("settings.gameFiles.pickTitle")); if (d) void run({ type: "set_game_dir", path: d }) }}>
                  <FolderOpen /> {t("common.choose")}
                </Button>
                <Button size="md" variant="ghost" disabled={snap.files.searching} onClick={() => void run({ type: "find_game_files" })}>
                  {snap.files.searching ? <Spinner /> : <Search />} {snap.files.searching ? t("settings.gameFiles.searching") : t("settings.gameFiles.find")}
                </Button>
              </Row>
              <Row label={t("settings.gameFiles.status")} hint={snap.files.find_error ?? undefined}>
                <FilesStatus />
              </Row>
            </Group>

            <Group title={t("settings.tools.title")} description={t("settings.tools.description")}>
              <Row
                label={!snap.tools ? t("common.checking") : snap.tools.ready ? t("settings.tools.ready") : t("settings.tools.missing", { tools: formatList(snap.tools.missing) })}
                hint={snap.tools && !snap.tools.ready ? t("settings.tools.hint") : undefined}
              >
                {snap.tools && !snap.tools.ready && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "install_tools" })}>{t("settings.tools.install")}</Button>}
                {snap.tools?.ready && <Dot ok />}
              </Row>
            </Group>

            <div className="space-y-8">
              <div className="px-1">
                <h2 className="font-display text-section font-semibold text-fg">{t("settings.graphics.title")}</h2>
                <p className="mt-0.5 text-secondary text-fg-3">{t("settings.graphics.description")}</p>
              </div>
              <GraphicsGroups graphics={s.graphics} onChange={(graphics) => void run({ type: "set_default_graphics", graphics })} />
            </div>

            <Group title={t("settings.upscaling.title")} description={t("settings.upscaling.description")}>
              <Row
                label={t("settings.nvidia.title")}
                hint={s.nvidia.busy ? s.nvidia.status ?? "" : s.nvidia.status ?? (s.nvidia.ready ? t("settings.nvidia.ready") : t("settings.nvidia.missing"))}
              >
                <Button size="sm" variant="primary" disabled={s.nvidia.busy} onClick={() => void run({ type: "get_nvidia_files" })}>
                  {t("settings.nvidia.get")}
                </Button>
              </Row>
              <Row label={t("settings.fsr.title")} hint={s.fsr_dll ? (s.fsr_ok ? s.fsr_dll : t("settings.fsr.gone")) : t("settings.fsr.none")}>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    onClick={async () => {
                      const path = await pickFile(t("settings.fsr.pickTitle"), [{ name: t("settings.fsr.filter"), extensions: ["dll"] }])
                      if (path) void run({ type: "set_fsr_dll", path })
                    }}
                  >
                    {t("settings.fsr.choose")}
                  </Button>
                  {s.fsr_dll && (
                    <Button size="sm" variant="ghost" onClick={() => void run({ type: "set_fsr_dll", path: null })}>
                      {t("settings.fsr.clear")}
                    </Button>
                  )}
                </div>
              </Row>
            </Group>

            <Group title={t("settings.shaders.title")} description={t("settings.shaders.description")}>
              <Row label={t("settings.shaders.warm")} hint={t("settings.shaders.warmHint")}>
                <Switch checked={s.warm_before_play} onCheckedChange={(v) => void run({ type: "set_settings", patch: { warm_before_play: v } })} aria-label={t("settings.shaders.warm")} />
              </Row>
              <Row label={t("settings.shaders.pipelines")} hint={t("settings.shaders.pipelinesHint")}>
                <Switch checked={s.warm_pipelines} onCheckedChange={(v) => void run({ type: "set_settings", patch: { warm_pipelines: v } })} aria-label={t("settings.shaders.pipelines")} />
              </Row>
              <Row stacked label={t("settings.shaders.args")} hint={t("settings.shaders.argsHint")}>
                <Input
                  value={args}
                  onChange={(e) => setArgs(e.target.value)}
                  // i18n-ignore: an example command line
                  placeholder="--strict -- -title-argument"
                  className="font-mono text-secondary"
                  aria-label={t("settings.shaders.args")}
                />
              </Row>
              <Row stacked label={t("settings.shaders.switches")} hint={t("settings.shaders.switchesHint")}>
                <Textarea
                  value={env}
                  onChange={(e) => setEnv(e.target.value)}
                  // i18n-ignore: an environment variable, as it is written
                  placeholder="VICTORY_NAME=value"
                  rows={3}
                  className="font-mono text-secondary"
                  aria-label={t("settings.shaders.switches")}
                />
              </Row>
            </Group>

            <Group title={t("settings.storage.title")}>
              <Row label={t("settings.storage.installFolder")} hint={<span className="font-mono break-all" data-selectable>{s.workspace}</span>}>
                <Button size="sm" disabled={busy} onClick={async () => { const d = await pickFolder(t("settings.storage.pickTitle")); if (d) void run({ type: "choose_install_dir", path: d }) }}>{t("common.change")}</Button>
                {s.install_dir_custom && <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run({ type: "use_default_install_dir" })}>{t("settings.storage.useDefault")}</Button>}
              </Row>
              <Row label={t("settings.storage.backgrounds")} hint={t("settings.storage.backgroundsHint")}>
                <Button size="sm" onClick={() => void run({ type: "open", what: "backgrounds" })}><FolderOpen /> {t("common.openFolder")}</Button>
              </Row>
            </Group>

            <DiskSpace />
          </>
        )}

        <ControllerKeysGroup />

        <Group title={t("settings.agents.title")}>
          <Row label={t("settings.agents.access")} hint={t("settings.agents.hint")}>
            <Switch
              checked={snap.settings.agent_access}
              onCheckedChange={(v) => (v ? turnOnAgents() : void run({ type: "set_agent_access", enabled: false }))}
              aria-label={t("settings.agents.access")}
            />
          </Row>
          <ResponsibilityDialog
            open={askAgents}
            onOpenChange={setAskAgents}
            id="agent-access"
            onAccept={() => void run({ type: "set_agent_access", enabled: true })}
          />
        </Group>

        <Group title={t("settings.updates.title")}>
          <Row
            label={
              u.state === "available"
                ? t("settings.updates.available", { tag: u.tag ?? "" })
                : u.state === "downloaded"
                  ? t("settings.updates.ready")
                  : u.state === "checking"
                    ? t("settings.updates.checking")
                    : u.state === "downloading"
                      ? t("settings.updates.downloading")
                      : u.state === "failed"
                        ? t("settings.updates.failed")
                        : t("settings.updates.current")
            }
            hint={u.restart_error ?? u.error ?? t("settings.updates.version", { version: snap.version })}
          >
            {u.state === "available" && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "download_update" })}>{t("settings.updates.install")}</Button>}
            {u.state === "downloaded" && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "restart_to_update" })}>{t("settings.updates.restart")}</Button>}
            {(u.state === "checking" || u.state === "downloading") && <Spinner />}
            {!["available", "downloaded", "checking", "downloading"].includes(u.state) && <Button size="sm" onClick={() => void run({ type: "check_update" })}>{t("settings.updates.check")}</Button>}
            <Button size="sm" variant="ghost" onClick={() => void openUrl(u.releases)}>{t("settings.updates.notes")}</Button>
          </Row>
          <Row label={t("settings.updates.warn")} hint={t("settings.updates.warnHint")}>
            <Switch
              checked={!snap.settings.skip_update_warning}
              onCheckedChange={(v) => void run({ type: "set_update_warning", show: v })}
              aria-label={t("settings.updates.warn")}
            />
          </Row>
        </Group>

        <Group title={t("settings.about.title")}>
          <Row label={t("settings.about.by", { name: snap.brand.name, team: snap.brand.team })} hint={t("settings.about.credits")}>
            <Button size="sm" variant="ghost" onClick={() => void openUrl(snap.brand.site)}><ExternalLink /> {snap.brand.site_label}</Button>
          </Row>
          <Row label={t("settings.about.basedOn")} hint={t("settings.about.license")}>
            <Button size="sm" variant="ghost" onClick={() => void openUrl(snap.brand.source)}><ExternalLink /> {t("settings.about.source")}</Button>
          </Row>
          <Row label={t("settings.about.legal")} hint={t("settings.about.legalHint")}>
            <Button size="sm" variant="ghost" onClick={() => useStore.getState().open({ kind: "notice" })}>{t("common.read")}</Button>
          </Row>
        </Group>
      </div>
    </div>
  )
}

function LanguageRow() {
  const t = useT()
  return (
    <Row label={t("settings.language.label")} hint={t("settings.language.hint")}>
      <LanguageSelect />
    </Row>
  )
}

/** What VILauncher takes on disk, largest first, and the parts that are safe to clear.
 *  Projects with nothing on disk are left out. Saves are counted but never offered. */
function DiskSpace() {
  const t = useT()
  const busy = useStore((s) => s.snap!.job.running || s.snap!.active_builds > 0)
  const [view, setView] = useState<StorageView | null>(null)
  const [scanning, setScanning] = useState(false)
  // The button waiting for its second click, as "cache:<slug>" or "previous:<slug>".
  const [armed, setArmed] = useState<string | null>(null)

  const scan = useCallback(async () => {
    setScanning(true)
    const next = await run<StorageView>({ type: "storage_scan" })
    if (next) setView(next)
    setScanning(false)
  }, [])

  useEffect(() => {
    void scan()
  }, [scan])

  const clear = async (kind: "cache" | "previous", repo: string, key: string) => {
    if (armed !== key) {
      setArmed(key)
      return
    }
    setArmed(null)
    await run(
      kind === "cache" ? { type: "clear_cache", repo } : { type: "clear_previous", repo },
      kind === "cache" ? t("settings.disk.cacheCleared") : t("settings.disk.previousCleared"),
    )
    await scan()
  }

  const rows = (view?.projects ?? [])
    .map((p) => ({ ...p, total: p.sources + p.builds + p.previous + p.cache + p.saves }))
    .filter((p) => p.total > 0)
    .sort((a, b) => b.total - a.total)
  const freeable = rows.reduce((sum, p) => sum + p.previous + p.cache, 0)

  return (
    <Group title={t("settings.disk.title")} description={t("settings.disk.description")}>
      <Row
        label={view ? t("settings.disk.total", { size: formatBytes(view.total_bytes) }) : t("settings.disk.measuring")}
        hint={
          view && (
            <>
              {view.free_bytes != null && t("settings.disk.free", { free: formatBytes(view.free_bytes) })}{" "}
              {freeable > 0 ? t("settings.disk.freeable", { size: formatBytes(freeable) }) : t("settings.disk.nothingToFree")}
            </>
          )
        }
      >
        <Button size="sm" variant="ghost" disabled={scanning} onClick={() => void scan()}>
          {scanning ? <Spinner /> : <RefreshCw />} {t("common.refresh")}
        </Button>
      </Row>
      {view && rows.length === 0 && <p className="px-4 py-3 text-secondary text-fg-3">{t("settings.disk.empty")}</p>}
      {rows.map((row) => {
        const cacheKey = `cache:${row.slug}`
        const previousKey = `previous:${row.slug}`
        const parts = [
          row.builds > 0 && t("settings.disk.build", { size: formatBytes(row.builds) }),
          row.sources > 0 && t("settings.disk.sources", { size: formatBytes(row.sources) }),
          row.previous > 0 && t("settings.disk.previous", { size: formatBytes(row.previous) }),
          row.cache > 0 && t("settings.disk.cache", { size: formatBytes(row.cache) }),
          row.saves > 0 && t("settings.disk.saves", { size: formatBytes(row.saves) }),
        ].filter(Boolean)
        return (
          <div key={row.slug} className="px-4 py-3">
            <div className="flex items-baseline justify-between gap-4">
              <span className="min-w-0 truncate text-body text-fg">{row.label}</span>
              <span className="shrink-0 text-body font-medium tabular-nums text-fg">{formatBytes(row.total)}</span>
            </div>
            <p className="mt-0.5 text-secondary text-fg-3">{parts.join(" · ")}</p>
            {(row.previous > 0 || row.cache > 0) && (
              <div className="mt-2 flex flex-wrap gap-2">
                {row.previous > 0 && (
                  <Button size="sm" variant="ghost" disabled={busy || scanning} onClick={() => void clear("previous", row.repo, previousKey)}>
                    {armed === previousKey ? t("common.confirm") : t("settings.disk.clearPrevious", { size: formatBytes(row.previous) })}
                  </Button>
                )}
                {row.cache > 0 && (
                  <Button size="sm" variant="ghost" disabled={busy || scanning} onClick={() => void clear("cache", row.repo, cacheKey)}>
                    {armed === cacheKey ? t("common.confirm") : t("settings.disk.clearCache", { size: formatBytes(row.cache) })}
                  </Button>
                )}
              </div>
            )}
          </div>
        )
      })}
      {view && view.other_bytes > 0 && (
        <div className="flex items-baseline justify-between gap-4 px-4 py-3">
          <span className="text-body text-fg-2">{t("settings.disk.elsewhere")}</span>
          <span className="shrink-0 text-body tabular-nums text-fg-2">{formatBytes(view.other_bytes)}</span>
        </div>
      )}
    </Group>
  )
}

function Dot({ ok }: { ok: boolean }) {
  return <span className={ok ? "size-2 rounded-full bg-ok" : "size-2 rounded-full bg-bad"} aria-hidden />
}

function FilesStatus() {
  const t = useT()
  const f = useStore((s) => s.snap!.files)
  if (!f.chosen) return <span className="text-secondary text-fg-3">{t("settings.gameFiles.none")}</span>
  if (f.ok) return <span className="flex items-center gap-2 text-secondary text-ok"><Dot ok /> {t("settings.gameFiles.ok")}</span>
  if (f.unplugged) return <span className="flex items-center gap-2 text-secondary text-bad"><Dot ok={false} /> {t("settings.gameFiles.unplugged")}</span>
  if (f.empty) return <span className="flex items-center gap-2 text-secondary text-bad"><Dot ok={false} /> {t("settings.gameFiles.empty")}</span>
  return (
    <span className="flex max-w-80 flex-wrap items-center justify-end gap-x-2 text-secondary text-bad">
      <Dot ok={false} />
      <Trans k="settings.gameFiles.missing" values={{ files: f.missing.join(", ") }} tags={{ files: (list) => <span className="font-mono text-caption text-fg-2">{list}</span> }} />
    </span>
  )
}
