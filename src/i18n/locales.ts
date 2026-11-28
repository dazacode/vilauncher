// The languages the window speaks. Keep in step with `Locale` in
// src-tauri/src/i18n.rs, which matches system languages the same way.

export const LOCALES = ["en", "zh-TW", "zh-CN"] as const

export type Locale = (typeof LOCALES)[number]

/** Each language's name in itself, as a language menu shows it. */
export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  "zh-TW": "繁體中文",
  "zh-CN": "简体中文",
}

export function isLocale(tag: unknown): tag is Locale {
  return typeof tag === "string" && (LOCALES as readonly string[]).includes(tag)
}

/**
 * The closest language for a BCP 47 tag ("zh-Hant-HK", "en-GB"): traditional Chinese for
 * Taiwan, Hong Kong, Macau and Hant, simplified for the rest of Chinese. Null when none fits.
 */
export function matchLocale(tag: string): Locale | null {
  const parts = tag.trim().toLowerCase().replace(/_/g, "-").split("-")
  if (parts[0] === "en") return "en"
  if (parts[0] === "zh") return parts.slice(1).some((p) => ["hant", "tw", "hk", "mo"].includes(p)) ? "zh-TW" : "zh-CN"
  return null
}

/** The first of the system's languages the window speaks, else English. */
export function systemLocale(languages: readonly string[] = typeof navigator === "undefined" ? [] : navigator.languages): Locale {
  for (const tag of languages) {
    const locale = matchLocale(tag)
    if (locale) return locale
  }
  return "en"
}
