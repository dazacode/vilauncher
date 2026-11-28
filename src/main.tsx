import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import { inTauri, platformName } from "./lib/api"
import "./styles.css"

// Inside the window the system's glass (Mica on Windows, the sidebar material
// on macOS) shows through; elsewhere, a solid ground. The platform class drives
// window chrome: macOS keeps the traffic lights, Windows draws its own.
const platform = platformName()
if (inTauri && platform !== "other") document.body.classList.add("has-glass")
document.body.classList.add(platform === "macos" ? "is-mac" : "is-windows")
document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea, [data-selectable]")) e.preventDefault()
})

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
