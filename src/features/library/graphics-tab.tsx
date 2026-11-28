import { Group, Row } from "@/components/ui/group"
import { Segmented } from "@/components/ui/segmented"
import { Select } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { useT } from "@/i18n"
import { graphicsMode, MODES, modeDetail, modeLabel, OUTPUTS, outputLabel, withMode } from "@/lib/format"
import { run } from "@/lib/store"
import type { Graphics, Project } from "@/lib/types"

/** The console profiles: product names, the same in every language. */
const CONSOLES = [
  { value: "x", label: "Series X" }, // i18n-ignore: product name
  { value: "s", label: "Series S" }, // i18n-ignore: product name
] as const

export function GraphicsTab({ project: p }: { project: Project }) {
  const t = useT()
  if (p.standalone) {
    return <p className="text-body text-fg-3">{t("library.graphics.standalone")}</p>
  }
  const g = p.launch.graphics
  const set = (graphics: Graphics) => void run({ type: "set_project_graphics", repo: p.repo, graphics })
  return (
    <div className="max-w-[760px] space-y-8">
      <Group>
        <Row label={t("library.graphics.useDefaults")} hint={p.launch.graphics_override ? t("library.graphics.ownSettings") : t("library.graphics.defaultsHint")}>
          <Switch
            checked={!p.launch.graphics_override}
            onCheckedChange={(on) => void run({ type: "set_project_graphics", repo: p.repo, graphics: on ? null : g })}
            aria-label={t("library.graphics.useDefaults")}
          />
        </Row>
      </Group>
      <GraphicsGroups graphics={g} onChange={set} />
    </div>
  )
}

/** The graphics controls, shared by a project and the defaults in Settings. */
export function GraphicsGroups({ graphics: g, onChange }: { graphics: Graphics; onChange(g: Graphics): void }) {
  const t = useT()
  const mode = graphicsMode(g)
  return (
    <>
      <Group title={t("library.graphics.quality")} description={t("library.graphics.qualityHint")}>
        <Row label={t("library.graphics.preset")} hint={mode ? modeDetail(mode) : t("library.graphics.renderScale", { percent: Math.round(g.render_scale * 100) })}>
          <Segmented
            label={t("library.graphics.presetLabel")}
            value={mode}
            options={MODES.map((m) => ({ value: m.id, label: modeLabel(m.id) }))}
            onChange={(id) => onChange(withMode(g, id))}
          />
        </Row>
        <Row label={t("library.graphics.console")} hint={t("library.graphics.consoleHint")}>
          <Select label={t("library.graphics.console")} value={g.series_s ? "s" : "x"} options={CONSOLES} onChange={(v) => onChange({ ...g, series_s: v === "s" })} />
        </Row>
        <Row label={t("library.graphics.output")} hint={t("library.graphics.outputHint")}>
          <Select
            label={t("library.graphics.output")}
            value={g.output}
            options={OUTPUTS.map((id) => ({ value: id, label: outputLabel(id) }))}
            onChange={(output) => onChange({ ...g, output })}
          />
        </Row>
      </Group>
      <Group title={t("library.graphics.fixes")}>
        <Row label={t("library.graphics.fix8gb")} hint={t("library.graphics.fix8gbHint")}>
          <Switch checked={g.fix_8gb} onCheckedChange={(fix_8gb) => onChange({ ...g, fix_8gb })} aria-label={t("library.graphics.fix8gb")} />
        </Row>
        <Row label={t("library.graphics.brightShadows")} hint={t("library.graphics.brightShadowsHint")}>
          <Switch checked={g.bright_shadows} onCheckedChange={(bright_shadows) => onChange({ ...g, bright_shadows })} aria-label={t("library.graphics.brightShadows")} />
        </Row>
        <Row label={t("library.graphics.fullscreen")} hint={t("library.graphics.fullscreenHint")}>
          <Switch checked={g.fullscreen} onCheckedChange={(fullscreen) => onChange({ ...g, fullscreen })} aria-label={t("library.graphics.fullscreen")} />
        </Row>
      </Group>
    </>
  )
}
