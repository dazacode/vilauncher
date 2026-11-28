import { PawPrint } from "lucide-react"
import { useRef } from "react"
import { toast } from "sonner"
import { t } from "@/i18n"
import { pawsUnlocked, unlockPaws, useTheme } from "@/lib/theme"
import { cn } from "@/lib/utils"

/** The VIL logo and "VILauncher" set as VIPortLive sets its name: VI heavy, the rest in pink script. */
export function Brand({ size = "md" }: { size?: "md" | "lg" }) {
  const lg = size === "lg"
  const theme = useTheme()
  const clicks = useRef(0)
  // Seven clicks on the logo unlock a hidden look. Nothing else happens.
  const onLogo = () => {
    clicks.current += 1
    if (clicks.current < 7 || pawsUnlocked()) return
    unlockPaws()
    toast.success(t("shell.unlocked"))
  }
  return (
    <div className={cn("flex items-center", lg ? "gap-3 [-webkit-app-region:no-drag]" : "gap-2")} onClick={lg ? onLogo : undefined}>
      <img
        hidden={lg}
        src="/logo.svg"
        alt=""
        onClick={onLogo}
        data-tauri-drag-region="false"
        className={cn("[-webkit-app-region:no-drag]", lg ? "size-10 rounded-[10px] shadow-[0_0_24px_rgb(255_61_154/0.35)]" : "size-6 rounded-[6px]")}
        draggable={false}
      />
      <span className="flex items-baseline leading-none">
        <span className={cn("font-display font-extrabold tracking-[-0.02em] text-fg", lg ? "text-[24px]" : "text-[17px]")}>VI</span>
        {/* i18n-ignore: the name "VILauncher", in two styles */}
        <span className={cn("ml-px font-script text-accent-fg", lg ? "text-[29px]" : "text-[21px]")}>Launcher</span>
      </span>
      {theme === "paws" && <PawPrint aria-hidden className="ml-1 size-3.5 text-accent-fg" />}
    </div>
  )
}
