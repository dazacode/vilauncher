import { useT } from "@/i18n"
import { useStore, type View } from "@/lib/store"
import { cn } from "@/lib/utils"

const ITEMS = [
  { view: "library", key: "shell.nav.builds" },
  { view: "mods", key: "shell.nav.mods" },
  { view: "settings", key: "shell.nav.settings" },
] as const satisfies readonly { view: View; key: string }[]

/** BUILDS · MODS · SETTINGS, top right over every page; the current one is underlined in VIPortLive pink. */
export function TopNav({ onArt }: { onArt: boolean }) {
  const t = useT()
  const view = useStore((s) => s.view)
  const setView = useStore((s) => s.setView)
  return (
    <nav aria-label={t("shell.nav.label")} className="flex items-center gap-[42px]">
      {ITEMS.map((item) => {
        const current = view === item.view
        return (
          <button
            key={item.view}
            type="button"
            onClick={() => setView(item.view)}
            aria-current={current ? "page" : undefined}
            className={cn(
              "no-drag relative -mr-[0.1em] cursor-pointer rounded-[2px] font-display text-[16px] leading-[28px] font-bold tracking-[0.1em] uppercase transition-colors duration-150 [text-shadow:0_1px_10px_rgb(20_10_40/0.45)]",
              // Over the art the words stay white; on the other pages they follow the theme.
              onArt ? (current ? "text-white" : "text-white/70 hover:text-white") : current ? "text-fg" : "text-fg-2 hover:text-fg",
              !onArt && "[text-shadow:none]",
            )}
          >
            {t(item.key)}
            <span
              aria-hidden
              className={cn(
                "absolute right-[0.1em] -bottom-[3px] left-0 h-[2px] rounded-full bg-accent shadow-[0_0_10px_var(--color-accent)] transition-opacity duration-150",
                current ? "opacity-100" : "opacity-0",
              )}
            />
          </button>
        )
      })}
    </nav>
  )
}
