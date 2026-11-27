import { Wrench } from "lucide-react"

const ITEMS = [
  { name: "Vice City shaders", tag: "Shaders", art: "/gta6/rail-2.jpg" },
  { name: "Leonida handling pack", tag: "Handling", art: "/gta6/rail-gta.jpg" },
  { name: "RAGE9 debug overlay", tag: "Tools", art: "/gta6/gta-icon.jpg" },
  { name: "Sunset timecycle", tag: "Visuals", art: "/gta6/gta-tile.jpg" },
]

/** Workshop: where community patches and packs will live. Not wired to a source yet, so it says so. */
export function WorkshopView() {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-[760px] px-10 pt-6 pb-16">
        <header>
          <p className="text-caption font-medium tracking-[0.08em] text-warn uppercase">Coming soon</p>
          <h1 className="mt-1 font-display text-page font-bold tracking-[-0.02em] text-fg">Workshop</h1>
          <p className="mt-1 max-w-[560px] text-body text-fg-3">
            Community patches, tools and visual packs for your builds will show here, ready to install in one click. These are examples of what is coming.
          </p>
        </header>
        <ul role="list" className="mt-8 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
          {ITEMS.map((item) => (
            <li key={item.name} className="overflow-hidden rounded-lg border border-line bg-fill-1">
              <div className="relative h-[112px] overflow-hidden">
                <img src={item.art} alt="" className="size-full object-cover opacity-70 grayscale-[35%]" draggable={false} />
                <span className="absolute top-2 right-2 rounded-xs bg-black/65 px-2 py-0.5 text-caption font-medium text-fg-2">{item.tag}</span>
              </div>
              <div className="flex items-center gap-2 p-3">
                <Wrench className="size-4 text-fg-4" />
                <h2 className="min-w-0 flex-1 truncate text-secondary font-medium text-fg-2">{item.name}</h2>
                <span className="text-caption text-fg-4">Example</span>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
