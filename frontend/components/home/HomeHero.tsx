import { HeroSearch } from "./HeroSearch"
import Link from "next/link"
import { ArrowRight, Briefcase, Database, GitBranch, Search, ShieldCheck } from "lucide-react"
import type { BuildInfo, CorpusStatus, SystemHealth } from "@/lib/types"
import type { DataSource } from "@/lib/data-state"

interface HomeHeroProps {
  corpus: CorpusStatus
  health: SystemHealth
  build: BuildInfo
  dataSource: DataSource
}

export function HomeHero({ corpus, health, build, dataSource }: HomeHeroProps) {
  const citationCoverage = formatPercent(corpus.citations.coveragePercent)
  const liveLabel = dataSource === "live" && health.api === "connected" ? "Live graph" : "Fallback graph"

  return (
    <section className="border-b border-border bg-card px-4 py-8 sm:px-6 lg:px-8">
      <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[minmax(0,1fr)_23rem] lg:items-end">
        <div className="min-w-0">
          <div className="mb-5 inline-flex max-w-full items-center gap-2 rounded-md border border-border bg-background px-3 py-1 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <span className="h-2 w-2 rounded-full bg-primary" aria-hidden="true" />
            <span className="truncate">ORSGraph / {corpus.source} / {corpus.editionYear}</span>
          </div>

          <h1 className="max-w-4xl text-balance text-3xl font-semibold tracking-normal text-foreground sm:text-4xl lg:text-5xl">
            The command center for source-backed Oregon legal work.
          </h1>

          <p className="mt-4 max-w-3xl text-pretty text-sm leading-6 text-muted-foreground sm:text-base">
            Search statutes, ask graph-grounded questions, open matters, and watch runtime health from one operational home base.
          </p>

          <div className="mt-6 flex flex-wrap gap-2">
            <Link
              href="/ask"
              className="inline-flex min-h-10 items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              Ask ORSGraph
              <ArrowRight className="h-4 w-4" />
            </Link>
            <Link
              href="/search"
              className="inline-flex min-h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium text-foreground transition-colors hover:border-primary/50 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              <Search className="h-4 w-4" />
              Explore Search
            </Link>
            <Link
              href="/matters"
              className="inline-flex min-h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium text-foreground transition-colors hover:border-primary/50 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              <Briefcase className="h-4 w-4" />
              Open Matters
            </Link>
          </div>

          <HeroSearch />
        </div>

        <aside aria-label="Home graph status" className="rounded-md border border-border bg-background p-4">
          <div className="mb-3 flex items-center justify-between gap-3">
            <div>
              <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">runtime</div>
              <div className="mt-1 text-sm font-semibold text-foreground">{liveLabel}</div>
            </div>
            <span className="rounded bg-primary/10 px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-primary">
              {build.graphEdition ?? `ORS ${corpus.editionYear}`}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-px overflow-hidden rounded-md border border-border bg-border">
            <HeroStat
              icon={Database}
              label="sections"
              value={formatNumber(corpus.counts.sections)}
            />
            <HeroStat
              icon={GitBranch}
              label="edges"
              value={formatNumber(corpus.counts.neo4jRelationships)}
            />
            <HeroStat
              icon={ShieldCheck}
              label="citations"
              value={`${citationCoverage}%`}
              helper="resolved"
            />
            <HeroStat
              icon={Database}
              label="chunks"
              value={formatNumber(corpus.counts.retrievalChunks)}
            />
          </div>

          <dl className="mt-4 space-y-2 border-t border-border pt-3 text-xs">
            <StatusRow label="API" value={health.api} state={health.api === "connected" ? "ok" : "warning"} />
            <StatusRow label="Neo4j" value={health.neo4j} state={health.neo4j === "connected" ? "ok" : "warning"} />
          </dl>
        </aside>
      </div>
    </section>
  )
}

function HeroStat({
  icon: Icon,
  label,
  value,
  helper,
}: {
  icon: typeof Database
  label: string
  value: string
  helper?: string
}) {
  return (
    <div className="bg-card p-3">
      <div className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <Icon className="h-3 w-3" />
        {label}
      </div>
      <div className="mt-1 font-mono text-lg font-semibold tabular-nums text-foreground">{value}</div>
      {helper && <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{helper}</div>}
    </div>
  )
}

function StatusRow({
  label,
  value,
  state,
}: {
  label: string
  value: string
  state: "ok" | "warning"
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className={state === "ok" ? "text-success" : "text-warning"}>{value}</dd>
    </div>
  )
}

function formatNumber(value: number) {
  return new Intl.NumberFormat(undefined, { notation: value >= 100_000 ? "compact" : "standard" }).format(value)
}

function formatPercent(value: number) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value)
}
