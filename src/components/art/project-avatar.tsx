import { FolderGit2, HardDrive, Package } from "lucide-react"
import { useState } from "react"
import type { Project } from "@/lib/types"
import { cn } from "@/lib/utils"

/** The GitHub account picture at `px` pixels. Big uses share one size, so the hero's cover and its wash load one file. */
export function avatarUrl(owner: string, px: number) {
  return `https://github.com/${owner}.png?size=${px}`
}

/** The project's face: its GitHub account's picture, or an icon for links and imports. */
export function ProjectAvatar({ project, owner, size = 28, className }: { project: Project; owner: string | null; size?: number; className?: string }) {
  const [failed, setFailed] = useState(false)
  const style = { width: size, height: size, borderRadius: size * 0.24 }
  if (owner && !failed) {
    return (
      <img
        src={avatarUrl(owner, size > 48 ? 256 : size * 3)}
        alt=""
        style={style}
        draggable={false}
        loading="lazy"
        onError={() => setFailed(true)}
        className={cn("shrink-0 bg-fill-2 object-cover ring-1 ring-line ring-inset", className)}
      />
    )
  }
  const Icon = project.kind === "zip" ? Package : project.kind === "local" ? HardDrive : FolderGit2
  return (
    <span style={style} className={cn("grid shrink-0 place-items-center bg-fill-3 text-fg-2 ring-1 ring-line ring-inset", className)}>
      <Icon style={{ width: size * 0.5, height: size * 0.5 }} strokeWidth={1.75} />
    </span>
  )
}
