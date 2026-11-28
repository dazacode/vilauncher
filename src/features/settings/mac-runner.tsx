import { FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Row, Group } from "@/components/ui/group"
import { Segmented } from "@/components/ui/segmented"
import { Select } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { useT } from "@/i18n"
import { pickFile, pickFolder } from "@/lib/api"
import { run } from "@/lib/store"
import type { MacRunner, MacRunnerChoice, RunnerKind } from "@/lib/types"

/** Saves the runner settings with one change. */
function save(runner: MacRunner, patch: Partial<MacRunnerChoice>) {
  const { kind, bottle, wine, prefix, metalfx, streamline_dir } = { ...runner, ...patch }
  void run({ type: "set_mac_runner", runner: { kind, bottle, wine, prefix, metalfx, streamline_dir } })
}

/** The path as the hint shows it, or "Not set". */
function shown(path: string, unset: string) {
  return path.trim() ? path : unset
}

/** Which runner starts the Windows builds on a Mac (runner.rs): CrossOver's bottle or a Wine prefix. */
export function MacRunnerGroup({ runner: r }: { runner: MacRunner }) {
  const t = useT()
  const kinds: { value: RunnerKind; label: string }[] = [
    { value: "none", label: t("settings.macRunner.none") },
    { value: "crossover", label: t("settings.macRunner.crossover") },
    { value: "wine", label: t("settings.macRunner.wine") },
  ]
  const status =
    r.kind === "none" ? t("settings.macRunner.noneHint") : r.problem ?? t("settings.macRunner.ready")
  return (
    <Group title={t("settings.macRunner.title")} description={t("settings.macRunner.description")}>
      <Row stacked label={t("settings.macRunner.runner")} hint={status}>
        <Segmented
          label={t("settings.macRunner.runner")}
          size="sm"
          value={r.kind}
          options={kinds}
          onChange={(kind) => save(r, { kind, wine: kind === "wine" && !r.wine && r.found_wine ? r.found_wine : r.wine })}
        />
      </Row>
      {r.kind === "crossover" && (
        <Row
          label={t("settings.macRunner.bottle")}
          hint={!r.crossover ? t("settings.macRunner.crossoverMissing") : r.bottles.length ? t("settings.macRunner.bottleHint") : t("settings.macRunner.noBottles")}
        >
          {r.bottles.length > 0 && (
            <Select
              label={t("settings.macRunner.bottle")}
              value={r.bottle}
              placeholder={t("settings.macRunner.pickBottle")}
              options={r.bottles.map((name) => ({ value: name, label: name }))}
              onChange={(bottle) => save(r, { bottle })}
            />
          )}
        </Row>
      )}
      {r.kind === "wine" && (
        <>
          <Row label={t("settings.macRunner.wineProgram")} hint={r.wine ? r.wine : t("settings.macRunner.wineProgramHint")}>
            {!r.wine && r.found_wine && (
              <Button size="sm" variant="ghost" onClick={() => save(r, { wine: r.found_wine ?? "" })}>
                {t("settings.macRunner.useFound")}
              </Button>
            )}
            <Button
              size="sm"
              onClick={async () => {
                const wine = await pickFile(t("settings.macRunner.pickWine"), [])
                if (wine) save(r, { wine })
              }}
            >
              {t("settings.fsr.choose")}
            </Button>
          </Row>
          <Row label={t("settings.macRunner.prefix")} hint={shown(r.prefix, t("settings.macRunner.prefixHint"))}>
            <Button
              size="sm"
              onClick={async () => {
                const prefix = await pickFolder(t("settings.macRunner.pickPrefix"))
                if (prefix) save(r, { prefix })
              }}
            >
              <FolderOpen /> {t("settings.fsr.choose")}
            </Button>
          </Row>
        </>
      )}
    </Group>
  )
}

/** DLSS through MetalFX on a Mac: the switch, and the player's own Streamline folder it needs. */
export function MetalfxRows({ runner: r }: { runner: MacRunner }) {
  const t = useT()
  const usable = r.kind !== "none" && !r.problem
  return (
    <>
      <Row label={t("settings.metalfx.title")} hint={usable ? t("settings.metalfx.hint") : t("settings.metalfx.needsRunner")}>
        <Switch checked={r.metalfx} disabled={!usable} onCheckedChange={(metalfx) => save(r, { metalfx })} aria-label={t("settings.metalfx.title")} />
      </Row>
      {r.metalfx && (
        <Row
          label={t("settings.metalfx.streamline")}
          hint={
            !r.streamline_dir
              ? t("settings.metalfx.streamlineHint")
              : r.streamline_ok
                ? t("settings.metalfx.streamlineOk", { path: r.streamline_dir })
                : t("settings.metalfx.streamlineMissing", { path: r.streamline_dir })
          }
        >
          <Button
            size="sm"
            onClick={async () => {
              const streamline_dir = await pickFolder(t("settings.metalfx.pickStreamline"))
              if (streamline_dir) save(r, { streamline_dir })
            }}
          >
            <FolderOpen /> {t("settings.fsr.choose")}
          </Button>
        </Row>
      )}
    </>
  )
}
