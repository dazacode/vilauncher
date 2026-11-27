import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import { inTauri } from "./lib/api"
import "./styles.css"

// Inside the window the system's glass (Mica) shows through; elsewhere, a solid ground.
if (inTauri && navigator.userAgent.includes("Windows")) document.body.classList.add("has-glass")
document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea, [data-selectable]")) e.preventDefault()
})

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
