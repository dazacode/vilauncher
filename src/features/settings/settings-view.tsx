import { ExternalLink, FolderOpen, Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Group, Row } from "@/components/ui/group"
import { Input, Textarea } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { GraphicsGroups } from "@/features/library/graphics-tab"
import { openUrl, pickFolder } from "@/lib/api"
import { useDraft } from "@/lib/hooks"
import { run, useStore } from "@/lib/store"

export function SettingsView() {
  const snap = useStore((s) => s.snap)!
  const screenshot = useStore((s) => s.screenshot)
  const setScreenshot = useStore((s) => s.setScreenshot)
  const s = snap.settings
  const [gameDir, setGameDir] = useDraft(s.game_dir, (v) => void run({ type: "set_game_dir", path: v }), 600)
  const [args, setArgs] = useDraft(s.launch_args, (v) => void run({ type: "set_settings", patch: { launch_args: v } }))
  const [env, setEnv] = useDraft(s.extra_env, (v) => void run({ type: "set_settings", patch: { extra_env: v } }))
  const busy = snap.job.running
  const u = snap.updater
  return (
    <div className="h-full overflow-y-auto">
      <div className="drag h-12" />
      <div className="mx-auto max-w-[760px] space-y-10 px-10 pb-16">
        <header>
          <h1 className="font-display text-page font-bold tracking-[-0.02em] text-fg">Settings</h1>
          <p className="mt-1 text-body text-fg-3">{snap.brand.name} {snap.version}</p>
        </header>

        <Group title="Privacy">
          <Row label="Screenshot mode" hint="Shows example names and hides paths, notes and logs, for sharing pictures of the launcher.">
            <Switch checked={screenshot} onCheckedChange={setScreenshot} aria-label="Screenshot mode" />
          </Row>
        </Group>

        {screenshot ? (
          <p className="text-body text-fg-3">The other settings are hidden in Screenshot mode.</p>
        ) : (
          <>
            <Group title="Game files" description="The folder with game_scarlett_master.exe from your own copy of the game. VILauncher never downloads game files.">
              <Row stacked label="Folder">
                <Input value={gameDir} onChange={(e) => setGameDir(e.target.value)} placeholder="Paste the folder's path" className="font-mono text-secondary" aria-label="Game folder" />
                <Button size="md" onClick={async () => { const d = await pickFolder("The folder with your game files"); if (d) void run({ type: "set_game_dir", path: d }) }}>
                  <FolderOpen /> Choose…
                </Button>
                <Button size="md" variant="ghost" disabled={snap.files.searching} onClick={() => void run({ type: "find_game_files" })}>
                  {snap.files.searching ? <Spinner /> : <Search />} {snap.files.searching ? "Searching" : "Find"}
                </Button>
              </Row>
              <Row label="Status" hint={snap.files.find_error ?? undefined}>
                <FilesStatus />
              </Row>
            </Group>

            <Group title="Build tools" description="Projects are downloaded with Git and built with Rust and the C++ build tools, once, on this PC.">
              <Row label={!snap.tools ? "Checking…" : snap.tools.ready ? "Git, Rust and the C++ build tools are ready" : `Missing ${snap.tools.missing.join(", ")}`} hint={snap.tools && !snap.tools.ready ? "Installs with winget; Windows asks for permission." : undefined}>
                {snap.tools && !snap.tools.ready && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "install_tools" })}>Install missing tools</Button>}
                {snap.tools?.ready && <Dot ok />}
              </Row>
            </Group>

            <div className="space-y-8">
              <div className="px-1">
                <h2 className="font-display text-section font-semibold text-fg">Default graphics</h2>
                <p className="mt-0.5 text-secondary text-fg-3">Every project uses these unless you change its own in its Graphics tab.</p>
              </div>
              <GraphicsGroups graphics={s.graphics} onChange={(graphics) => void run({ type: "set_default_graphics", graphics })} />
            </div>

            <Group title="Shaders and launch" description="VIctory can translate the game's shaders before you play, so the game doesn't stop to do it on the way. Each project keeps its own cache.">
              <Row label="Warm up shaders before playing" hint="Runs each time you press Play. The first one takes minutes and about 1.3 GB.">
                <Switch checked={s.warm_before_play} onCheckedChange={(v) => void run({ type: "set_settings", patch: { warm_before_play: v } })} aria-label="Warm up shaders before playing" />
              </Row>
              <Row label="Also build the GPU pipelines" hint="Slow: about 20 minutes on 16 threads, hours after a translator update.">
                <Switch checked={s.warm_pipelines} onCheckedChange={(v) => void run({ type: "set_settings", patch: { warm_pipelines: v } })} aria-label="Also build the GPU pipelines" />
              </Row>
              <Row stacked label="Default launch arguments" hint="Added to victory run, for example --strict. Arguments for the game go after --.">
                <Input value={args} onChange={(e) => setArgs(e.target.value)} placeholder="--strict -- -title-argument" className="font-mono text-secondary" aria-label="Default launch arguments" />
              </Row>
              <Row stacked label="Default launch switches" hint="One VICTORY_NAME=value per line.">
                <Textarea value={env} onChange={(e) => setEnv(e.target.value)} placeholder="VICTORY_NAME=value" rows={3} className="font-mono text-secondary" aria-label="Default launch switches" />
              </Row>
            </Group>

            <Group title="Storage">
              <Row label="Install folder" hint={<span className="font-mono break-all" data-selectable>{s.workspace}</span>}>
                <Button size="sm" disabled={busy} onClick={async () => { const d = await pickFolder("Where projects are downloaded and built"); if (d) void run({ type: "choose_install_dir", path: d }) }}>Change…</Button>
                {s.install_dir_custom && <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run({ type: "use_default_install_dir" })}>Use default</Button>}
              </Row>
              <Row label="Background pictures" hint="Put .jpg or .png pictures here and they show behind every project.">
                <Button size="sm" onClick={() => void run({ type: "open", what: "backgrounds" })}><FolderOpen /> Open folder</Button>
              </Row>
            </Group>
          </>
        )}

        <Group title="Updates">
          <Row
            label={u.state === "available" ? `${u.tag} is available` : u.state === "downloaded" ? "The update is ready" : u.state === "checking" ? "Checking for updates…" : u.state === "downloading" ? "Downloading and verifying…" : u.state === "failed" ? "The update check failed" : "You're up to date"}
            hint={u.restart_error ?? u.error ?? `Version ${snap.version}`}
          >
            {u.state === "available" && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "download_update" })}>Update and restart</Button>}
            {u.state === "downloaded" && <Button size="sm" variant="primary" disabled={busy} onClick={() => void run({ type: "restart_to_update" })}>Restart now</Button>}
            {(u.state === "checking" || u.state === "downloading") && <Spinner />}
            {!["available", "downloaded", "checking", "downloading"].includes(u.state) && <Button size="sm" onClick={() => void run({ type: "check_update" })}>Check now</Button>}
            <Button size="sm" variant="ghost" onClick={() => void openUrl(u.releases)}>Release notes</Button>
          </Row>
        </Group>

        <Group title="About">
          <Row label={`${snap.brand.name} by the ${snap.brand.team}`} hint="The library comes from VIPortLive, built on PortWatch's tracker. The projects are their authors' work, built on VIctory by Marvisak and its contributors.">
            <Button size="sm" variant="ghost" onClick={() => void openUrl(snap.brand.site)}><ExternalLink /> {snap.brand.site_label}</Button>
          </Row>
          <Row label="Based on Mandyhub by Zettrox" hint="Free software under the GNU General Public License v3 or later. Redesign of the original by ellion369; save picker, checkpoints, presets and build tracking by TheGameOnAddict.">
            <Button size="sm" variant="ghost" onClick={() => void openUrl(snap.brand.source)}><ExternalLink /> Source code</Button>
          </Row>
          <Row label="Legal notice" hint="You provide your own copy of the game, and you are responsible for what you install, build and run. VILauncher only manages the projects.">
            <Button size="sm" variant="ghost" onClick={() => useStore.getState().open({ kind: "notice" })}>Read</Button>
          </Row>
        </Group>
      </div>
    </div>
  )
}

function Dot({ ok }: { ok: boolean }) {
  return <span className={ok ? "size-2 rounded-full bg-ok" : "size-2 rounded-full bg-bad"} aria-hidden />
}

function FilesStatus() {
  const f = useStore((s) => s.snap!.files)
  if (!f.chosen) return <span className="text-secondary text-fg-3">No folder chosen</span>
  if (f.ok) return <span className="flex items-center gap-2 text-secondary text-ok"><Dot ok /> All game files found</span>
  if (f.empty) return <span className="flex items-center gap-2 text-secondary text-bad"><Dot ok={false} /> This folder is empty</span>
  return (
    <span className="flex max-w-80 flex-wrap items-center justify-end gap-x-2 text-secondary text-bad">
      <Dot ok={false} /> Missing <span className="font-mono text-caption text-fg-2">{f.missing.join(", ")}</span>
    </span>
  )
}
