import { useT } from "@/i18n"

/** Mods: where community patches and packs will live. Not wired to a source yet, so it says so. */
export function ModsView() {
  const t = useT()
  return (
    <div className="grid h-full place-items-center px-10 pb-[80px]">
      <div className="text-center">
        <p className="text-caption font-semibold tracking-[0.16em] text-accent-fg uppercase">{t("library.mods.soon")}</p>
        <h1 className="mt-2 font-display text-page font-bold tracking-[-0.02em] text-fg">{t("library.mods.title")}</h1>
        <p className="mx-auto mt-2 max-w-[420px] text-body text-fg-3">{t("library.mods.body")}</p>
      </div>
    </div>
  )
}
