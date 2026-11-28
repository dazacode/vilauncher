// Tauri's webviews ignore `-webkit-app-region`, so the `drag` class alone moves nothing.
// A press on a `drag` element (and not on a control or `no-drag` element inside it) asks
// the window to start moving; a double press maximises or restores it, as a title bar does.

const CONTROLS = "button, a, input, textarea, select, label, summary, [role=button], [contenteditable=true], .no-drag"

function dragRegion(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  const region = target.closest(".drag")
  if (!region) return false
  const control = target.closest(CONTROLS)
  return !control || !region.contains(control)
}

export function installWindowDrag() {
  document.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || !dragRegion(e.target)) return
    e.preventDefault()
    void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      const w = getCurrentWindow()
      return e.detail === 2 ? w.toggleMaximize() : w.startDragging()
    })
  })
}
