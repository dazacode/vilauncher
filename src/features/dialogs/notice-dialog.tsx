import { Languages } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Dialog } from "@/components/ui/dialog"
import { LanguageSelect } from "@/features/shared/language-select"
import { Trans, useLocale, useT } from "@/i18n"
import { inTauri } from "@/lib/api"
import { run, useStore } from "@/lib/store"

/**
 * The notice's numbered points, in order: LEGAL.md's notice (also in the README and the
 * installer), in the player's language from the `legal.notice` messages. The English one governs.
 */
const POINTS = ["game", "responsibility", "code", "warranty", "affiliation", "licence", "takedowns", "data"] as const

function Point({ n, id }: { n: number; id: (typeof POINTS)[number] }) {
  // "<b>You must provide the game.</b> Rest…": the lead sentence carries the point.
  return (
    <li className="flex gap-3">
      <span className="w-4 shrink-0 text-body font-semibold text-fg-3 tabular-nums">{n}</span>
      <p className="text-body text-fg-2">
        <Trans k={`legal.notice.points.${id}`} tags={{ b: (s) => <span className="font-semibold text-fg">{s}</span> }} />
      </p>
    </li>
  )
}

/** The window's language, as in Settings, so the notice can be read in it before agreeing. */
function LanguagePicker() {
  return (
    <div className="mr-auto flex items-center gap-2">
      <Languages aria-hidden className="size-4 shrink-0 text-fg-3" />
      <LanguageSelect />
    </div>
  )
}

/**
 * The legal notice. On the first start it must be agreed to before the
 * library opens (no way around it but Quit); from Settings it is only read.
 */
export function NoticeDialog() {
  const t = useT()
  const locale = useLocale()
  const accepted = useStore((s) => s.snap?.notice_accepted ?? true)
  const dialog = useStore((s) => s.dialog)
  const open = useStore((s) => s.open)
  const reading = dialog?.kind === "notice"
  if (accepted && !reading) return null
  return (
    <Dialog
      open
      dismissable={accepted}
      onOpenChange={(o) => !o && accepted && open(null)}
      title={t("legal.notice.title")}
      description={t("legal.notice.intro")}
      className="w-[min(600px,calc(100vw-48px))]"
      footer={
        accepted ? (
          <>
            <Button variant="primary" onClick={() => open(null)}>{t("common.close")}</Button>
            <LanguagePicker />
          </>
        ) : (
          <>
            <Button variant="primary" onClick={() => void run({ type: "accept_notice" })}>{t("legal.notice.agree")}</Button>
            <Button
              variant="ghost"
              onClick={async () => {
                if (!inTauri) return
                const { getCurrentWindow } = await import("@tauri-apps/api/window")
                await getCurrentWindow().close()
              }}
            >
              {t("legal.notice.quit")}
            </Button>
            <LanguagePicker />
          </>
        )
      }
    >
      <ol className="space-y-4" data-selectable>
        {POINTS.map((id, i) => (
          <Point key={id} n={i + 1} id={id} />
        ))}
      </ol>
      <p className="mt-5 text-secondary text-fg-3">{t("legal.notice.closing")}</p>
      {locale !== "en" && <p className="mt-2 text-secondary text-fg-3">{t("legal.notice.translationNote")}</p>}
    </Dialog>
  )
}
