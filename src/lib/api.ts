import type { Action, LogView, Snapshot } from "./types"

/** True inside the Tauri window; false in a plain browser (development, with a mock launcher). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

interface Backend {
  snapshot(): Promise<Snapshot>
  act(action: Action): Promise<unknown>
  log(from: number): Promise<LogView>
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
  async log(from) {
    const { invoke } = await import("@tauri-apps/api/core")
    return invoke<LogView>("log", { from })
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

export async function copyText(text: string) {
  if (inTauri) {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager")
    return writeText(text)
  }
  return navigator.clipboard.writeText(text)
}

export async function openUrl(url: string) {
  if (inTauri) {
    const { openUrl } = await import("@tauri-apps/plugin-opener")
    return openUrl(url)
  }
  window.open(url, "_blank", "noopener")
}

/** A local picture as the window can load it. */
export async function fileSrc(path: string) {
  if (!inTauri) return path
  const { convertFileSrc } = await import("@tauri-apps/api/core")
  return convertFileSrc(path)
}
