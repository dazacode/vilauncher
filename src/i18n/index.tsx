// The window's words in the player's language. See docs/I18N.md.
//
// - `useT()` in components: `const t = useT()`, then `t("settings.language.label")`.
//   Keys and their `{placeholders}` are checked by TypeScript.
// - `t()` outside components (toasts, stores); `useT()` is the same function, but
//   it also re-renders the component when the language changes.
// - `<Trans k="…" values={{…}} tags={{ code: (s) => <code>{s}</code> }} />` for a
//   message with markup (`<code>…</code>`) or React elements in its placeholders.
// - `formatNumber`, `formatBytes`, `formatList` write numbers and lists the language's way.
//
// The language is the launcher's (the snapshot's `locale`): the player's choice in
// Settings, else the system's. Until the first snapshot arrives, the last language
// used on this PC, else the system's.

import { Fragment, type ReactNode, useSyncExternalStore } from "react"
import { isLocale, type Locale, systemLocale } from "./locales"
import { en, type Messages } from "./messages/en"
import { zhCN } from "./messages/zh-CN"
import { zhTW } from "./messages/zh-TW"
import type { MessageAt, MessageKey, MessageTree, ParamNames, ParamValue, PluralForms } from "./types"

export { LOCALE_NAMES, LOCALES, type Locale, matchLocale, systemLocale } from "./locales"

const CATALOGS: Record<Locale, MessageTree> = { en, "zh-TW": zhTW, "zh-CN": zhCN }
const STORAGE_KEY = "vilauncher.locale"

export type Key = MessageKey<Messages>

/** The parameters a message needs: none, or an object with each placeholder (and `count`). */
export type ParamsFor<K extends Key> = [ParamNames<MessageAt<Messages, K>>] extends [never]
  ? [params?: Record<string, never>]
  : [params: Record<ParamNames<MessageAt<Messages, K>>, ParamValue>]

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (isLocale(saved)) return saved
  } catch {
    // No storage (tests, private mode): the system's language.
  }
  return systemLocale()
}

let current: Locale = initialLocale()
const listeners = new Set<() => void>()

function apply() {
  if (typeof document !== "undefined") document.documentElement.lang = current
}
apply()

/** The language the window is in now. */
export function getLocale(): Locale {
  return current
}

/** Shows the window in `locale` (the launcher's, from the snapshot). Unknown tags are ignored. */
export function setLocale(locale: string | null | undefined) {
  if (!isLocale(locale) || locale === current) return
  current = locale
  try {
    localStorage.setItem(STORAGE_KEY, locale)
  } catch {
    // Only the first paint of the next start uses it.
  }
  apply()
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** The current language, updated when it changes. */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getLocale, getLocale)
}

function lookup(tree: MessageTree, key: string): string | PluralForms | undefined {
  let node: MessageTree | string | PluralForms | undefined = tree
  for (const part of key.split(".")) {
    if (!node || typeof node === "string") return undefined
    node = (node as MessageTree)[part]
  }
  if (typeof node === "string") return node
  if (node && typeof node === "object" && typeof (node as PluralForms).other === "string") return node as PluralForms
  return undefined
}

const pluralRules = new Map<Locale, Intl.PluralRules>()
const numberFormats = new Map<string, Intl.NumberFormat>()

function numberFormat(locale: Locale, options?: Intl.NumberFormatOptions) {
  const id = `${locale}|${JSON.stringify(options ?? {})}`
  let format = numberFormats.get(id)
  if (!format) {
    format = new Intl.NumberFormat(locale, options)
    numberFormats.set(id, format)
  }
  return format
}

/** The message's text for this language: the plural form for `count` when it has forms. */
function template(locale: Locale, key: string, params?: Record<string, unknown>): string {
  const found = lookup(CATALOGS[locale], key) ?? lookup(en, key)
  if (found === undefined) {
    if (import.meta.env.DEV) console.warn(`[i18n] no message "${key}"`)
    return key
  }
  if (typeof found === "string") return found
  const count = Number(params?.count ?? 0)
  let rules = pluralRules.get(locale)
  if (!rules) {
    rules = new Intl.PluralRules(locale)
    pluralRules.set(locale, rules)
  }
  if (count === 0 && found.zero !== undefined) return found.zero
  return found[rules.select(count) as keyof PluralForms] ?? found.other
}

/** Splits a message into text and `{name}` placeholders (`{{` and `}}` are braces). */
function pieces(text: string): (string | { name: string })[] {
  const out: (string | { name: string })[] = []
  let literal = ""
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if ((c === "{" || c === "}") && text[i + 1] === c) {
      literal += c
      i++
    } else if (c === "{") {
      const end = text.indexOf("}", i)
      if (end < 0) {
        literal += text.slice(i)
        break
      }
      if (literal) out.push(literal)
      literal = ""
      out.push({ name: text.slice(i + 1, end).trim() })
      i = end
    } else {
      literal += c
    }
  }
  if (literal) out.push(literal)
  return out
}

function written(value: unknown, locale: Locale): string {
  return typeof value === "number" ? numberFormat(locale).format(value) : String(value)
}

/**
 * The message `key` in the current language, with its placeholders filled in. Numbers are
 * written the language's way (pass a string for an id or a year). Falls back to English.
 */
export function t<K extends Key>(key: K, ...[params]: ParamsFor<K>): string {
  const locale = current
  const values = params as Record<string, unknown> | undefined
  return pieces(template(locale, key, values))
    .map((piece) => (typeof piece === "string" ? piece : piece.name in (values ?? {}) ? written(values![piece.name], locale) : `{${piece.name}}`))
    .join("")
}

/** `t` for components: the same function, and the component re-renders when the language changes. */
export function useT(): typeof t {
  useLocale()
  return t
}

type TagRenderer = (children: ReactNode) => ReactNode

/**
 * A message with markup: `<name>…</name>` tags are drawn by `tags[name]`, and placeholders
 * can hold React elements. Unknown tags are shown as their text.
 */
export function Trans<K extends Key>({
  k,
  values,
  tags,
}: {
  k: K
  values?: Partial<Record<ParamNames<MessageAt<Messages, K>>, ReactNode>>
  tags?: Record<string, TagRenderer>
}) {
  const locale = useLocale()
  const params = values as Record<string, unknown> | undefined
  const text = template(locale, k, params)
  // Tokens: tags open and close; the text between them is split into placeholders.
  const tokens = text.split(/(<\/?[a-zA-Z][\w-]*>)/)
  const stack: { tag: string | null; children: ReactNode[] }[] = [{ tag: null, children: [] }]
  let n = 0
  for (const token of tokens) {
    if (!token) continue
    const open = /^<([a-zA-Z][\w-]*)>$/.exec(token)
    const close = /^<\/([a-zA-Z][\w-]*)>$/.exec(token)
    if (open && tags?.[open[1]]) {
      stack.push({ tag: open[1], children: [] })
    } else if (close && stack.length > 1 && stack[stack.length - 1].tag === close[1]) {
      const done = stack.pop()!
      stack[stack.length - 1].children.push(<Fragment key={n++}>{tags![done.tag!](<>{done.children}</>)}</Fragment>)
    } else {
      for (const piece of pieces(token)) {
        const node = typeof piece === "string" ? piece : piece.name in (params ?? {}) ? (typeof params![piece.name] === "number" ? written(params![piece.name], locale) : (params![piece.name] as ReactNode)) : `{${piece.name}}`
        stack[stack.length - 1].children.push(<Fragment key={n++}>{node}</Fragment>)
      }
    }
  }
  while (stack.length > 1) {
    const open = stack.pop()!
    stack[stack.length - 1].children.push(...open.children)
  }
  return <>{stack[0].children}</>
}

/** A number written the current language's way. */
export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return numberFormat(current, options).format(value)
}

/** A size in B, KB, MB, GB or TB (1024-based) with one decimal, written the language's way. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${formatNumber(bytes)} B`
  const units = ["KB", "MB", "GB", "TB"]
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${formatNumber(value, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} ${units[unit]}`
}

/**
 * Items joined the language's way: "a, b, and c" or "a、b 和 c". In Chinese, a space goes
 * between Chinese and Latin letters or digits (docs/I18N.md), which Intl.ListFormat leaves out.
 */
export function formatList(items: readonly string[], type: "conjunction" | "disjunction" | "unit" = "conjunction"): string {
  const list = new Intl.ListFormat(current, { style: "long", type }).format(items)
  return current.startsWith("zh") ? spaceBetweenScripts(list) : list
}

const HAN = "\\p{Script=Han}"
const LATIN = "[A-Za-z0-9+#]"
const HAN_THEN_LATIN = new RegExp(`(${HAN})(${LATIN})`, "gu")
const LATIN_THEN_HAN = new RegExp(`(${LATIN})(${HAN})`, "gu")

/** "NVIDIA和AMD" → "NVIDIA 和 AMD". */
export function spaceBetweenScripts(text: string): string {
  return text.replace(HAN_THEN_LATIN, "$1 $2").replace(LATIN_THEN_HAN, "$1 $2")
}
