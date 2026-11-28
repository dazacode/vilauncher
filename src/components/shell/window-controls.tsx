import { Minus, Square, X } from "lucide-react"
import { useEffect, useState } from "react"
import { useT } from "@/i18n"
import { inTauri } from "@/lib/api"
import { cn } from "@/lib/utils"

async function win() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window")
  return getCurrentWindow()
}

/** Minimise, maximise and close, drawn the way Windows 11 draws them. */
export function WindowControls({ className }: { className?: string }) {
  const t = useT()
  const [maximized, setMaximized] = useState(false)
  useEffect(() => {
    if (!inTauri) return
    let off: (() => void) | undefined
    void win().then(async (w) => {
      setMaximized(await w.isMaximized())
      off = await w.onResized(async () => setMaximized(await w.isMaximized()))
    })
    return () => off?.()
  }, [])
  const button = "no-drag grid h-full w-[46px] place-items-center text-fg-2 transition-colors duration-100 hover:bg-fill-3 hover:text-fg"
  return (
    <div className={cn("flex h-8 items-stretch", className)}>
      <button type="button" aria-label={t("shell.window.minimise")} className={button} onClick={() => void win().then((w) => w.minimize())}>
        <Minus className="size-4" strokeWidth={1.5} />
      </button>
      <button type="button" aria-label={maximized ? t("shell.window.restore") : t("shell.window.maximise")} className={button} onClick={() => void win().then((w) => w.toggleMaximize())}>
        {maximized ? (
          <svg viewBox="0 0 16 16" className="size-3.5" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden>
            <rect x="2.5" y="4.5" width="9" height="9" rx="1.5" />
            <path d="M5 4.5V3.5a1.5 1.5 0 0 1 1.5-1.5h6A1.5 1.5 0 0 1 14 3.5v6a1.5 1.5 0 0 1-1.5 1.5h-1" />
          </svg>
        ) : (
          <Square className="size-3.5" strokeWidth={1.5} />
        )}
      </button>
      <button type="button" aria-label={t("common.close")} className={cn(button, "hover:bg-[#c42b1c] hover:text-white")} onClick={() => void win().then((w) => w.close())}>
        <X className="size-4" strokeWidth={1.5} />
      </button>
    </div>
  )
}
