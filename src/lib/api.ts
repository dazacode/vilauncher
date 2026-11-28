import { t } from "@/i18n"
import type { Action, LogView, Snapshot } from "./types"

/**
 * The system this runs on, from the browser or webview's own description. The
 * snapshot's `brand.platform` is the launcher's answer; this covers the first
 * paint and the browser mock.
 */
export function platformName(): "windows" | "macos" | "other" {
  const ua = typeof navigator === "undefined" ? "" : navigator.userAgent
  if (/Macintosh|Mac OS X/i.test(ua)) return "macos"
  if (/Windows/i.test(ua)) return "windows"
  return "other"
}

/** True inside the Tauri window; false in a plain browser (development, with a mock launcher). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

/**
 * The window's own glass and appearance, as the theme asks (main.rs window_look): light or dark,
 * glass on or off, and the tint Acrylic takes on Windows 10. Nothing in a browser.
 */
export async function setWindowLook(dark: boolean, glass: boolean, tint: [number, number, number]) {
  if (!inTauri) return
  const { invoke } = await import("@tauri-apps/api/core")
  await invoke("window_look", { dark, glass, tint }).catch(() => undefined)
}

interface Backend {
  snapshot(): Promise<Snapshot>
  act(action: Action): Promise<unknown>
  log(from: number, repo?: string): Promise<LogView>
  onChange(listener: () => void): Promise<() => void>
}

const tauriBackend: Backend = {
  async snapshot() {
    const { invoke } = await import("@tauri-apps/api/core")
    return invoke<Snapshot>("snapshot")
  },
  async act(action) {
    const { invoke } = await import("@tauri-apps/api/core")
    return invoke("act", { action })
  },
  async log(from, repo) {
    const { invoke } = await import("@tauri-apps/api/core")
    return invoke<LogView>("log", { from, repo })
  },
  async onChange(listener) {
    const { listen } = await import("@tauri-apps/api/event")
    return listen("launcher://changed", () => listener())
  },
}

let backend: Promise<Backend> | null = null

export function getBackend(): Promise<Backend> {
  if (!backend) {
    backend = inTauri ? Promise.resolve(tauriBackend) : import("@/dev/mock").then((m) => m.mockBackend)
  }
  return backend
}

export async function act<T = unknown>(action: Action): Promise<T> {
  return (await getBackend()).act(action) as Promise<T>
}

/** A file or folder picker (the system's own dialog); null when cancelled. */
export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return null
  const { open } = await import("@tauri-apps/plugin-dialog")
  const picked = await open({ directory: true, title })
  return typeof picked === "string" ? picked : null
}

export async function pickFile(title: string, filters: { name: string; extensions: string[] }[]): Promise<string | null> {
  if (!inTauri) return null
  const { open } = await import("@tauri-apps/plugin-dialog")
  const picked = await open({ title, filters, multiple: false })
  return typeof picked === "string" ? picked : null
}

export async function saveFile(title: string, defaultPath: string, filters: { name: string; extensions: string[] }[]) {
  if (!inTauri) return null
  const { save } = await import("@tauri-apps/plugin-dialog")
  return save({ title, defaultPath, filters })
}

/**
 * Copies text: the clipboard plugin, then the browser's clipboard, then a hidden text field.
 * Throws only when every route fails, so the caller can say so.
 */
export async function copyText(text: string) {
  if (inTauri) {
    try {
      const { writeText } = await import("@tauri-apps/plugin-clipboard-manager")
      await writeText(text)
      return
    } catch {
      // Try the next route.
    }
  }
  try {
    await navigator.clipboard.writeText(text)
    return
  } catch {
    // Try the next route.
  }
  const field = document.createElement("textarea")
  field.value = text
  field.setAttribute("readonly", "")
  field.style.position = "fixed"
  field.style.opacity = "0"
  document.body.appendChild(field)
  field.select()
  const copied = document.execCommand("copy")
  field.remove()
  if (!copied) throw new Error(t("common.errors.copyFailed"))
}

/**
 * Opens a web link: the opener plugin, then the launcher itself (the system's
 * shell opens the default browser).
 * Throws only when both fail, so the caller can say so.
 */
export async function openUrl(url: string) {
  if (inTauri) {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener")
      await openUrl(url)
      return
    } catch {
      const { invoke } = await import("@tauri-apps/api/core")
      await invoke("act", { action: { type: "open_link", url } })
      return
    }
  }
  window.open(url, "_blank", "noopener")
}

/** A local picture as the window can load it. */
export async function fileSrc(path: string) {
  if (!inTauri) return path
  const { convertFileSrc } = await import("@tauri-apps/api/core")
  return convertFileSrc(path)
}
