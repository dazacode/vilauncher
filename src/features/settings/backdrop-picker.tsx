import { Check, Shuffle } from "lucide-react"
import { Row } from "@/components/ui/group"
import { Switch } from "@/components/ui/switch"
import { BACKDROPS, setBackdrop, useBackdropChoice } from "@/lib/backdrops"
import { useStore } from "@/lib/store"
import { cn } from "@/lib/utils"

/** Which GTA VI picture sits behind the library, or a new one each launch. */
export function BackdropPicker() {
  const choice = useBackdropChoice()
  const own = useStore((s) => (s.snap?.backgrounds.length ?? 0) > 0)
  const shuffle = choice === "shuffle"
  return (
    <>
      <Row
        label="Library background"
        hint={own ? "Your own background pictures are showing instead. Remove them to use these." : "Official GTA VI artwork from Rockstar Games."}
        stacked
      >
        <div role="radiogroup" aria-label="Library background" className="grid w-full grid-cols-3 gap-2">
          {BACKDROPS.map((b) => {
            const on = !shuffle && choice === b.id
            return (
              <button
                key={b.id}
                type="button"
                role="radio"
                aria-checked={on}
                aria-label={b.name}
                title={b.name}
                onClick={() => setBackdrop(b.id)}
                className={cn(
                  "group relative aspect-video cursor-pointer overflow-hidden rounded-md ring-1 ring-line transition-[box-shadow,opacity] duration-150",
                  on ? "ring-2 ring-accent" : "hover:ring-line-2",
                  shuffle && "opacity-60 hover:opacity-100",
                )}
              >
                <img src={b.thumb} alt="" loading="lazy" draggable={false} className="size-full object-cover transition-transform duration-300 group-hover:scale-[1.04]" />
                <span className="absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/75 to-transparent px-2 pt-4 pb-1.5 text-left text-caption text-white">{b.name}</span>
                {on && (
                  <span className="absolute top-1.5 right-1.5 grid size-5 place-items-center rounded-full bg-accent text-white">
                    <Check className="size-3.5" strokeWidth={3} />
                  </span>
                )}
              </button>
            )
          })}
        </div>
      </Row>
      <Row label={<span className="inline-flex items-center gap-2"><Shuffle className="size-4 text-fg-3" /> Shuffle on each launch</span>} hint="Picks one of these at random every time VILauncher starts.">
        <Switch checked={shuffle} onCheckedChange={(v) => setBackdrop(v ? "shuffle" : "robbery")} aria-label="Shuffle the library background on each launch" />
      </Row>
    </>
  )
}
