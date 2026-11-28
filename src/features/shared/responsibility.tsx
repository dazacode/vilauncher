import { AlertTriangle } from "lucide-react"
import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Dialog } from "@/components/ui/dialog"
import { type Key, useT } from "@/i18n"
import { run } from "@/lib/store"
import type { ResponsibilityId } from "@/lib/types"

/** A message of the warnings (legal.responsibility); none has placeholders. */
type WarningKey = Extract<Key, `legal.responsibility.${string}`>

/** The plain-language warning for each risky step, as message keys. Keep every risk listed. */
const WARNINGS: Record<ResponsibilityId, { titleKey: WarningKey; bodyKey: WarningKey; riskKeys: WarningKey[] }> = {
  "import-repo": {
    titleKey: "legal.responsibility.importRepo.title",
    bodyKey: "legal.responsibility.importRepo.body",
    riskKeys: [
      "legal.responsibility.importRepo.risks.build",
      "legal.responsibility.importRepo.risks.download",
      "legal.responsibility.importRepo.risks.write",
      "legal.responsibility.importRepo.risks.saves",
    ],
  },
  "agent-access": {
    titleKey: "legal.responsibility.agentAccess.title",
    bodyKey: "legal.responsibility.agentAccess.body",
    riskKeys: [
      "legal.responsibility.agentAccess.risks.read",
      "legal.responsibility.agentAccess.risks.scope",
      "legal.responsibility.agentAccess.risks.confirm",
      "legal.responsibility.agentAccess.risks.off",
    ],
  },
  "agent-patch": {
    titleKey: "legal.responsibility.agentPatch.title",
    bodyKey: "legal.responsibility.agentPatch.body",
    riskKeys: [],
  },
}

/**
 * Asks the player to accept responsibility before a risky step. The acceptance is
 * stored by the launcher (settings.acknowledged), so it is asked once per warning.
 * Continue stays disabled until the box is ticked.
 */
export function ResponsibilityDialog({
  open,
  onOpenChange,
  id,
  onAccept,
}: {
  open: boolean
  onOpenChange(open: boolean): void
  id: ResponsibilityId
  onAccept(): void
}) {
  const t = useT()
  const [agreed, setAgreed] = useState(false)
  const [busy, setBusy] = useState(false)
  const warning = WARNINGS[id]

  useEffect(() => {
    if (open) {
      setAgreed(false)
      setBusy(false)
    }
  }, [open])

  const accept = async () => {
    setBusy(true)
    // run() reports a failure as undefined (and shows the error), so only a saved acceptance goes on.
    const saved = await run({ type: "acknowledge", id })
    setBusy(false)
    if (saved === undefined) return
    onOpenChange(false)
    onAccept()
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t(warning.titleKey)}
      footer={
        <>
          <Button variant="primary" disabled={!agreed || busy} onClick={() => void accept()}>
            {t("common.continue")}
          </Button>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
        </>
      }
    >
      <p className="text-body text-fg-2">{t(warning.bodyKey)}</p>
      {warning.riskKeys.length > 0 && (
        <div className="mt-4 rounded-lg border border-warn/20 bg-warn/8 px-3.5 py-3">
          <p className="flex items-center gap-2 text-secondary font-medium text-fg">
            <AlertTriangle className="size-4 shrink-0 text-warn" /> {t("legal.responsibility.risks")}
          </p>
          <ul className="mt-2 list-disc space-y-1 pl-5 text-secondary text-fg-2">
            {warning.riskKeys.map((risk) => (
              <li key={risk}>{t(risk)}</li>
            ))}
          </ul>
        </div>
      )}
      <label className="mt-5 flex cursor-pointer items-start gap-2.5 text-body text-fg-2">
        <input
          type="checkbox"
          checked={agreed}
          onChange={(e) => setAgreed(e.target.checked)}
          className="mt-1 size-4"
        />
        {t("legal.responsibility.accept")}
      </label>
    </Dialog>
  )
}
