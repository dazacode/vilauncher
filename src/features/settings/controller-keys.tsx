import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Group, Row } from "@/components/ui/group"
import { Switch } from "@/components/ui/switch"
import { useT } from "@/i18n"
import { run, useStore } from "@/lib/store"
import type { ControllerControl } from "@/lib/types"

/** The controls VIctory reads from the keyboard (src-tauri/src/controller_keys.rs), by id. */
const CONTROL_IDS = [
  "menu", "view", "a", "b", "x", "y", "up", "down", "left", "right",
  "left_shoulder", "right_shoulder", "left_stick_click", "right_stick_click",
  "left_stick_up", "left_stick_left", "left_stick_down", "left_stick_right",
  "right_stick_up", "right_stick_left", "right_stick_down", "right_stick_right",
  "left_trigger", "right_trigger",
] as const
type ControlId = (typeof CONTROL_IDS)[number]

const ESCAPE = 27
/** Pressed alone, these do not choose a key. */
const MODIFIERS = [16, 17, 18, 91, 92]
/** Keys whose names are printed on the keyboard, not translated. */
const NAMED: Record<number, string> = { 8: "Backspace", 9: "Tab", 13: "Enter", 32: "Space", 37: "←", 38: "↑", 39: "→", 40: "↓" }

/** Virtual-key codes VIctory reads: letters and digits are their own character. */
function isLetterOrDigit(code: number): boolean {
  return (code >= 0x30 && code <= 0x39) || (code >= 0x41 && code <= 0x5a)
}

export function ControllerKeysGroup() {
  const t = useT()
  const keys = useStore((s) => s.snap)!.settings.controller_keys
  const [capturing, setCapturing] = useState<string | null>(null)

  // While a control waits for its key, the next key press is the choice (Esc cancels).
  useEffect(() => {
    if (capturing === null) return
    const onKey = (event: KeyboardEvent) => {
      if (MODIFIERS.includes(event.keyCode)) return
      event.preventDefault()
      event.stopPropagation()
      const id = capturing
      setCapturing(null)
      if (event.keyCode !== ESCAPE) void run({ type: "set_controller_key", control: id, key: event.keyCode })
    }
    window.addEventListener("keydown", onKey, true)
    return () => window.removeEventListener("keydown", onKey, true)
  }, [capturing])

  const keyLabel = (code: number) => {
    if (NAMED[code]) return NAMED[code]
    if (isLetterOrDigit(code)) return String.fromCharCode(code)
    return t("settings.controllerKeys.unknownKey", { code: String(code) })
  }
  const controlLabel = (control: ControllerControl) => {
    const id = control.id as ControlId
    return CONTROL_IDS.includes(id) ? t(`settings.controllerKeys.controls.${id}`) : control.label
  }
  const changed = keys.controls.filter((control) => control.key !== control.default_key)

  return (
    <Group
      title={t("settings.controllerKeys.title")}
      description={t("settings.controllerKeys.description")}
      action={
        changed.length > 0 ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={async () => {
              for (const control of changed) await run({ type: "set_controller_key", control: control.id, key: null })
            }}
          >
            {t("settings.controllerKeys.resetAll")}
          </Button>
        ) : undefined
      }
    >
      <Row label={t("settings.controllerKeys.on")} hint={t("settings.controllerKeys.onHint")}>
        <Switch
          checked={keys.on}
          onCheckedChange={(on) => void run({ type: "set_controller_keys", on })}
          aria-label={t("settings.controllerKeys.on")}
        />
      </Row>
      {keys.controls.map((control) => (
        <Row
          key={control.id}
          label={controlLabel(control)}
          hint={control.key !== control.default_key ? t("settings.controllerKeys.keyDefault", { key: keyLabel(control.default_key) }) : undefined}
        >
          <span className="min-w-14 text-right font-mono text-secondary text-fg-2">{keyLabel(control.key)}</span>
          <Button
            size="sm"
            variant={capturing === control.id ? "primary" : "ghost"}
            onClick={() => setCapturing(capturing === control.id ? null : control.id)}
          >
            {capturing === control.id ? t("settings.controllerKeys.press") : t("settings.controllerKeys.change")}
          </Button>
          {control.key !== control.default_key && (
            <Button size="sm" variant="ghost" onClick={() => void run({ type: "set_controller_key", control: control.id, key: null })}>
              {t("settings.controllerKeys.reset")}
            </Button>
          )}
        </Row>
      ))}
    </Group>
  )
}
