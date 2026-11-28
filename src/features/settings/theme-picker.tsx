import { Check, Plus, Share2, Trash2 } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Row } from "@/components/ui/group"
import { Input } from "@/components/ui/input"
import { Segmented } from "@/components/ui/segmented"
import { useT } from "@/i18n"
import { copyText, platformName } from "@/lib/api"
import { useStore } from "@/lib/store"
import {
  addTheme,
  type Base,
  BUILT_IN,
  type BuiltIn,
  type CustomTheme,
  deleteTheme,
  MAX_CUSTOM,
  palette,
  pawsUnlocked,
  readShareCode,
  setTheme,
  shareCode,
  type ThemeSpec,
  themeLabel,
  updateTheme,
  usableAccent,
  usableGround,
  useCustomThemes,
  useLook,
} from "@/lib/theme"
import { cn } from "@/lib/utils"

const ACCENTS = ["#e0338a", "#7c5cff", "#2f7cf6", "#14a3a3", "#2fa25a", "#e0892f", "#e0473a", "#8a8f98"]
const GROUNDS: Record<Base, string[]> = {
  dark: ["#0c0b10", "#101417", "#121019", "#0f1420", "#14110d", "#000000"],
  light: ["#f5f4f7", "#ffffff", "#f3f1ec", "#eef2f7", "#f1f5f1", "#f7eff3"],
}

/** Settings → Appearance: the looks to pick from, the editor for one of the player's own, and import. */
export function ThemeRows() {
  const t = useT()
  const look = useLook()
  const custom = useCustomThemes()
  const builtIns: BuiltIn[] = ["dark", "light", ...(pawsUnlocked() || look.builtIn === "paws" ? (["paws"] as const) : [])]
  const editing = custom.find((c) => c.id === look.id)
  const full = custom.length >= MAX_CUSTOM
  const create = () => addTheme({ ...look.spec, name: t("settings.theme.defaultName", { n: custom.length + 1 }) })
  return (
    <>
      <Row stacked label={t("settings.appearance.look")} hint={full ? t("settings.theme.full") : t("settings.theme.hint")}>
        <div role="radiogroup" aria-label={t("settings.appearance.look")} className="grid w-full grid-cols-3 gap-2 sm:grid-cols-4">
          {builtIns.map((id) => (
            <Card key={id} spec={BUILT_IN[id]} name={themeLabel(id)} on={look.id === id} onPick={() => setTheme(id)} />
          ))}
          {custom.map((c) => (
            <Card key={c.id} spec={c} name={c.name || t("settings.theme.untitled")} on={look.id === c.id} onPick={() => setTheme(c.id)} />
          ))}
          {!full && (
            <button
              type="button"
              onClick={create}
              title={t("settings.theme.newHint")}
              className="flex aspect-[16/10] cursor-pointer flex-col items-center justify-center gap-1 rounded-md border border-dashed border-line-2 text-caption text-fg-3 transition-colors hover:border-accent/60 hover:text-fg"
            >
              <Plus className="size-4" />
              {t("settings.theme.new")}
            </button>
          )}
        </div>
      </Row>
      {editing && <Editor key={editing.id} theme={editing} />}
      <ImportRow full={full} />
    </>
  )
}

/** A look as a small window: its ground, sidebar, text and accent. */
function Card({ spec, name, on, onPick }: { spec: ThemeSpec; name: string; on: boolean; onPick(): void }) {
  const p = palette(spec)
  return (
    <button
      type="button"
      role="radio"
      aria-checked={on}
      onClick={onPick}
      className={cn(
        "group relative flex cursor-pointer flex-col overflow-hidden rounded-md text-left ring-1 transition-shadow duration-150",
        on ? "ring-2 ring-accent" : "ring-line hover:ring-line-2",
      )}
    >
      <span aria-hidden className="flex aspect-[16/10] w-full" style={{ background: p["--color-ground"] }}>
        <span className="h-full w-[28%]" style={{ background: p["--color-sidebar"], borderRight: `1px solid ${p["--color-line"]}` }} />
        <span className="flex flex-1 flex-col justify-end gap-1 p-2">
          <span className="h-1.5 w-3/4 rounded-full" style={{ background: p["--color-fg"] }} />
          <span className="h-1.5 w-1/2 rounded-full" style={{ background: p["--color-fg-3"] }} />
          <span className="mt-1 h-3 w-10 rounded-[3px]" style={{ background: p["--color-accent"] }} />
        </span>
      </span>
      <span className="flex items-center justify-between gap-1 border-t border-line bg-fill-1 px-2 py-1.5 text-caption text-fg">
        <span className="truncate">{name}</span>
        {on && <Check className="size-3.5 shrink-0 text-accent-fg" strokeWidth={3} />}
      </span>
    </button>
  )
}

/** The player's own theme, changed as they go (it applies at once). */
function Editor({ theme: c }: { theme: CustomTheme }) {
  const t = useT()
  const platform = useStore((s) => s.snap?.brand.platform) ?? platformName()
  const glassHint = platform === "windows" ? t("settings.theme.glassWindows") : platform === "macos" ? t("settings.theme.glassMac") : t("settings.theme.glassOther")
  const set = (patch: Partial<ThemeSpec>) => updateTheme(c.id, patch)
  return (
    <>
      <Row label={t("settings.theme.name")}>
        <Input value={c.name} maxLength={32} onChange={(e) => set({ name: e.target.value })} aria-label={t("settings.theme.name")} className="w-56" />
      </Row>
      <Row label={t("settings.theme.base")} hint={t("settings.theme.baseHint")}>
        <Segmented
          label={t("settings.theme.base")}
          size="sm"
          value={c.base}
          options={[
            { value: "dark", label: themeLabel("dark") },
            { value: "light", label: themeLabel("light") },
          ]}
          // A new base starts from its own background; the accent stays.
          onChange={(base: Base) => base !== c.base && set({ base, ground: BUILT_IN[base].ground })}
        />
      </Row>
      <ColourRow label={t("settings.theme.accent")} hint={t("settings.theme.accentHint")} pick={t("settings.theme.accentPick")} value={usableAccent(c)} raw={c.accent} swatches={ACCENTS} onChange={(accent) => set({ accent })} />
      <ColourRow label={t("settings.theme.background")} hint={t("settings.theme.backgroundHint")} pick={t("settings.theme.backgroundPick")} value={usableGround(c)} raw={c.ground} swatches={GROUNDS[c.base]} onChange={(ground) => set({ ground })} />
      <SliderRow label={t("settings.theme.glass")} hint={glassHint} min={0} max={100} value={c.glass} shown={c.glass === 0 ? t("settings.theme.flat") : `${c.glass}%`} onChange={(glass) => set({ glass })} />
      <SliderRow label={t("settings.theme.roundness")} hint={t("settings.theme.roundnessHint")} min={0} max={150} value={c.roundness} shown={`${c.roundness}%`} onChange={(roundness) => set({ roundness })} />
      <Row label={c.name || t("settings.theme.untitled")}>
        <Button
          size="sm"
          onClick={() =>
            copyText(shareCode(c))
              .then(() => toast.success(t("settings.theme.shared")))
              .catch((e) => toast.error(String(e)))
          }
        >
          <Share2 /> {t("settings.theme.share")}
        </Button>
        <Button size="sm" variant="ghost" className="text-bad" onClick={() => deleteTheme(c.id)}>
          <Trash2 /> {t("settings.theme.delete")}
        </Button>
      </Row>
    </>
  )
}

function ColourRow(props: { label: string; hint: string; pick: string; value: string; raw: string; swatches: string[]; onChange(hex: string): void }) {
  const t = useT()
  return (
    <Row label={props.label} hint={props.hint}>
      <div className="flex items-center gap-1.5">
        {props.swatches.map((hex) => (
          <button
            key={hex}
            type="button"
            aria-label={t("settings.theme.swatch", { colour: hex })}
            title={hex}
            onClick={() => props.onChange(hex)}
            className={cn("size-5 cursor-pointer rounded-full ring-1 ring-line-2 transition-transform hover:scale-110", props.raw.toLowerCase() === hex && "ring-2 ring-accent")}
            style={{ background: hex }}
          />
        ))}
        <input
          type="color"
          aria-label={props.pick}
          title={props.pick}
          value={props.value}
          onChange={(e) => props.onChange(e.target.value)}
          className="ml-1 h-7 w-9 cursor-pointer rounded-md border border-line bg-fill-1 p-0.5"
        />
      </div>
    </Row>
  )
}

function SliderRow(props: { label: string; hint: string; min: number; max: number; value: number; shown: string; onChange(v: number): void }) {
  return (
    <Row label={props.label} hint={props.hint}>
      <div className="flex items-center gap-3">
        <input
          type="range"
          min={props.min}
          max={props.max}
          value={props.value}
          aria-label={props.label}
          onChange={(e) => props.onChange(Number(e.target.value))}
          className="w-40 cursor-pointer accent-accent"
        />
        <span className="w-11 text-right text-secondary text-fg-2 tabular-nums">{props.shown}</span>
      </div>
    </Row>
  )
}

/** A share code someone sent, added as one of the player's own themes. */
function ImportRow({ full }: { full: boolean }) {
  const t = useT()
  const [code, setCode] = useState("")
  const add = () => {
    const spec = readShareCode(code)
    if (!spec) return void toast.error(t("settings.theme.importBad"))
    if (full || !addTheme({ ...spec, name: spec.name || t("settings.theme.untitled") })) return void toast.error(t("settings.theme.importFull"))
    setCode("")
    toast.success(t("settings.theme.imported", { name: spec.name || t("settings.theme.untitled") }))
  }
  return (
    <Row label={t("settings.theme.import")} hint={t("settings.theme.importHint")}>
      <Input
        value={code}
        onChange={(e) => setCode(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && add()}
        placeholder="VITHEME1:…" // i18n-ignore: the code's own prefix
        aria-label={t("settings.theme.import")}
        spellCheck={false}
        className="w-56 font-mono text-caption"
      />
      <Button size="sm" disabled={!code.trim()} onClick={add}>
        {t("settings.theme.importButton")}
      </Button>
    </Row>
  )
}
