import { Wrench } from "lucide-react"
import { useT } from "@/i18n"

const ITEMS = [
  { id: "shaders", tag: "shaders", art: "/gta6/rail-2.jpg" },
  { id: "handling", tag: "handling", art: "/gta6/rail-gta.jpg" },
  { id: "overlay", tag: "tools", art: "/gta6/gta-icon.jpg" },
  { id: "timecycle", tag: "visuals", art: "/gta6/gta-tile.jpg" },
] as const

/** Workshop: where community patches and packs will live. Not wired to a source yet, so it says so. */
export function WorkshopView() {
  const t = useT()
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-[760px] px-10 pt-6 pb-16">
        <header>
          <p className="text-caption font-medium tracking-[0.08em] text-warn uppercase">{t("library.workshop.soon")}</p>
          <h1 className="mt-1 font-display text-page font-bold tracking-[-0.02em] text-fg">{t("library.workshop.title")}</h1>
          <p className="mt-1 max-w-[560px] text-body text-fg-3">
            {t("library.workshop.body")}
          </p>
        </header>
        <ul role="list" className="mt-8 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
          {ITEMS.map((item) => (
            <li key={item.id} className="overflow-hidden rounded-lg border border-line bg-fill-1">
              <div className="relative h-[112px] overflow-hidden">
                <img src={item.art} alt="" className="size-full object-cover opacity-70 grayscale-[35%]" draggable={false} />
                <span className="absolute top-2 right-2 rounded-xs bg-black/65 px-2 py-0.5 text-caption font-medium text-fg-2">{t(`library.workshop.tags.${item.tag}`)}</span>
              </div>
              <div className="flex items-center gap-2 p-3">
                <Wrench className="size-4 text-fg-4" />
                <h2 className="min-w-0 flex-1 truncate text-secondary font-medium text-fg-2">{t(`library.workshop.items.${item.id}`)}</h2>
                <span className="text-caption text-fg-4">{t("library.workshop.example")}</span>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
