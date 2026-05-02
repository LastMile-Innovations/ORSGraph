"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Activity,
  AlertTriangle,
  Ban,
  Braces,
  Database,
  FileSearch,
  GitBranch,
  Layers,
  Play,
  RefreshCcw,
  SearchCode,
  ShieldCheck,
  Sparkles,
  Square,
} from "lucide-react"
import {
  cancelAdminJob,
  getAdminOverview,
  getAdminSource,
  isTerminalJob,
  listAdminSources,
  startAdminJob,
  type AdminJob,
  type AdminJobKind,
  type AdminJobParams,
  type AdminOverview,
  type AdminSourceDetail,
  type AdminSourceRegistryEntry,
  type AdminSourceRegistryResponse,
} from "@/lib/admin-api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Progress } from "@/components/ui/progress"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

const REFRESH_MS = 5000

const PRIORITY_FILTERS = ["all", "P0", "P1", "P2", "P3"] as const
const CONNECTOR_FILTERS = ["all", "implemented", "partial", "planned", "deferred"] as const

const WORKFLOWS: Array<{
  id: string
  label: string
  description: string
  icon: React.ComponentType<{ className?: string }>
  kind: AdminJobKind
  params: AdminJobParams
  disabled?: boolean
}> = [
  {
    id: "ors-registry-smoke",
    label: "ORS registry smoke",
    description: "Ingest two ORS chapters through the source registry.",
    icon: SearchCode,
    kind: "crawl",
    params: { out_dir: "data/sources", max_chapters: 2, edition_year: 2025 },
  },
  {
    id: "combine-graph",
    label: "Combine graph",
    description: "Merge source graph JSONL into the canonical graph directory.",
    icon: Layers,
    kind: "combine_graph",
    params: { out_dir: "data/sources", graph_dir: "data/graph" },
  },
  {
    id: "qc",
    label: "Run QC",
    description: "Validate the current graph directory and write a QC report.",
    icon: ShieldCheck,
    kind: "qc",
    params: { graph_dir: "data/graph", out_dir: "data/admin/qc", edition_year: 2025 },
  },
  {
    id: "seed-dry",
    label: "Seed dry run",
    description: "Validate graph JSONL files before writing to Neo4j.",
    icon: Database,
    kind: "seed_neo4j",
    params: { graph_dir: "data/graph", edition_year: 2025, dry_run: true },
  },
  {
    id: "materialize",
    label: "Materialize graph",
    description: "Create derived Neo4j relationships from the loaded graph.",
    icon: GitBranch,
    kind: "materialize_neo4j",
    params: { graph_dir: "data/graph", edition_year: 2025 },
  },
  {
    id: "embed",
    label: "Embed/index",
    description: "Run Neo4j embedding/index maintenance using configured credentials.",
    icon: Sparkles,
    kind: "embed_neo4j",
    params: { edition_year: 2025 },
  },
  {
    id: "source-ingest",
    label: "Source ingest",
    description: "Run the registry-driven source connector pipeline for P0 sources.",
    icon: Braces,
    kind: "source_ingest",
    params: { priority: "P0" },
  },
]

export function AdminDashboardClient() {
  const router = useRouter()
  const [overview, setOverview] = useState<AdminOverview | null>(null)
  const [sourceRegistry, setSourceRegistry] = useState<AdminSourceRegistryResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [starting, setStarting] = useState<string | null>(null)
  const [actionBusy, setActionBusy] = useState<string | null>(null)
  const [maxChapters, setMaxChapters] = useState("2")
  const [chapters, setChapters] = useState("")
  const [sessionKey, setSessionKey] = useState("2025R1")
  const [sourcePriorityFilter, setSourcePriorityFilter] = useState<(typeof PRIORITY_FILTERS)[number]>("all")
  const [connectorStatusFilter, setConnectorStatusFilter] = useState<(typeof CONNECTOR_FILTERS)[number]>("all")
  const [selectedSourceId, setSelectedSourceId] = useState("")
  const [sourceDetail, setSourceDetail] = useState<AdminSourceDetail | null>(null)
  const [sourceDetailLoading, setSourceDetailLoading] = useState(false)
  const [sourceDetailError, setSourceDetailError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const [next, sources] = await Promise.all([
        getAdminOverview(),
        listAdminSources({
          priority: sourcePriorityFilter === "all" ? undefined : sourcePriorityFilter,
          connector_status: connectorStatusFilter === "all" ? undefined : connectorStatusFilter,
        }),
      ])
      setOverview(next)
      setSourceRegistry(sources)
      setSelectedSourceId((current) => {
        if (current && sources.sources.some((source) => source.source_id === current)) return current
        return sources.sources[0]?.source_id ?? ""
      })
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Admin API unavailable")
    } finally {
      setLoading(false)
    }
  }, [connectorStatusFilter, sourcePriorityFilter])

  useEffect(() => {
    load()
    const interval = window.setInterval(load, REFRESH_MS)
    return () => window.clearInterval(interval)
  }, [load])

  useEffect(() => {
    if (!selectedSourceId) {
      setSourceDetail(null)
      setSourceDetailError(null)
      return
    }

    let cancelled = false
    let firstLoad = true
    let inFlight = false
    setSourceDetail(null)

    async function refreshSourceDetail() {
      if (inFlight) return
      inFlight = true
      if (firstLoad) setSourceDetailLoading(true)
      try {
        const detail = await getAdminSource(selectedSourceId)
        if (cancelled) return
        setSourceDetail(detail)
        setSourceDetailError(null)
      } catch (err) {
        if (cancelled) return
        setSourceDetail(null)
        setSourceDetailError(err instanceof Error ? err.message : "Source detail unavailable")
      } finally {
        if (!cancelled && firstLoad) setSourceDetailLoading(false)
        firstLoad = false
        inFlight = false
      }
    }

    refreshSourceDetail()
    const interval = window.setInterval(refreshSourceDetail, REFRESH_MS)

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [selectedSourceId])

  const activeJob = overview?.active_job ?? null
  const adminReady = Boolean(overview && !error)
  const activeMutating = Boolean(activeJob && !activeJob.is_read_only && !isTerminalJob(activeJob.status))
  const selectedSource = useMemo(
    () => sourceRegistry?.sources.find((source) => source.source_id === selectedSourceId) ?? null,
    [selectedSourceId, sourceRegistry],
  )
  const selectedSourceDetail = sourceDetail?.source.source_id === selectedSourceId ? sourceDetail : null
  const displayedSource = selectedSourceDetail?.source ?? selectedSource
  const graphProgress = useMemo(() => {
    if (!overview?.graph.jsonl_files) return 0
    return Math.min(100, Math.round((overview.graph.jsonl_files / 70) * 100))
  }, [overview])
  const apiMetricValue = overview?.health.api ?? (error ? "offline" : loading ? "checking" : "unavailable")
  const neo4jMetricValue = overview?.health.neo4j ?? (error ? "unknown" : "checking")
  const crawlerRunning = overview?.crawler.running_jobs ?? 0
  const crawlerMetricValue = crawlerRunning > 0 ? "running" : adminReady ? "idle" : loading ? "checking" : "unavailable"
  const graphRowsExact = overview?.graph.rows_are_exact !== false
  const graphMetricLabel = graphRowsExact ? "Graph rows" : "Graph files"
  const graphMetricValue = graphRowsExact ? formatNumber(overview?.graph.rows) : formatNumber(overview?.graph.jsonl_files)
  const graphMetricHint = graphRowsExact
    ? `${formatNumber(overview?.graph.jsonl_files)} JSONL files`
    : `${formatNumber(overview?.graph.bytes)} bytes, row scan deferred`

  async function startWorkflow(workflow: (typeof WORKFLOWS)[number]) {
    if (workflow.disabled) return
    if (!overview) {
      setError("Admin API is not ready yet.")
      return
    }
    setStarting(workflow.id)
    try {
      const params = { ...workflow.params }
      if (params.out_dir === "data") params.out_dir = overview.paths.data_dir
      if (params.out_dir === "data/sources") params.out_dir = `${overview.paths.data_dir.replace(/\/$/, "")}/sources`
      if (params.graph_dir === "data/graph") params.graph_dir = overview.paths.graph_dir
      if (workflow.id === "qc" && params.out_dir === "data/admin/qc") {
        params.out_dir = `${overview.paths.data_dir.replace(/\/$/, "")}/admin/qc`
      }
      if (workflow.kind === "crawl") {
        params.max_chapters = Number(maxChapters) || 0
        if (chapters.trim()) params.chapters = chapters.trim()
      }
      if (workflow.id === "source-ingest" && params.priority === "P0") {
        params.out_dir = `${overview.paths.data_dir.replace(/\/$/, "")}/sources`
        if (sessionKey.trim()) params.session_key = sessionKey.trim()
      }
      const detail = await startAdminJob(workflow.kind, params)
      router.push(`/admin/jobs/${encodeURIComponent(detail.job.job_id)}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start job")
    } finally {
      setStarting(null)
    }
  }

  async function startSourceJob(kind: "selected" | "p0" | "combine") {
    if (!overview) {
      setError("Admin API is not ready yet.")
      return
    }
    if (kind === "selected" && !selectedSourceId) {
      setError("Pick a source first.")
      return
    }
    const jobKey = `source-${kind}`
    setStarting(jobKey)
    try {
      const dataDir = overview.paths.data_dir.replace(/\/$/, "")
      const params: AdminJobParams =
        kind === "combine"
          ? { priority: "P0", out_dir: `${dataDir}/sources`, graph_dir: overview.paths.graph_dir }
          : kind === "p0"
            ? { priority: "P0", out_dir: `${dataDir}/sources`, edition_year: 2025 }
            : { source_id: selectedSourceId, out_dir: `${dataDir}/sources`, edition_year: 2025 }
      if (kind !== "combine" && sessionKey.trim()) params.session_key = sessionKey.trim()
      const detail = await startAdminJob(kind === "combine" ? "combine_graph" : "source_ingest", params)
      router.push(`/admin/jobs/${encodeURIComponent(detail.job.job_id)}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start source job")
    } finally {
      setStarting(null)
    }
  }

  async function startSourceOperation(source: AdminSourceRegistryEntry, operation: "ingest" | "combine") {
    if (!overview) {
      setError("Admin API is not ready yet.")
      return
    }
    const jobKey = `source-${operation}-${source.source_id}`
    setStarting(jobKey)
    try {
      const dataDir = overview.paths.data_dir.replace(/\/$/, "")
      const params: AdminJobParams =
        operation === "combine"
          ? { source_id: source.source_id, out_dir: `${dataDir}/sources`, graph_dir: overview.paths.graph_dir }
          : { source_id: source.source_id, out_dir: `${dataDir}/sources`, edition_year: 2025 }
      if (operation === "ingest" && sessionKey.trim()) params.session_key = sessionKey.trim()
      const detail = await startAdminJob(operation === "combine" ? "combine_graph" : "source_ingest", params)
      router.push(`/admin/jobs/${encodeURIComponent(detail.job.job_id)}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to ${operation} source`)
    } finally {
      setStarting(null)
    }
  }

  async function cancelActiveJob() {
    if (!activeJob || isTerminalJob(activeJob.status)) return
    setActionBusy("cancel-active")
    try {
      await cancelAdminJob(activeJob.job_id)
      await load()
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to cancel crawler job")
    } finally {
      setActionBusy(null)
    }
  }

  return (
    <div className="flex h-full min-w-0 flex-col overflow-hidden">
      <div className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="mb-1 font-mono text-xs uppercase tracking-wider text-muted-foreground">
              Internal operations
            </div>
            <h1 className="font-serif text-3xl tracking-tight text-foreground">Admin Dashboard</h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              Monitor source artifacts, graph outputs, crawler jobs, QC, seeding, and indexing from one control surface.
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={load} disabled={loading} className="w-fit gap-2">
            <RefreshCcw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            Refresh
          </Button>
        </div>

        {error && (
          <div className="mt-4 flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
          <MetricCard icon={Activity} label="API" value={apiMetricValue} hint={`Neo4j ${neo4jMetricValue}`} />
          <MetricCard icon={Activity} label="Crawler" value={crawlerMetricValue} hint={`${formatNumber(overview?.crawler.mutating_running_jobs)} mutating, ${formatNumber(overview?.crawler.read_only_running_jobs)} read-only`} />
          <MetricCard icon={FileSearch} label="Sources" value={formatNumber(sourceRegistry?.totals.sources ?? overview?.sources.registry_sources)} hint={`${formatNumber(sourceRegistry?.totals.p0_sources)} P0, ${formatNumber(overview?.sources.source_dirs)} local dirs`} />
          <MetricCard icon={Database} label={graphMetricLabel} value={graphMetricValue} hint={graphMetricHint} />
          <MetricCard icon={Sparkles} label="Indexing" value={overview?.indexing.vector_search_enabled ? "enabled" : "off"} hint={overview?.indexing.vector_index ?? "no vector index"} />
        </div>

        <section className="mt-5 rounded-md border border-border bg-card">
          <div className="flex flex-col gap-3 border-b border-border px-4 py-3 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <h2 className="text-sm font-semibold text-foreground">Crawler Runtime</h2>
              <p className="mt-0.5 text-xs text-muted-foreground">Backend process wrapper, active lock, and command path used by admin jobs.</p>
            </div>
            <div className="flex flex-wrap gap-2">
              {activeJob && (
                <Button asChild size="sm" variant="outline" className="gap-2">
                  <Link href={`/admin/jobs/${encodeURIComponent(activeJob.job_id)}`}>
                    <Activity className="h-3.5 w-3.5" />
                    Open active
                  </Link>
                </Button>
              )}
              <Button
                size="sm"
                variant="outline"
                className="gap-2"
                disabled={!activeJob || isTerminalJob(activeJob.status) || Boolean(actionBusy)}
                onClick={cancelActiveJob}
              >
                {actionBusy === "cancel-active" ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <Ban className="h-3.5 w-3.5" />}
                Cancel active
              </Button>
            </div>
          </div>
          <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-2">
              <MiniStat label="running" value={overview?.crawler.running_jobs ?? 0} />
              <MiniStat label="mutating" value={overview?.crawler.mutating_running_jobs ?? 0} />
              <MiniStat label="read-only" value={overview?.crawler.read_only_running_jobs ?? 0} />
              <MiniStat label="pid" value={overview?.crawler.active_pid ?? 0} />
            </div>
            <div className="grid gap-2 text-xs sm:grid-cols-2">
              <PathRow label="mode" value={overview?.crawler.control_mode ?? "unavailable"} />
              <PathRow label="binary" value={overview?.crawler.configured_bin ?? "cargo"} />
              <PathRow label="workdir" value={overview?.crawler.workdir ?? "."} />
              <PathRow label="kill" value={overview?.allow_kill ? "enabled" : "disabled"} />
              <PathRow label="last success" value={overview?.crawler.last_success_at_ms ? formatTime(overview.crawler.last_success_at_ms) : "none"} />
              <PathRow label="last terminal" value={overview?.crawler.last_terminal_status?.replace("_", " ") ?? "none"} />
              <PathRow label="command" value={overview?.crawler.command_prefix.join(" ") ?? "cargo run -p ors-crawler-v0"} />
              <PathRow label="lock" value={overview?.crawler.active_mutating_job ? "mutating job active" : "available"} />
            </div>
          </div>
        </section>

        <section className="mt-5 rounded-md border border-border bg-card">
          <div className="border-b border-border px-4 py-3">
            <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <div>
                <h2 className="text-sm font-semibold text-foreground">Source Registry</h2>
                <p className="mt-0.5 text-xs text-muted-foreground">Monitor every registry source and start connector-backed ingest or combine runs.</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button size="sm" variant="outline" className="gap-2" disabled={!adminReady || !selectedSource || Boolean(starting) || activeMutating} onClick={() => startSourceJob("selected")}>
                  {starting === "source-selected" ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <Braces className="h-3.5 w-3.5" />}
                  Ingest selected
                </Button>
                <Button size="sm" variant="outline" className="gap-2" disabled={!adminReady || Boolean(starting) || activeMutating} onClick={() => startSourceJob("p0")}>
                  {starting === "source-p0" ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <Layers className="h-3.5 w-3.5" />}
                  Ingest P0
                </Button>
                <Button size="sm" variant="outline" className="gap-2" disabled={!adminReady || Boolean(starting) || activeMutating} onClick={() => startSourceJob("combine")}>
                  {starting === "source-combine" ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <GitBranch className="h-3.5 w-3.5" />}
                  Combine P0
                </Button>
              </div>
            </div>
          </div>
          <div className="grid gap-4 p-4 lg:grid-cols-[minmax(18rem,0.8fr)_minmax(0,1.2fr)]">
            <div className="space-y-3">
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <Label className="text-xs">Priority</Label>
                  <Select value={sourcePriorityFilter} onValueChange={(value) => setSourcePriorityFilter(value as (typeof PRIORITY_FILTERS)[number])}>
                    <SelectTrigger className="mt-1 w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {PRIORITY_FILTERS.map((priority) => (
                        <SelectItem key={priority} value={priority}>
                          {priority === "all" ? "All" : priority}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Connector</Label>
                  <Select value={connectorStatusFilter} onValueChange={(value) => setConnectorStatusFilter(value as (typeof CONNECTOR_FILTERS)[number])}>
                    <SelectTrigger className="mt-1 w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {CONNECTOR_FILTERS.map((status) => (
                        <SelectItem key={status} value={status}>
                          {status === "all" ? "All" : status}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <Label className="text-xs">Source</Label>
              <Select value={selectedSourceId} onValueChange={setSelectedSourceId}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select a source" />
                </SelectTrigger>
                <SelectContent>
                  {(sourceRegistry?.sources ?? []).map((source) => (
                    <SelectItem key={source.source_id} value={source.source_id}>
                      {source.source_id}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <div className="grid grid-cols-2 gap-2">
                <MiniStat label="listed" value={sourceRegistry?.totals.sources ?? 0} />
                <MiniStat label="artifacts" value={sourceRegistry?.totals.local_artifacts ?? 0} />
              </div>
              <div>
                <Label htmlFor="session-key" className="text-xs">Legislature session</Label>
                <Input id="session-key" value={sessionKey} onChange={(event) => setSessionKey(event.target.value)} placeholder="2025R1" className="mt-1 h-8" />
              </div>
            </div>
            <SourceSummary
              source={displayedSource}
              detail={selectedSourceDetail}
              loading={sourceDetailLoading}
              error={sourceDetailError}
            />
          </div>
          <SourceRegistryTable
            sources={sourceRegistry?.sources ?? []}
            selectedSourceId={selectedSourceId}
            starting={starting}
            disabled={!adminReady || Boolean(starting) || activeMutating}
            onMonitor={setSelectedSourceId}
            onIngest={(source) => startSourceOperation(source, "ingest")}
            onCombine={(source) => startSourceOperation(source, "combine")}
          />
        </section>

        <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(24rem,0.9fr)]">
          <section className="rounded-md border border-border bg-card">
            <div className="border-b border-border px-4 py-3">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <h2 className="text-sm font-semibold text-foreground">Run Controls</h2>
                  <p className="mt-0.5 text-xs text-muted-foreground">Allowlisted crawler and indexing workflows.</p>
                </div>
                {activeJob && !isTerminalJob(activeJob.status) && <JobStatusBadge job={activeJob} />}
              </div>
            </div>
            <div className="grid gap-3 p-4 lg:grid-cols-2">
              <div className="grid gap-2 rounded-md border border-border bg-background/40 p-3 lg:col-span-2">
                <div className="grid gap-3 sm:grid-cols-[8rem_1fr]">
                  <div>
                    <Label htmlFor="max-chapters" className="text-xs">Max chapters</Label>
                    <Input id="max-chapters" value={maxChapters} onChange={(event) => setMaxChapters(event.target.value.replace(/[^0-9]/g, ""))} className="mt-1 h-8" />
                  </div>
                  <div>
                    <Label htmlFor="chapters" className="text-xs">Specific chapters</Label>
                    <Input id="chapters" value={chapters} onChange={(event) => setChapters(event.target.value)} placeholder="optional, e.g. 90,105-107" className="mt-1 h-8" />
                  </div>
                </div>
              </div>
              {WORKFLOWS.map((workflow) => (
                <WorkflowButton
                  key={workflow.id}
                  workflow={workflow}
                  disabled={Boolean(workflow.disabled || !adminReady || starting || (activeMutating && workflow.kind !== "qc"))}
                  loading={starting === workflow.id}
                  onStart={() => startWorkflow(workflow)}
                />
              ))}
            </div>
          </section>

          <section className="rounded-md border border-border bg-card">
            <div className="border-b border-border px-4 py-3">
              <h2 className="text-sm font-semibold text-foreground">Active Job</h2>
              <p className="mt-0.5 text-xs text-muted-foreground">Current operation and latest phase signal.</p>
            </div>
            {activeJob ? (
              <div className="space-y-4 p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <Link href={`/admin/jobs/${encodeURIComponent(activeJob.job_id)}`} className="font-mono text-sm text-primary hover:underline">
                      {activeJob.job_id}
                    </Link>
                    <div className="mt-1 text-xs text-muted-foreground">{formatKind(activeJob.kind)}</div>
                  </div>
                  <JobStatusBadge job={activeJob} />
                </div>
                <Progress value={activeJob.status === "running" ? 48 : activeJob.status === "cancel_requested" ? 70 : 100} />
                <div className="rounded-md bg-background/60 p-3 font-mono text-xs text-muted-foreground">
                  {activeJob.progress.phase ?? activeJob.message ?? "Waiting for crawler output..."}
                </div>
                <div className="grid grid-cols-3 gap-2 text-xs">
                  <MiniStat label="stdout" value={activeJob.progress.stdout_lines} />
                  <MiniStat label="stderr" value={activeJob.progress.stderr_lines} />
                  <MiniStat label="events" value={activeJob.progress.event_count} />
                </div>
              </div>
            ) : (
              <div className="p-6 text-sm text-muted-foreground">No active admin jobs.</div>
            )}
          </section>
        </div>

        <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_22rem]">
          <section className="rounded-md border border-border bg-card">
            <div className="border-b border-border px-4 py-3">
              <h2 className="text-sm font-semibold text-foreground">Recent Jobs</h2>
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Job</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Started</TableHead>
                  <TableHead>Message</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(overview?.recent_jobs ?? []).map((job) => (
                  <TableRow key={job.job_id}>
                    <TableCell>
                      <Link href={`/admin/jobs/${encodeURIComponent(job.job_id)}`} className="font-mono text-xs text-primary hover:underline">
                        {job.job_id}
                      </Link>
                    </TableCell>
                    <TableCell>{formatKind(job.kind)}</TableCell>
                    <TableCell><JobStatusBadge job={job} /></TableCell>
                    <TableCell>{formatTime(job.started_at_ms ?? job.created_at_ms)}</TableCell>
                    <TableCell className="max-w-md truncate text-muted-foreground">{job.message ?? job.progress.phase ?? ""}</TableCell>
                  </TableRow>
                ))}
                {!overview?.recent_jobs?.length && (
                  <TableRow>
                    <TableCell colSpan={5} className="py-8 text-center text-muted-foreground">
                      No jobs have been recorded yet.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </section>

          <section className="rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-semibold text-foreground">Storage Snapshot</h2>
            <div className="mt-4 space-y-3">
              <PathRow label="jobs" value={overview?.paths.jobs_dir ?? "data/admin/jobs"} />
              <PathRow label="data" value={overview?.paths.data_dir ?? "data"} />
              <PathRow label="graph" value={overview?.paths.graph_dir ?? "data/graph"} />
              <div className="pt-2">
                <div className="mb-1 flex items-center justify-between text-xs">
                  <span className="text-muted-foreground">Graph coverage</span>
                  <span className="font-mono">{graphProgress}%</span>
                </div>
                <Progress value={graphProgress} />
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}

function SourceSummary({
  source,
  detail,
  loading,
  error,
}: {
  source: AdminSourceRegistryEntry | null
  detail: AdminSourceDetail | null
  loading: boolean
  error: string | null
}) {
  if (!source) {
    return (
      <div className="rounded-md border border-border bg-background/40 p-4 text-sm text-muted-foreground">
        No source selected.
      </div>
    )
  }

  const stats = detail?.stats && typeof detail.stats === "object" ? detail.stats as Record<string, unknown> : null
  const qc = detail?.qc_report && typeof detail.qc_report === "object" ? detail.qc_report as Record<string, unknown> : null
  const warnings = Array.isArray(qc?.warnings) ? qc.warnings.filter((item): item is string => typeof item === "string") : []
  const errors = Array.isArray(qc?.errors) ? qc.errors.filter((item): item is string => typeof item === "string") : []
  const fetchedArtifacts = Array.isArray(stats?.artifacts) ? stats.artifacts.length : source.local.source_artifacts
  const graphRows = numberFrom(stats?.graph_rows) ?? source.local.graph_rows
  const discoveredItems = numberFrom(stats?.discovered_items)
  const graphFiles = detail?.graph_files ?? []
  const artifacts = detail?.raw_artifacts ?? []

  return (
    <div className="rounded-md border border-border bg-background/40 p-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="truncate text-sm font-semibold text-foreground">{source.name}</h3>
            <Badge variant="outline" className="font-mono text-[10px]">{source.priority}</Badge>
            <Badge variant="outline" className="font-mono text-[10px]">{source.source_type}</Badge>
          </div>
          <div className="mt-1 font-mono text-xs text-muted-foreground">{source.source_id}</div>
          <div className="mt-1 text-xs text-muted-foreground">{source.owner} · {source.jurisdiction}</div>
        </div>
        <SourceQcBadge status={source.local.qc_status} exists={source.local.source_dir_exists} />
      </div>

      <div className="mt-4 grid gap-2 sm:grid-cols-4">
        <MiniStat label="graph files" value={graphFiles.length || source.local.graph_files} />
        <MiniStat label="graph rows" value={graphRows} />
        <MiniStat label="items" value={discoveredItems ?? 0} />
        <MiniStat label="files" value={fetchedArtifacts} />
      </div>

      {loading && (
        <div className="mt-3 flex items-center gap-2 rounded-md border border-border bg-card/50 px-3 py-2 text-xs text-muted-foreground">
          <RefreshCcw className="h-3.5 w-3.5 animate-spin" />
          Loading source details
        </div>
      )}

      {error && (
        <div className="mt-3 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
          {error}
        </div>
      )}

      <div className="mt-4 grid gap-2 sm:grid-cols-4">
        <MiniStat label="bytes" value={source.local.source_bytes} />
        <MiniStat label="warnings" value={warnings.length} />
        <MiniStat label="errors" value={errors.length} />
        <MiniStat label="raw files" value={artifacts.length} />
      </div>

      <div className="mt-4 grid gap-2 text-xs sm:grid-cols-2">
        <PathRow label="nodes" value={source.graph_nodes_created.slice(0, 4).join(", ") || "none"} />
        <PathRow label="edges" value={source.graph_edges_created.slice(0, 4).join(", ") || "none"} />
        <PathRow label="status" value={source.connector_status} />
        <PathRow label="access" value={source.access} />
        <PathRow label="last run" value={source.local.last_finished_at ? formatDateTime(source.local.last_finished_at) : "not run"} />
        <PathRow label="official" value={source.official_status} />
      </div>

      {(graphFiles.length > 0 || artifacts.length > 0) && (
        <div className="mt-4 grid gap-3 text-xs lg:grid-cols-2">
          <SourceFileList
            title="Graph output"
            empty="No graph files"
            rows={graphFiles.slice(0, 4).map((file) => ({
              label: file.file,
              value: `${formatNumber(file.rows)} rows`,
            }))}
          />
          <SourceFileList
            title="Raw artifacts"
            empty="No raw artifacts"
            rows={artifacts.slice(0, 4).map((artifact) => ({
              label: artifact.file,
              value: artifact.status ?? artifact.content_type ?? `${formatNumber(artifact.bytes)} bytes`,
            }))}
          />
        </div>
      )}
    </div>
  )
}

function SourceRegistryTable({
  sources,
  selectedSourceId,
  starting,
  disabled,
  onMonitor,
  onIngest,
  onCombine,
}: {
  sources: AdminSourceRegistryEntry[]
  selectedSourceId: string
  starting: string | null
  disabled: boolean
  onMonitor: (sourceId: string) => void
  onIngest: (source: AdminSourceRegistryEntry) => void
  onCombine: (source: AdminSourceRegistryEntry) => void
}) {
  return (
    <div className="border-t border-border">
      <div className="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">Source Operations</h3>
          <p className="mt-0.5 text-xs text-muted-foreground">Per-source status, artifact counts, and job controls.</p>
        </div>
        <Badge variant="outline" className="font-mono text-[10px]">{formatNumber(sources.length)} listed</Badge>
      </div>
      <div className="max-h-[32rem] overflow-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Source</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Artifacts</TableHead>
              <TableHead>Graph</TableHead>
              <TableHead>Last run</TableHead>
              <TableHead className="text-right">Controls</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sources.map((source) => {
              const selected = source.source_id === selectedSourceId
              const ingestKey = `source-ingest-${source.source_id}`
              const combineKey = `source-combine-${source.source_id}`
              const ingesting = starting === ingestKey
              const combining = starting === combineKey
              const combineDisabled = disabled || source.local.graph_files === 0
              return (
                <TableRow key={source.source_id} className={cn(selected && "bg-primary/5")}>
                  <TableCell className="min-w-72">
                    <button
                      type="button"
                      onClick={() => onMonitor(source.source_id)}
                      className="block max-w-sm text-left hover:text-primary"
                    >
                      <span className="block truncate text-sm font-medium text-foreground">{source.name}</span>
                      <span className="mt-0.5 block truncate font-mono text-xs text-muted-foreground">{source.source_id}</span>
                    </button>
                    <div className="mt-1 truncate text-xs text-muted-foreground">{source.owner}</div>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1.5">
                      <Badge variant="outline" className="font-mono text-[10px]">{source.priority}</Badge>
                      <Badge variant="outline" className="font-mono text-[10px]">{source.connector_status}</Badge>
                      <SourceQcBadge status={source.local.qc_status} exists={source.local.source_dir_exists} />
                    </div>
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    <div>{formatNumber(source.local.source_artifacts)} files</div>
                    <div className="mt-0.5 text-muted-foreground">{formatNumber(source.local.source_bytes)} bytes</div>
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    <div>{formatNumber(source.local.graph_files)} files</div>
                    <div className="mt-0.5 text-muted-foreground">{formatNumber(source.local.graph_rows)} rows</div>
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    {source.local.last_finished_at ? formatDateTime(source.local.last_finished_at) : "not run"}
                  </TableCell>
                  <TableCell>
                    <div className="flex justify-end gap-1.5">
                      <Button size="sm" variant={selected ? "secondary" : "outline"} onClick={() => onMonitor(source.source_id)}>
                        Monitor
                      </Button>
                      <Button size="sm" variant="outline" className="gap-1.5" disabled={disabled} onClick={() => onIngest(source)}>
                        {ingesting ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <Braces className="h-3.5 w-3.5" />}
                        Ingest
                      </Button>
                      <Button size="sm" variant="outline" className="gap-1.5" disabled={combineDisabled} onClick={() => onCombine(source)}>
                        {combining ? <RefreshCcw className="h-3.5 w-3.5 animate-spin" /> : <GitBranch className="h-3.5 w-3.5" />}
                        Combine
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              )
            })}
            {sources.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                  No sources match the current filters.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}

function SourceFileList({ title, empty, rows }: { title: string; empty: string; rows: Array<{ label: string; value: string }> }) {
  return (
    <div className="rounded-md border border-border bg-card/50 p-3">
      <div className="mb-2 font-mono text-[10px] uppercase text-muted-foreground">{title}</div>
      <div className="space-y-1">
        {rows.map((row) => (
          <div key={`${row.label}:${row.value}`} className="flex items-center justify-between gap-3">
            <span className="min-w-0 truncate font-mono text-foreground">{row.label}</span>
            <span className="shrink-0 text-muted-foreground">{row.value}</span>
          </div>
        ))}
        {rows.length === 0 && <div className="text-muted-foreground">{empty}</div>}
      </div>
    </div>
  )
}

function MetricCard({ icon: Icon, label, value, hint }: { icon: React.ComponentType<{ className?: string }>; label: string; value?: string; hint?: string }) {
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</span>
      </div>
      <div className="mt-2 font-mono text-2xl text-foreground">{value ?? "0"}</div>
      {hint && <div className="mt-1 truncate text-xs text-muted-foreground">{hint}</div>}
    </div>
  )
}

function SourceQcBadge({ status, exists }: { status?: string | null; exists: boolean }) {
  const label = status ?? (exists ? "no qc" : "not run")
  const cls =
    status === "pass"
      ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-600"
      : status === "fail"
        ? "border-rose-500/30 bg-rose-500/10 text-rose-600"
        : exists
          ? "border-amber-500/30 bg-amber-500/10 text-amber-600"
          : "border-muted bg-muted/20 text-muted-foreground"
  return <Badge variant="outline" className={cn("font-mono text-[10px]", cls)}>{label}</Badge>
}

function WorkflowButton({ workflow, disabled, loading, onStart }: { workflow: (typeof WORKFLOWS)[number]; disabled: boolean; loading: boolean; onStart: () => void }) {
  const Icon = workflow.icon
  return (
    <button
      type="button"
      onClick={onStart}
      disabled={disabled}
      className="flex min-h-28 items-start gap-3 rounded-md border border-border bg-background/40 p-3 text-left transition-colors hover:bg-muted/50 disabled:cursor-not-allowed disabled:opacity-50"
    >
      <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-card">
        {loading ? <RefreshCcw className="h-4 w-4 animate-spin text-primary" /> : <Icon className="h-4 w-4 text-foreground" />}
      </span>
      <span className="min-w-0">
        <span className="flex items-center gap-2 text-sm font-medium text-foreground">
          {workflow.label}
          {!workflow.disabled && <Play className="h-3 w-3 text-muted-foreground" />}
          {workflow.disabled && <Square className="h-3 w-3 text-muted-foreground" />}
        </span>
        <span className="mt-1 block text-xs leading-relaxed text-muted-foreground">{workflow.description}</span>
      </span>
    </button>
  )
}

function JobStatusBadge({ job }: { job: AdminJob }) {
  const cls =
    job.status === "succeeded"
      ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-600"
      : job.status === "failed"
      ? "border-rose-500/30 bg-rose-500/10 text-rose-600"
      : job.status === "cancelled" || job.status === "cancel_requested"
      ? "border-amber-500/30 bg-amber-500/10 text-amber-600"
      : "border-sky-500/30 bg-sky-500/10 text-sky-600"
  return <Badge variant="outline" className={cn("font-mono text-[10px]", cls)}>{job.status.replace("_", " ")}</Badge>
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-border bg-background/40 p-2">
      <div className="font-mono text-[10px] uppercase text-muted-foreground">{label}</div>
      <div className="font-mono text-lg text-foreground">{formatNumber(value)}</div>
    </div>
  )
}

function PathRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-52 truncate font-mono text-foreground">{value}</span>
    </div>
  )
}

function formatKind(kind: AdminJobKind) {
  return kind.replaceAll("_", " ")
}

function formatTime(value?: number) {
  if (!value) return "not started"
  return new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit", second: "2-digit" }).format(new Date(Number(value)))
}

function formatNumber(value?: number) {
  return (value ?? 0).toLocaleString()
}

function formatDateTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" }).format(date)
}

function numberFrom(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}
