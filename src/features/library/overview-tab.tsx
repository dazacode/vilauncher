import { AlertTriangle, Check, ExternalLink, Minus, RefreshCw } from "lucide-react"
import { useMemo } from "react"
import { Button } from "@/components/ui/button"
import { Group, Row } from "@/components/ui/group"
import { useT } from "@/i18n"
import { openUrl } from "@/lib/api"
import { assess, type Check as CompatCheck, type Level, mainGpu, matchesCard, withSpec } from "@/lib/compat"
import { shortCommit } from "@/lib/format"
import { leadSentence } from "./hero"
import { UpdateButton } from "./update-button"
import { run, useAlias, useStore } from "@/lib/store"
import type { Project } from "@/lib/types"
import { cn } from "@/lib/utils"

export function OverviewTab({ project: p }: { project: Project }) {
  const t = useT()
  const screenshot = useStore((s) => s.screenshot)
  const busy = useStore((s) => s.snap?.job.running) || !!p.build_job?.running
  const alias = useAlias()(p)
  const built = shortCommit(p.built)
  return (
    <div className="grid gap-12 xl:grid-cols-[minmax(0,1fr)_340px]">
      <div className="min-w-0 space-y-8">
        {p.update && (
          <div className="flex items-center gap-4 rounded-lg border border-warn/25 bg-warn/8 px-4 py-3">
            <RefreshCw className="size-4 shrink-0 text-warn" />
            <div className="min-w-0 flex-1">
              <p className="text-body font-medium text-fg">{t("library.overview.newBuild")}</p>
              <p className="text-caption text-fg-3">
                {screenshot
                  ? t("library.overview.versionHidden")
                  : built
                    ? t("library.overview.newVersion", { version: shortCommit(p.update)!, current: built })
                    : t("library.overview.newVersionOlder", { version: shortCommit(p.update)! })}
              </p>
            </div>
            <Button size="sm" variant="ghost" onClick={() => void run({ type: "later", repo: p.repo, remote: p.update! })}>{t("common.later")}</Button>
            <UpdateButton repo={p.repo} size="sm" variant="primary" disabled={busy}>{t("library.overview.updateNow")}</UpdateButton>
          </div>
        )}
        {p.custom && p.kind === "git" && (
          <div className="flex items-start gap-3 rounded-lg border border-line bg-fill-1 px-4 py-3">
            <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warn" />
            <p className="text-body text-fg-2">{t("library.overview.linkWarning")}</p>
          </div>
        )}
        <section>
          <h2 className="font-display text-section font-semibold tracking-[-0.01em] text-fg">{t("library.overview.about")}</h2>
          <p className="mt-3 max-w-[68ch] text-lead text-fg-2" data-selectable>
            {screenshot
              ? t("library.overview.descriptionHidden")
              : aboutText(p.summary) ||
                (p.kind === "zip" ? t("library.overview.aboutZip") : p.kind === "local" ? t("library.overview.aboutLocal") : t("library.overview.aboutNone"))}
          </p>
        </section>
        {!screenshot && p.highlights.length > 0 && (
          <List title={t("library.overview.works")} items={p.highlights} mark={<Check className="size-4 text-fg-3" strokeWidth={2.25} />} />
        )}
        {!screenshot && p.blockers.length > 0 && <List title={t("library.overview.issues")} items={p.blockers} mark={<span className="size-1.5 rounded-full bg-fg-4" />} />}
        {!screenshot && p.advice.length > 0 && <AdviceList project={p} disabled={!!busy} />}
      </div>
      <div className="grid content-start gap-8 lg:grid-cols-2 xl:grid-cols-1">
        <YourPc project={p} />
        <Group title={t("library.details.title")}>
          <Row
            label={t("library.details.source")}
            hint={p.kind === "zip" ? t("library.details.sourceZip") : p.custom ? t("library.hero.addedByLink") : t("library.details.sourceLibrary")}
          />
          {p.kind === "git" && (
            <Row label={t("library.details.repository")} hint={<span className="break-all">{screenshot ? alias.repo : p.repo}</span>}>
              {p.web_url && !screenshot && (
                <Button size="icon-sm" variant="ghost" aria-label={t("library.hero.openRepo")} onClick={() => void openUrl(p.web_url!)}>
                  <ExternalLink />
                </Button>
              )}
            </Row>
          )}
          <Row
            label={t("library.details.installed")}
            hint={
              !p.installed
                ? t("library.details.notYet")
                : p.kind === "git" && !screenshot
                  ? built
                    ? t("library.details.version", { version: built })
                    : t("library.details.versionUnknown")
                  : t("library.details.installedYes")
            }
          />
          {p.activity && <Row label={t("library.details.updated")} hint={p.activity.updated_when} />}
          {p.activity?.last_run && <Row label={t("library.details.lastPlayed")} hint={p.activity.last_run} />}
        </Group>
      </div>
    </div>
  )
}

function List({ title, items, mark }: { title: string; items: string[]; mark: React.ReactNode }) {
  return (
    <section>
      <h3 className="text-lead font-semibold text-fg">{title}</h3>
      <ul className="mt-3 max-w-[68ch] space-y-2" data-selectable>
        {items.map((item) => (
          <li key={item} className="flex gap-3 text-body text-fg-2">
            <span className="grid h-5 w-4 shrink-0 place-items-center">{mark}</span>
            <span className="min-w-0">{item}</span>
          </li>
        ))}
      </ul>
    </section>
  )
}

/** The mark before a check, with its meaning for screen readers. */
/** Possible settings for this PC. Nothing is set until the player presses Apply. */
function AdviceList({ project: p, disabled }: { project: Project; disabled: boolean }) {
  const t = useT()
  return (
    <section>
      <h3 className="text-lead font-semibold text-fg">{t("library.advice.title")}</h3>
      <ul className="mt-3 max-w-[68ch] space-y-3" data-selectable>
        {p.advice.map((a) => (
          <li key={a.id} className="flex flex-wrap items-start gap-3 rounded-lg border border-line bg-fill-1 px-4 py-3">
            <div className="min-w-0 flex-1 space-y-1">
              <p className="text-body font-medium text-fg">{a.summary}</p>
              <p className="text-caption text-fg-3">
                {a.recommended ? t("library.advice.recommended") : t("library.advice.possible")} · <span className="font-mono">{a.switch}</span>
              </p>
              <p className="text-caption text-fg-3">{t("library.advice.source", { source: a.source })}</p>
            </div>
            <Button size="sm" variant="ghost" disabled={disabled} onClick={() => void run({ type: "apply_advice", repo: p.repo, id: a.id })}>
              {t("library.advice.apply")}
            </Button>
          </li>
        ))}
      </ul>
    </section>
  )
}

function Mark({ level }: { level: Level }) {
  const t = useT()
  switch (level) {
    case "good":
      return <Check className="size-4 text-ok" strokeWidth={2.25} aria-label={t("library.pc.mark.good")} />
    case "close":
      return <Minus className="size-4 text-warn" aria-label={t("library.pc.mark.close")} />
    case "concern":
      return <AlertTriangle className="size-4 text-warn" aria-label={t("library.pc.mark.concern")} />
    case "unknown":
      return <Minus className="size-4 text-fg-4" aria-label={t("library.pc.mark.unknown")} />
  }
}

/** How this build suits the player's PC, from the author's report and the launcher's look at the hardware. */
function YourPc({ project: p }: { project: Project }) {
  const t = useT()
  const detected = useStore((s) => s.snap?.system ?? null)
  const spec = useStore((s) => s.snap?.settings)
  const system = useMemo(() => withSpec(detected, spec), [detected, spec])
  const filter = useStore((s) => s.snap?.settings.gpu_filter ?? 0)
  const a = assess(p, system)
  // Builds don't say which processor they need, so the processor is shown, not compared.
  const cpu: CompatCheck = {
    label: t("library.pc.processor"),
    value: system?.cpu ?? null,
    level: "unknown",
    note: system?.cpu_cores ? t("library.pc.threads", { count: system.cpu_cores }) : t("library.pc.noCpuNeed"),
  }
  const vendor = mainGpu(system)?.vendor
  const target = vendor === "NVIDIA" ? 1 : 2
  const offerFilter = system && a.level === "concern" && matchesCard(p, system) === false && filter !== target
  return (
    <section className="space-y-2">
      <h3 className="px-1 text-body font-semibold text-fg">{t("library.pc.title")}</h3>
      {!system ? (
        <div className="divide-y divide-line overflow-hidden rounded-lg border border-line bg-fill-1" aria-busy aria-label={t("library.pc.looking")}>
          <div className="px-4 py-3">
            <div className="h-4 w-4/5 animate-pulse rounded-xs bg-fill-3" />
          </div>
          {[0, 1, 2].map((i) => (
            <div key={i} className="space-y-2 px-4 py-3">
              <div className="flex justify-between gap-4">
                <div className="h-4 w-24 animate-pulse rounded-xs bg-fill-3" />
                <div className="h-4 w-16 animate-pulse rounded-xs bg-fill-2" />
              </div>
              <div className="h-3 w-40 animate-pulse rounded-xs bg-fill-2" />
            </div>
          ))}
        </div>
      ) : (
        <div className="divide-y divide-line overflow-hidden rounded-lg border border-line bg-fill-1">
          <p className="px-4 py-3 text-body text-fg">{a.headline}</p>
          {[a.gpu, a.ram, a.vram, a.commit, cpu].map((c) => (
            <CheckRow key={c.label} check={c} />
          ))}
          {offerFilter && (
            <div className="px-4 py-2">
              <Button variant="ghost" size="sm" className="-ml-3" onClick={() => void run({ type: "set_gpu_filter", value: target })}>
                {t("library.pc.showForCard")}
              </Button>
            </div>
          )}
        </div>
      )}
      <p className="px-1 text-caption text-fg-4">{t("library.pc.memoryNote")}</p>
    </section>
  )
}

function CheckRow({ check: c }: { check: CompatCheck }) {
  const t = useT()
  const value = c.value ?? t("common.unknown")
  return (
    <div className="flex gap-3 px-4 py-3">
      <span className="grid h-5 w-4 shrink-0 place-items-center">
        <Mark level={c.level} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-4">
          <span className="shrink-0 text-body text-fg">{c.label}</span>
          <span className={cn("min-w-0 truncate text-right text-body", c.value == null ? "text-fg-4" : "text-fg-2")} title={value}>
            {value}
          </span>
        </div>
        <p className="mt-0.5 text-caption text-fg-3">{c.note}</p>
      </div>
    </div>
  )
}

/** The summary after the sentence the hero already shows, or all of it when the hero shows none. */
function aboutText(summary: string) {
  const lead = leadSentence(summary)
  const rest = lead ? summary.trim().slice(lead.length).trim() : summary
  return rest || summary
}
