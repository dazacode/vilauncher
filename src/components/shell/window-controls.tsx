import { useEffect, useState } from "react"
import { useT } from "@/i18n"
import { inTauri } from "@/lib/api"
import { cn } from "@/lib/utils"

async function win() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window")
  return getCurrentWindow()
}

/** Minimise, maximise and close: thin Windows 11 glyphs over the art. */
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
  const button = "no-drag grid h-full w-[42px] cursor-pointer place-items-center text-[#d4d2d8] transition-colors duration-100 hover:bg-white/[0.1] hover:text-white"
  const glyph = { viewBox: "0 0 10 10", className: "size-[10px]", fill: "none", stroke: "currentColor", strokeWidth: 1, "aria-hidden": true } as const
  return (
    <div className={cn("flex h-[28px] items-stretch", className)}>
      <button type="button" aria-label={t("shell.window.minimise")} className={button} onClick={() => void win().then((w) => w.minimize())}>
        <svg {...glyph}>
          <path d="M0 5.5h10" />
        </svg>
      </button>
      <button type="button" aria-label={maximized ? t("shell.window.restore") : t("shell.window.maximise")} className={button} onClick={() => void win().then((w) => w.toggleMaximize())}>
        {maximized ? (
          <svg {...glyph}>
            <rect x="0.5" y="2.5" width="7" height="7" />
            <path d="M2.5 2.5v-2h7v7h-2" />
          </svg>
        ) : (
          <svg {...glyph}>
            <rect x="0.5" y="0.5" width="9" height="9" />
          </svg>
        )}
      </button>
      <button type="button" aria-label={t("common.close")} className={cn(button, "hover:bg-[#c42b1c] hover:text-white")} onClick={() => void win().then((w) => w.close())}>
        <svg {...glyph}>
          <path d="M0.5 0.5l9 9M9.5 0.5l-9 9" />
        </svg>
      </button>
    </div>
  )
}
