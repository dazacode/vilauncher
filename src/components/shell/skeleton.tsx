import { cn } from "@/lib/utils"

/** A placeholder bar with a slow sheen; it stays still for reduced-motion users. */
export function Bone({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <span
      aria-hidden
      style={style}
      className={cn(
        "block rounded-sm bg-fill-2 bg-[linear-gradient(90deg,transparent_30%,rgb(255_255_255/0.05)_50%,transparent_70%)] bg-size-[200%_100%] animate-[busy_1.8s_ease-in-out_infinite] motion-reduce:animate-none",
        className,
      )}
    />
  )
}

const WIDTHS: [string, string][] = [
  ["w-[56%]", "w-[38%]"],
  ["w-[44%]", "w-[52%]"],
  ["w-[62%]", "w-[30%]"],
  ["w-[48%]", "w-[44%]"],
]

/** The project list while it loads: the shape of the rows it will become. */
export function SidebarSkeleton({ rows = 7 }: { rows?: number }) {
  return (
    <div aria-hidden className="pt-4">
      <Bone className="mx-2 mb-2 h-3 w-16" />
      {Array.from({ length: rows }, (_, i) => {
        const [a, b] = WIDTHS[i % WIDTHS.length]
        return (
          <div key={i} className="flex h-[52px] items-center gap-3 px-2">
            <Bone className="size-8 shrink-0 rounded-[8px]" />
            <span className="flex-1 space-y-2">
              <Bone className={cn("h-3", a)} />
              <Bone className={cn("h-2.5", b)} />
            </span>
          </div>
        )
      })}
    </div>
  )
}
