import { useState } from "react"
import { Button, type ButtonProps } from "@/components/ui/button"
import { Dialog } from "@/components/ui/dialog"
import { useT } from "@/i18n"
import { run, useStore } from "@/lib/store"

/**
 * The one Update entry point, for every control that replaces an installed build.
 * `ask` starts the install, after the warning unless the player turned it off
 * ("Don't show this again"); `dialog` must be rendered once by the caller.
 */
export function useUpdateWarning(repo: string) {
  const t = useT()
  const skip = useStore((s) => s.snap?.settings.skip_update_warning ?? false)
  const [open, setOpen] = useState(false)
  const [dontShow, setDontShow] = useState(false)

  const update = async () => {
    setOpen(false)
    if (dontShow) await run({ type: "set_update_warning", show: false })
    void run({ type: "install", repo })
  }

  const dialog = (
    <Dialog
      open={open}
      onOpenChange={setOpen}
      title={t("library.updateDialog.title")}
      description={t("library.updateDialog.description")}
      footer={
        <>
          <Button variant="primary" onClick={() => void update()}>
            {t("library.action.update")}
          </Button>
          <Button variant="ghost" onClick={() => setOpen(false)}>
            {t("common.cancel")}
          </Button>
        </>
      }
    >
      <label className="flex cursor-pointer items-center gap-2.5 text-body text-fg-2">
        <input type="checkbox" checked={dontShow} onChange={(e) => setDontShow(e.target.checked)} className="size-4" />
        {t("library.updateDialog.dontShow")}
      </label>
    </Dialog>
  )

  return { ask: () => (skip ? void run({ type: "install", repo }) : setOpen(true)), dialog }
}

/** An Update button that goes through the warning. */
export function UpdateButton({ repo, children, ...props }: ButtonProps & { repo: string }) {
  const warning = useUpdateWarning(repo)
  return (
    <>
      <Button {...props} onClick={warning.ask}>
        {children}
      </Button>
      {warning.dialog}
    </>
  )
}
