#!/usr/bin/env node
// Checks the window's translations, and looks for words still written straight
// into the UI code. Run with `pnpm i18n:check` (Node needs --experimental-strip-types
// on 22, where it reads the .ts message files; it is the default from Node 23.6).
//
// 1. Messages (src/i18n/messages/<locale>/<namespace>.ts): every language has the
//    English keys and no others, the same {placeholders} and <tags> in each
//    message, plural forms where English has them, and no empty message.
//    A translation identical to a long English message is a warning: probably
//    not translated yet. The English legal notice must match LEGAL.md word for word.
// 2. Code (src/**/*.ts, *.tsx; not src/i18n, src/dev, or *.test.ts tests): English text shown to the
//    player outside the catalogs: JSX text, UI attributes (aria-label, label,
//    hint, placeholder, title…), rendered strings in JSX, toasts and dialogs'
//    titles, and English sentences anywhere. Write `// i18n-ignore` on the line
//    (or the line above) for text that must stay as it is.
//
// Exit code: 0 clean (warnings allowed), 1 problems found, 2 usage error.

import { readdir, readFile } from "node:fs/promises"
import { dirname, join, relative, sep } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..")
const MESSAGES = join(ROOT, "src", "i18n", "messages")
const SOURCE = "en"

const errors = []
const warnings = []

// --- 1. Messages ---------------------------------------------------------------

const isPlural = (v) => v && typeof v === "object" && typeof v.other === "string"
const placeholders = (text) => new Set([...text.replace(/\{\{|\}\}/g, "").matchAll(/\{([^{}]+)\}/g)].map((m) => m[1].trim()))
const tags = (text) => new Set([...text.matchAll(/<\/?([a-zA-Z][\w-]*)>/g)].map((m) => m[1]))
const same = (a, b) => a.size === b.size && [...a].every((x) => b.has(x))
const forms = (v) => (typeof v === "string" ? [v] : Object.values(v))
const union = (sets) => new Set(sets.flatMap((s) => [...s]))

function compare(english, translated, where, source = false) {
  for (const key of Object.keys(english)) {
    const path = `${where}.${key}`
    const e = english[key]
    const t = translated?.[key]
    if (t === undefined) {
      errors.push(`${path}: missing`)
      continue
    }
    if (typeof e === "string" || isPlural(e)) {
      if (!(typeof t === "string" || isPlural(t))) {
        errors.push(`${path}: should be a message, not a group`)
        continue
      }
      if (isPlural(e) && (e.one === undefined || e.other === undefined) && source) {
        errors.push(`${path}: English plural forms need "one" and "other"`)
      }
      const eText = forms(e)
      const tText = forms(t)
      if (tText.some((x) => typeof x !== "string" || !x.trim())) errors.push(`${path}: empty message`)
      const ePh = union(eText.map(placeholders))
      const tPh = union(tText.map(placeholders))
      if (isPlural(e)) ePh.add("count")
      if (isPlural(t)) tPh.add("count")
      if (!same(ePh, tPh)) errors.push(`${path}: placeholders {${[...ePh].join("}, {")}} in English, {${[...tPh].join("}, {")}} here`)
      if (!same(union(eText.map(tags)), union(tText.map(tags)))) errors.push(`${path}: different <tags> than English`)
      const eJoined = eText.join(" ")
      if (!source && tText.join(" ") === eJoined && (eJoined.match(/[A-Za-z]{3,}/g) ?? []).length >= 3) {
        warnings.push(`${path}: same as English (${JSON.stringify(eJoined.slice(0, 60))})`)
      }
    } else {
      if (typeof t !== "object" || isPlural(t)) {
        errors.push(`${path}: should be a group of messages`)
        continue
      }
      compare(e, t, path, source)
    }
  }
  for (const key of Object.keys(translated ?? {})) {
    if (!(key in english)) errors.push(`${where}.${key}: not in English (remove it, or add it to English first)`)
  }
}

async function load(locale, namespace) {
  const file = join(MESSAGES, locale, `${namespace}.ts`)
  try {
    return (await import(pathToFileURL(file).href)).default
  } catch (error) {
    errors.push(`${relative(ROOT, file)}: could not be read (${error.message.split("\n")[0]})`)
    return undefined
  }
}

const locales = (await readdir(MESSAGES, { withFileTypes: true })).filter((d) => d.isDirectory()).map((d) => d.name)
const namespaces = (await readdir(join(MESSAGES, SOURCE))).filter((f) => f.endsWith(".ts") && f !== "index.ts").map((f) => f.replace(/\.ts$/, ""))
let messageCount = 0
for (const namespace of namespaces) {
  const english = await load(SOURCE, namespace)
  if (!english) continue
  const count = (tree) => Object.values(tree).reduce((n, v) => n + (typeof v === "string" || isPlural(v) ? 1 : count(v)), 0)
  messageCount += count(english)
  compare(english, english, `${SOURCE}/${namespace}`, true)
  for (const locale of locales.filter((l) => l !== SOURCE)) {
    const translated = await load(locale, namespace)
    if (translated) compare(english, translated, `${locale}/${namespace}`)
  }
}
for (const locale of locales.filter((l) => l !== SOURCE)) {
  const files = (await readdir(join(MESSAGES, locale))).filter((f) => f.endsWith(".ts") && f !== "index.ts").map((f) => f.replace(/\.ts$/, ""))
  for (const extra of files.filter((f) => !namespaces.includes(f))) errors.push(`${locale}/${extra}.ts: no English namespace "${extra}"`)
}

// The English legal notice is LEGAL.md's, word for word: the window must not drift from it.
{
  const legal = (await readFile(join(ROOT, "LEGAL.md"), "utf8")).replace(/\s+/g, " ")
  const notice = (await load(SOURCE, "legal"))?.notice
  const texts = notice ? [notice.intro, ...Object.values(notice.points ?? {}), notice.closing] : []
  if (notice && texts.some((t) => typeof t !== "string")) errors.push("en/legal.notice: intro, points and closing must be messages")
  for (const text of texts.filter((t) => typeof t === "string")) {
    const plain = text.replace(/<\/?[a-zA-Z][\w-]*>/g, "").replace(/\s+/g, " ").trim()
    if (!legal.includes(plain)) errors.push(`en/legal.notice: not word for word in LEGAL.md: ${JSON.stringify(plain.slice(0, 70))}`)
  }
}

// --- 2. Code -----------------------------------------------------------------

const { parseAst } = await import("vite")

// Names, units and other words that are the same in every language.
const KEEP = new Set(
  "vilauncher viportlive victory portwatch mandyhub zettrox github gitlab nvidia amd intel apple microsoft windows winget git rust cargo fps ram vram gpu gpus cpu zip url readme sha json png jpg exe ps1 msvc d3d12 agility sdk nuget xbox scarlett radeon geforce rtx tauri ellion369 thegameonaddict marvisak victory_name".split(
    " ",
  ),
)
const naturalWords = (text) => (text.match(/[A-Za-z][A-Za-z']*/g) ?? []).filter((w) => /[a-z]{2,}/.test(w) && !KEEP.has(w.toLowerCase()))
const isSentence = (text) => {
  const trimmed = text.trim()
  return /^[A-Z]/.test(trimmed) && /\s/.test(trimmed) && naturalWords(trimmed).length >= 2
}

const UI_ATTRIBUTES = new Set(
  "aria-label aria-description aria-valuetext placeholder title alt label hint description detail text heading caption tooltip message emptyText confirmLabel cancelLabel subtitle".split(" "),
)
const UI_PROPERTIES = new Set("label detail title hint description text placeholder message heading caption subtitle".split(" "))
const UI_CALLS = /^(toast(\.\w+)?|pickFolder|pickFile|saveFile|Error|alert|confirm)$/

function calleeName(node) {
  if (!node) return ""
  if (node.type === "Identifier") return node.name
  if (node.type === "MemberExpression") return `${calleeName(node.object)}.${node.property?.name ?? ""}`
  return ""
}

/** The strings an expression shows as it is: itself, either branch, a template's text. */
function rendered(node) {
  if (!node) return []
  switch (node.type) {
    case "Literal":
      return typeof node.value === "string" ? [node] : []
    case "TemplateLiteral":
      return node.quasis
    case "ConditionalExpression":
      return [...rendered(node.consequent), ...rendered(node.alternate)]
    case "LogicalExpression":
      return [...rendered(node.left), ...rendered(node.right)]
    case "JSXExpressionContainer":
      return rendered(node.expression)
    case "ParenthesizedExpression":
    case "TSAsExpression":
    case "TSSatisfiesExpression":
      return rendered(node.expression)
    default:
      return []
  }
}

const textOf = (node) => (node.type === "TemplateElement" ? (node.value?.cooked ?? node.value?.raw ?? "") : node.type === "JSXText" ? node.value : String(node.value))

async function sourceFiles(dir, out = []) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name)
    const rel = relative(join(ROOT, "src"), path).split(sep).join("/")
    if (entry.isDirectory()) {
      if (rel === "i18n" || rel === "dev") continue
      await sourceFiles(path, out)
    } else if (/\.(ts|tsx)$/.test(entry.name) && !entry.name.endsWith(".d.ts") && !/\.test\.tsx?$/.test(entry.name)) out.push(path)
  }
  return out
}

let codeProblems = 0
for (const file of await sourceFiles(join(ROOT, "src"))) {
  const code = await readFile(file, "utf8")
  const lines = code.split("\n")
  const starts = [0]
  for (let i = 0; i < code.length; i++) if (code[i] === "\n") starts.push(i + 1)
  const lineOf = (offset) => {
    let lo = 0
    let hi = starts.length - 1
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1
      if (starts[mid] <= offset) lo = mid
      else hi = mid - 1
    }
    return lo + 1
  }
  const ignored = (line) => /i18n-ignore/.test(lines[line - 1] ?? "") || /i18n-ignore/.test(lines[line - 2] ?? "")
  let ast
  try {
    ast = parseAst(code, { lang: file.endsWith(".tsx") ? "tsx" : "ts" })
  } catch (error) {
    errors.push(`${relative(ROOT, file)}: could not be parsed (${error.message.split("\n")[0]})`)
    continue
  }
  const seen = new Set()
  const flag = (node, why) => {
    if (seen.has(node)) return
    seen.add(node)
    const text = textOf(node).replace(/\s+/g, " ").trim()
    if (!text || naturalWords(text).length === 0) return
    const line = lineOf(node.start ?? 0)
    if (ignored(line)) return
    codeProblems++
    errors.push(`${relative(ROOT, file).split(sep).join("/")}:${line}: ${why}: ${JSON.stringify(text.slice(0, 80))}`)
  }

  const visit = (node, parent, key) => {
    if (!node || typeof node !== "object") return
    if (Array.isArray(node)) {
      for (const child of node) visit(child, parent, key)
      return
    }
    if (typeof node.type !== "string") return
    switch (node.type) {
      case "ImportDeclaration":
      case "ExportAllDeclaration":
      case "TSTypeAnnotation":
      case "TSTypeAliasDeclaration":
      case "TSInterfaceDeclaration":
      case "TSLiteralType":
        return
      case "ExportNamedDeclaration":
        if (node.source) return
        break
      case "CallExpression": {
        const name = calleeName(node.callee)
        // t("a.key") and console.log("…") are not text for the player.
        if (name === "t" || name.endsWith(".t") || name.startsWith("console.")) return
        if (UI_CALLS.test(name) || UI_CALLS.test(name.replace(/^new\s+/, ""))) for (const arg of node.arguments) for (const s of rendered(arg)) flag(s, `text in ${name}()`)
        // run(action, "Saved"): the second argument is a toast.
        if (name === "run" && node.arguments[1]) for (const s of rendered(node.arguments[1])) flag(s, "toast text in run()")
        break
      }
      case "NewExpression": {
        const name = calleeName(node.callee)
        if (UI_CALLS.test(name)) for (const arg of node.arguments) for (const s of rendered(arg)) flag(s, `text in new ${name}()`)
        break
      }
      case "JSXText":
        flag(node, "JSX text")
        return
      case "JSXAttribute": {
        const name = node.name?.type === "JSXIdentifier" ? node.name.name : ""
        if (name === "k") return
        if (UI_ATTRIBUTES.has(name)) for (const s of rendered(node.value)) flag(s, `${name}=`)
        break
      }
      case "JSXExpressionContainer":
        if (parent && (parent.type === "JSXElement" || parent.type === "JSXFragment")) for (const s of rendered(node)) flag(s, "text in JSX")
        break
      case "Property": {
        const name = node.key?.type === "Identifier" ? node.key.name : node.key?.type === "Literal" ? String(node.key.value) : ""
        if (UI_PROPERTIES.has(name)) for (const s of rendered(node.value)) flag(s, `property ${name}`)
        // Keys are not text.
        visit(node.value, node, "value")
        return
      }
      case "Literal":
        if (typeof node.value === "string" && isSentence(node.value)) flag(node, "English sentence")
        return
      case "TemplateLiteral":
        for (const q of node.quasis) if (isSentence(textOf(q))) flag(q, "English sentence")
        break
    }
    for (const [childKey, child] of Object.entries(node)) {
      if (childKey === "type" || childKey === "start" || childKey === "end" || childKey === "range" || childKey === "loc") continue
      if (child && typeof child === "object") visit(child, node, childKey)
    }
  }
  visit(ast.body, ast, "body")
}

// --- Report ------------------------------------------------------------------

for (const w of warnings) console.log(`warn  ${w}`)
for (const e of errors) console.log(`error ${e}`)
console.log(
  `i18n: ${messageCount} messages in ${namespaces.length} namespaces, ${locales.length} languages (${locales.join(", ")}); ` +
    `${errors.length} problem(s)${codeProblems ? ` (${codeProblems} in the code)` : ""}, ${warnings.length} warning(s).`,
)
process.exit(errors.length ? 1 : 0)
