/** The VIL logo and "VILauncher" set as VIPortLive sets its name: VI heavy, the rest in pink script. */
export function Brand() {
  return (
    <div className="flex items-center gap-2">
      <img src="/logo.svg" alt="" className="size-6 rounded-[6px]" draggable={false} />
      <span className="flex items-baseline leading-none">
        <span className="font-display text-[17px] font-extrabold tracking-[-0.02em] text-fg">VI</span>
        <span className="ml-px font-script text-[21px] text-accent-fg">Launcher</span>
      </span>
    </div>
  )
}
