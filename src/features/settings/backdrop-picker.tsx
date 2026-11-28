import { Check, ChevronDown, Shuffle } from "lucide-react"
import { useState } from "react"
import { Row } from "@/components/ui/group"
import { Switch } from "@/components/ui/switch"
import { useT } from "@/i18n"
import { SHOTS } from "@/lib/backdrop-shots"
import { type Backdrop, BACKDROPS, setBackdrop, useBackdropChoice } from "@/lib/backdrops"
import { useStore } from "@/lib/store"
import { cn } from "@/lib/utils"

/** The screenshots under a heading each, in the pack's order. */
const SHOT_GROUPS = SHOTS.reduce<{ name: string; shots: Backdrop[] }[]>((groups, shot) => {
  const last = groups.at(-1)
  if (last && last.name === shot.group) last.shots.push(shot)
  else groups.push({ name: shot.group ?? "", shots: [shot] })
  return groups
}, [])

/** Which GTA VI picture sits behind the library, or a new one each launch: the artwork, then the screenshots. */
export function BackdropPicker() {
  const t = useT()
  const choice = useBackdropChoice()
  const own = useStore((s) => (s.snap?.backgrounds.length ?? 0) > 0)
  const shuffle = choice === "shuffle"
  // The screenshots open by themselves when one of them is the background.
  const [showShots, setShowShots] = useState(() => SHOTS.some((s) => s.id === choice))
  return (
    <>
      <Row
        label={t("settings.appearance.background")}
        hint={own ? t("settings.appearance.backgroundOwn") : t("settings.appearance.backgroundHint")}
        stacked
      >
        <div role="radiogroup" aria-label={t("settings.appearance.background")} className="w-full space-y-4">
          <section className="space-y-2">
            <h4 className="text-caption font-semibold tracking-[0.08em] text-fg-3 uppercase">{t("settings.appearance.artwork")}</h4>
            <div className="grid grid-cols-3 gap-2">
              {BACKDROPS.map((b) => (
                <Tile key={b.id} backdrop={b} on={!shuffle && choice === b.id} dim={shuffle} label />
              ))}
            </div>
          </section>
          <section className="space-y-3">
            <button
              type="button"
              aria-expanded={showShots}
              onClick={() => setShowShots((v) => !v)}
              className="flex w-full cursor-pointer items-center justify-between rounded-md text-caption font-semibold tracking-[0.08em] text-fg-3 uppercase hover:text-fg"
            >
              <span>{t("settings.appearance.screenshots")}</span>
              <span className="inline-flex items-center gap-1 font-medium tracking-normal normal-case">
                {showShots ? t("settings.appearance.hideShots") : t("settings.appearance.showShots", { count: SHOTS.length })}
                <ChevronDown className={cn("size-4 transition-transform duration-150", showShots && "rotate-180")} />
              </span>
            </button>
            {showShots &&
              SHOT_GROUPS.map((g) => (
                <div key={g.name} className="space-y-1.5">
                  <p className="text-caption text-fg-2">{g.name}</p>
                  <div className="grid grid-cols-4 gap-1.5">
                    {g.shots.map((b) => (
                      <Tile key={b.id} backdrop={b} on={!shuffle && choice === b.id} dim={shuffle} />
                    ))}
                  </div>
                </div>
              ))}
          </section>
        </div>
      </Row>
      <Row label={<span className="inline-flex items-center gap-2"><Shuffle className="size-4 text-fg-3" /> {t("settings.appearance.shuffle")}</span>} hint={t("settings.appearance.shuffleHint")}>
        <Switch checked={shuffle} onCheckedChange={(v) => setBackdrop(v ? "shuffle" : "robbery")} aria-label={t("settings.appearance.shuffleLabel")} />
      </Row>
    </>
  )
}

/** One picture to pick; the artwork shows its name on it, a screenshot only in its tooltip. */
function Tile({ backdrop: b, on, dim, label }: { backdrop: Backdrop; on: boolean; dim: boolean; label?: boolean }) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={on}
      aria-label={b.name}
      title={b.name}
      onClick={() => setBackdrop(b.id)}
      className={cn(
        "group relative aspect-video cursor-pointer overflow-hidden rounded-md ring-1 ring-line transition-[box-shadow,opacity] duration-150",
        on ? "ring-2 ring-accent" : "hover:ring-line-2",
        dim && "opacity-60 hover:opacity-100",
      )}
    >
      <img src={b.thumb} alt="" loading="lazy" draggable={false} className="size-full object-cover transition-transform duration-300 group-hover:scale-[1.04]" />
      {label && (
        <span className="absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/75 to-transparent px-2 pt-4 pb-1.5 text-left text-caption text-white">{b.name}</span>
      )}
      {on && (
        <span className="absolute top-1.5 right-1.5 grid size-5 place-items-center rounded-full bg-accent text-white">
          <Check className="size-3.5" strokeWidth={3} />
        </span>
      )}
    </button>
  )
}
