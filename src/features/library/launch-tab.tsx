import { ClipboardPaste, Copy, Download, FileUp, MoreHorizontal, Save, Trash2 } from "lucide-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Group, Row } from "@/components/ui/group"
import { Input, Textarea } from "@/components/ui/input"
import { Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "@/components/ui/menu"
import { Select } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { useT } from "@/i18n"
import { copyText, pickFile, saveFile } from "@/lib/api"
import { useDraft } from "@/lib/hooks"
import { run, useStore } from "@/lib/store"
import type { Project } from "@/lib/types"

export function LaunchTab({ project: p }: { project: Project }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const global = snap.settings
  const l = p.launch
  const [args, setArgs] = useDraft(l.args ?? (p.standalone ? "" : global.launch_args), (v) => void run({ type: "set_project_args", repo: p.repo, value: v }))
  const [switches, setSwitches] = useDraft(l.switches ?? global.extra_env, (v) => void run({ type: "set_project_switches", repo: p.repo, value: v }))
  const [branch, setBranch] = useDraft(p.branch_chosen ? (p.branch ?? "") : "", (v) => void run({ type: "set_project_branch", repo: p.repo, value: v.trim() || null }))
  const options = l.script_options
  return (
    <div className="max-w-[760px] space-y-8">
      {p.kind === "git" && (
        <Group title={t("library.launch.branch")} description={t("library.launch.branchHint")}>
          <Row stacked label={p.branch_chosen ? t("library.launch.branchChosen") : t("library.launch.branchFeed")}>
            <Input
              value={branch}
              onChange={(e) => setBranch(e.target.value)}
              // i18n-ignore: a branch name, as it is written
              placeholder={p.branch ?? "main"}
              className="font-mono text-secondary"
              aria-label={t("library.launch.branchLabel")}
            />
            {p.branch_chosen && (
              <Button size="sm" variant="ghost" onClick={() => void run({ type: "set_project_branch", repo: p.repo, value: null })}>{t("library.launch.useDefault")}</Button>
            )}
          </Row>
        </Group>
      )}
      <Group title={t("library.launch.args")} description={p.standalone ? t("library.launch.argsStandalone") : t("library.launch.argsHint")}>
        <Row stacked label={l.args == null && !p.standalone ? t("library.launch.usingDefaultArgs") : t("library.launch.projectArgs")}>
          <Input
            value={args}
            onChange={(e) => setArgs(e.target.value)}
            // i18n-ignore: an example command line
            placeholder={p.standalone ? t("library.launch.programArgs") : "--strict -- -windowed"}
            className="font-mono text-secondary"
            aria-label={t("library.launch.argsLabel")}
          />
          {l.args != null && !p.standalone && (
            <Button size="sm" variant="ghost" onClick={() => void run({ type: "set_project_args", repo: p.repo, value: null })}>{t("library.launch.useDefault")}</Button>
          )}
        </Row>
      </Group>
      {!p.standalone && (
        <Group title={t("library.launch.switches")} description={t("library.launch.switchesHint")}>
          {options && (options.ray_tracing || options.probe_rays || options.hooks) && (
            <>
              {options.ray_tracing && (
                <Row label={t("library.launch.rayTracing")}>
                  <Switch checked={l.script_state.ray_tracing} onCheckedChange={(v) => void run({ type: "set_script_option", repo: p.repo, option: "ray_tracing", enabled: v })} aria-label={t("library.launch.rayTracing")} />
                </Row>
              )}
              {options.probe_rays && (
                <Row label={t("library.launch.probeRays")} hint={t("library.launch.probeRaysHint")}>
                  <Switch disabled={l.script_state.ray_tracing} checked={!l.script_state.ray_tracing && l.script_state.probe_rays} onCheckedChange={(v) => void run({ type: "set_script_option", repo: p.repo, option: "probe_rays", enabled: v })} aria-label={t("library.launch.probeRays")} />
                </Row>
              )}
              {options.hooks && (
                <Row label={t("library.launch.hooks")}>
                  <Switch checked={l.script_state.hooks} onCheckedChange={(v) => void run({ type: "set_script_option", repo: p.repo, option: "hooks", enabled: v })} aria-label={t("library.launch.hooks")} />
                </Row>
              )}
            </>
          )}
          <Row stacked label={l.switches == null ? t("library.launch.usingDefaultSwitches") : t("library.launch.projectSwitches")}>
            <Textarea
              value={switches}
              onChange={(e) => setSwitches(e.target.value)}
              // i18n-ignore: an environment variable, as it is written
              placeholder="VICTORY_NAME=value"
              className="font-mono text-secondary"
              rows={4}
              aria-label={t("library.launch.switchesLabel")}
            />
            {l.switches != null && (
              <Button size="sm" variant="ghost" onClick={() => void run({ type: "set_project_switches", repo: p.repo, value: null })}>{t("library.launch.useDefault")}</Button>
            )}
          </Row>
        </Group>
      )}
      <Presets project={p} />
    </div>
  )
}

function Presets({ project: p }: { project: Project }) {
  const t = useT()
  const snap = useStore((s) => s.snap)!
  const compatible = snap.presets.filter((x) => !p.standalone || x.switches == null)
  const [detected, setDetected] = useState<string | null>(null)
  const [picked, setPicked] = useState<string>("")
  const [saving, setSaving] = useState<{ name: string; argsOnly: boolean } | null>(null)
  const [pasting, setPasting] = useState<string | null>(null)
  useEffect(() => {
    void run<string | null>({ type: "detected_preset", repo: p.repo }).then((d) => setDetected(d ?? null))
  }, [p.repo, snap.presets.length])
  useEffect(() => setPicked(p.launch.selected_preset ?? ""), [p.launch.selected_preset])
  const choices = [
    ...(detected ? [{ value: "__detected", label: t("library.presets.detected", { name: detected }) }] : []),
    ...compatible.map((x) => ({ value: x.name, label: x.name })),
  ]
  const value = picked || (detected ? "__detected" : "")
  const selectedName = value === "__detected" ? null : value || null
  return (
    <Group
      title={t("library.presets.title")}
      description={t("library.presets.description")}
      action={
        <Menu>
          <MenuTrigger asChild>
            <Button size="sm" variant="ghost"><MoreHorizontal /> {t("library.presets.import")}</Button>
          </MenuTrigger>
          <MenuContent align="end">
            <MenuItem
              onSelect={async () => {
                const path = await pickFile(t("library.presets.importTitle"), [{ name: t("library.presets.importFilter"), extensions: ["json", "ps1"] }])
                if (path) void run<string>({ type: "import_preset_file", repo: p.repo, path }).then((n) => n && toast.success(t("library.presets.imported", { name: n })))
              }}
            >
              <FileUp /> {t("library.presets.importFile")}
            </MenuItem>
            <MenuItem onSelect={() => setPasting("")}>
              <ClipboardPaste /> {t("library.presets.paste")}
            </MenuItem>
          </MenuContent>
        </Menu>
      }
    >
      <Row label={t("library.presets.preset")} hint={p.launch.selected_preset ? t("library.presets.applied", { name: p.launch.selected_preset }) : t("library.presets.noneApplied")}>
        {choices.length > 0 ? (
          <Select label={t("library.presets.preset")} value={value || choices[0].value} options={choices} onChange={setPicked} />
        ) : (
          <span className="text-secondary text-fg-3">{t("library.presets.noSaved")}</span>
        )}
        <Button size="sm" variant="secondary" disabled={choices.length === 0} onClick={() => void run<string>({ type: "apply_preset", repo: p.repo, name: selectedName ?? (value ? null : choices[0]?.value ?? null) }).then((n) => n && toast.success(t("library.presets.appliedToast", { name: n })))}>
          {t("library.presets.apply")}
        </Button>
        {selectedName && (
          <Menu>
            <MenuTrigger asChild>
              <Button size="icon-sm" variant="ghost" aria-label={t("library.presets.actions")}><MoreHorizontal /></Button>
            </MenuTrigger>
            <MenuContent align="end">
              <MenuItem
                onSelect={async () => {
                  // i18n-ignore: the file's default name
                  const path = await saveFile(t("library.presets.exportTitle"), "launch-preset.json", [{ name: t("library.presets.exportFilter"), extensions: ["json"] }])
                  if (path) void run({ type: "export_preset", name: selectedName, path }, t("library.presets.exported"))
                }}
              >
                <Download /> {t("library.presets.exportFile")}
              </MenuItem>
              <MenuItem onSelect={() => void run<string>({ type: "preset_text", name: selectedName }).then((text) => text && copyText(text).then(() => toast.success(t("library.presets.textCopied"))))}>
                <Copy /> {t("library.presets.copyText")}
              </MenuItem>
              <MenuSeparator />
              <MenuItem tone="danger" onSelect={() => void run({ type: "delete_preset", repo: p.repo, name: selectedName }, t("library.presets.deleted", { name: selectedName })).then(() => setPicked(""))}>
                <Trash2 /> {t("library.presets.delete")}
              </MenuItem>
            </MenuContent>
          </Menu>
        )}
      </Row>
      {saving ? (
        <Row stacked label={t("library.presets.saveTitle")}>
          <Input autoFocus value={saving.name} onChange={(e) => setSaving({ ...saving, name: e.target.value })} placeholder={t("library.presets.name")} aria-label={t("library.presets.name")} />
          {!p.standalone && (
            <label className="flex shrink-0 items-center gap-2 text-secondary text-fg-2">
              <Switch checked={saving.argsOnly} onCheckedChange={(argsOnly) => setSaving({ ...saving, argsOnly })} aria-label={t("library.presets.argsOnly")} /> {t("library.presets.argsOnly")}
            </label>
          )}
          <Button size="sm" variant="primary" disabled={!saving.name.trim()} onClick={() => void run<string>({ type: "save_preset", repo: p.repo, name: saving.name, include_switches: !saving.argsOnly }).then((n) => { if (n) { toast.success(t("library.presets.saved", { name: n })); setSaving(null); setPicked(n) } })}>
            {t("common.save")}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSaving(null)}>{t("common.cancel")}</Button>
        </Row>
      ) : pasting != null ? (
        <Row stacked label={t("library.presets.pasteTitle")}>
          <Textarea autoFocus value={pasting} onChange={(e) => setPasting(e.target.value)} rows={3} className="font-mono text-secondary" aria-label={t("library.presets.text")} />
          <Button size="sm" variant="primary" disabled={!pasting.trim()} onClick={() => void run<string>({ type: "import_preset_text", repo: p.repo, text: pasting }).then((n) => { if (n) { toast.success(t("library.presets.imported", { name: n })); setPasting(null) } })}>
            {t("library.presets.import")}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setPasting(null)}>{t("common.cancel")}</Button>
        </Row>
      ) : (
        <Row label={t("library.presets.saveAs")} hint={t("library.presets.saveAsHint")}>
          <Button size="sm" variant="secondary" onClick={() => setSaving({ name: "", argsOnly: false })}><Save /> {t("library.presets.saveAsButton")}</Button>
        </Row>
      )}
    </Group>
  )
}
