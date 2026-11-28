import { useEffect } from "react"
import { Toaster } from "sonner"
import { Sidebar } from "@/components/shell/sidebar"
import { WindowControls } from "@/components/shell/window-controls"
import { TooltipProvider } from "@/components/ui/tooltip"
import { Dialogs } from "@/features/dialogs/dialogs"
import { useT } from "@/i18n"
import { LibrarySkeleton, LibraryView } from "@/features/library/library-view"
import { SettingsView } from "@/features/settings/settings-view"
import { act, getBackend, platformName } from "@/lib/api"
import { scheduleRefresh, useStore } from "@/lib/store"

export function App() {
  const t = useT()
  const snap = useStore((s) => s.snap)
  const mac = (snap?.brand.platform ?? platformName()) === "macos"
  const view = useStore((s) => s.view)
  useEffect(() => {
    let off: (() => void) | undefined
    void getBackend().then(async (b) => {
      off = await b.onChange(scheduleRefresh)
      // The launcher follows the system's language unless the player chose one.
      await act({ type: "set_system_languages", languages: [...navigator.languages] }).catch(() => undefined)
      scheduleRefresh()
    })
    return () => off?.()
  }, [])
  return (
    <TooltipProvider>
      <div className="flex h-full">
        <Sidebar />
        <main className="relative min-w-0 flex-1">
          {/* The top of the window drags it; the controls sit over the art. WebView2 (Windows)
              moves the window from the `drag` class; WebKitGTK and WKWebView ignore it, so Tauri
              drags from data-tauri-drag-region there. */}
          {/* macOS draws its own traffic lights (top left, over the sidebar), so no buttons here. */}
          <div data-tauri-drag-region="deep" className="drag absolute inset-x-0 top-0 z-30 flex h-10 justify-end">
            {!mac && <WindowControls />}
          </div>
          {!snap ? view === "library" && <LibrarySkeleton /> : view === "library" ? <LibraryView /> : <SettingsView />}
        </main>
      </div>
      <Dialogs />
      <Toaster
        position="bottom-right"
        theme="dark"
        containerAriaLabel={t("shell.notifications")}
        toastOptions={{ className: "!glass !rounded-lg !text-fg !border-line !text-[13px]" }}
      />
    </TooltipProvider>
  )
}
