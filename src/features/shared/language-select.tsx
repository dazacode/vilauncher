import { Select } from "@/components/ui/select"
import { LOCALE_NAMES, LOCALES, systemLocale, useT } from "@/i18n"
import { run, useStore } from "@/lib/store"

/**
 * The window's language: the system's (named in its own language), or one the player picks.
 * The launcher keeps the choice (`set_language`) and sends the result as the snapshot's locale.
 * In Settings → Appearance, and on the legal notice so it can be read before agreeing.
 */
export function LanguageSelect({ className }: { className?: string }) {
  const t = useT()
  const choice = useStore((s) => s.snap?.settings.language) ?? "system"
  const options = [
    { value: "system", label: t("settings.language.system", { name: LOCALE_NAMES[systemLocale()] }) },
    ...LOCALES.map((locale) => ({ value: locale as string, label: LOCALE_NAMES[locale] })),
  ]
  return (
    <Select
      label={t("settings.language.label")}
      value={choice}
      options={options}
      className={className}
      onChange={(value) => void run({ type: "set_language", language: value === "system" ? null : value })}
    />
  )
}
