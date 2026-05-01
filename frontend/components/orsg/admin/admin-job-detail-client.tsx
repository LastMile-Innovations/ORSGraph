"use client"

import Link from "next/link"
import { useCallback, useEffect, useState } from "react"
import { AlertTriangle, ArrowLeft, Ban, RefreshCcw, ShieldAlert, Terminal } from "lucide-react"
import {
  cancelAdminJob,
  getAdminJobDetail,
  isTerminalJob,
  killAdminJob,
  type AdminJob,
  type AdminJobDetail,
} from "@/lib/admin-api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

const REFRESH_MS = 2500

export function AdminJobDetailClient({ jobId }: { jobId: string }) {
  const [detail, setDetail] = useState<AdminJobDetail | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const next = await getAdminJobDetail(jobId)
      setDetail(next)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load job")
    }
  }, [jobId])

  useEffect(() => {
    load()
  }, [load])

  const jobStatus = detail?.job.status

  useEffect(() => {
    if (!jobStatus || isTerminalJob(jobStatus)) return
    const interval = window.setInterval(load, REFRESH_MS)
    return () => window.clearInterval(interval)
  }, [jobStatus, load])

  async function runAction(action: "cancel" | "kill") {
    if (!detail) return
    setBusy(true)
    try {
      const next = action === "cancel" ? await cancelAdminJob(detail.job.job_id) : await killAdminJob(detail.job.job_id)
      setDetail(next)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to ${action} job`)
    } finally {
      setBusy(false)
    }
  }

  const job = detail?.job
  const terminal = job ? isTerminalJob(job.status) : true
  const killDisabled = terminal || busy || !detail?.allow_kill

  return (
    <div className="flex h-full min-w-0 flex-col overflow-hidden">
      <div className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0">
            <Link href="/admin" className="mb-2 inline-flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground">
              <ArrowLeft className="h-3.5 w-3.5" />
              Admin
            </Link>
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="font-serif text-3xl tracking-tight text-foreground">{job?.job_id ?? jobId}</h1>
              {job && <JobStatusBadge job={job} />}
            </div>
            <p className="mt-1 text-sm text-muted-foreground">{job ? formatKind(job.kind) : "Loading job..."}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" size="sm" onClick={load} className="gap-2">
              <RefreshCcw className="h-3.5 w-3.5" />
              Refresh
            </Button>
            {job && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="outline" size="sm" disabled={terminal || busy} className="gap-2">
                    <Ban className="h-3.5 w-3.5" />
                    Cancel
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Cancel this job?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This asks the backend to stop the running process gracefully. Partial artifacts may remain on disk.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Keep running</AlertDialogCancel>
                    <AlertDialogAction onClick={() => runAction("cancel")}>Cancel job</AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
            {job && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="destructive" size="sm" disabled={killDisabled} className="gap-2">
                    <ShieldAlert className="h-3.5 w-3.5" />
                    Kill
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Force stop this job?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This sends a force-stop signal. It is only available when ORS_ADMIN_ALLOW_KILL is enabled.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Back</AlertDialogCancel>
                    <AlertDialogAction onClick={() => runAction("kill")} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                      Force stop
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
          </div>
        </div>
        {error && (
          <div className="mt-4 flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}
      </div>

      {detail && job ? (
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <div className="grid gap-4 lg:grid-cols-4">
            <InfoCard label="Kind" value={formatKind(job.kind)} />
            <InfoCard label="Started" value={formatTime(job.started_at_ms ?? job.created_at_ms)} />
            <InfoCard label="Finished" value={job.finished_at_ms ? formatTime(job.finished_at_ms) : "running"} />
            <InfoCard label="Exit" value={job.exit_code == null ? "none" : String(job.exit_code)} />
          </div>

          <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_22rem]">
            <section className="rounded-md border border-border bg-card">
              <div className="border-b border-border px-4 py-3">
                <h2 className="text-sm font-semibold text-foreground">Command</h2>
              </div>
              <div className="p-4">
                <pre className="max-h-32 overflow-auto rounded-md bg-background p-3 text-xs text-foreground">{job.command_display}</pre>
                {job.message && <p className="mt-3 text-sm text-muted-foreground">{job.message}</p>}
              </div>
            </section>

            <section className="rounded-md border border-border bg-card p-4">
              <h2 className="text-sm font-semibold text-foreground">Progress</h2>
              <div className="mt-3 rounded-md bg-background/60 p-3 font-mono text-xs text-muted-foreground">
                {job.progress.phase ?? "No phase signal yet."}
              </div>
              <div className="mt-3 grid grid-cols-3 gap-2 text-xs">
                <MiniStat label="stdout" value={job.progress.stdout_lines} />
                <MiniStat label="stderr" value={job.progress.stderr_lines} />
                <MiniStat label="events" value={job.progress.event_count} />
              </div>
            </section>
          </div>

          <div className="mt-5 rounded-md border border-border bg-card">
            <Tabs defaultValue="stderr">
              <div className="flex items-center justify-between border-b border-border px-4 py-2">
                <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                  <Terminal className="h-4 w-4" />
                  Logs
                </div>
                <TabsList className="h-8">
                  <TabsTrigger value="stderr" className="h-7 text-xs">stderr</TabsTrigger>
                  <TabsTrigger value="stdout" className="h-7 text-xs">stdout</TabsTrigger>
                  <TabsTrigger value="events" className="h-7 text-xs">events</TabsTrigger>
                </TabsList>
              </div>
              <TabsContent value="stderr" className="m-0">
                <LogBlock lines={detail.stderr_tail} empty="No stderr output yet." />
              </TabsContent>
              <TabsContent value="stdout" className="m-0">
                <LogBlock lines={detail.stdout_tail} empty="No stdout output yet." />
              </TabsContent>
              <TabsContent value="events" className="m-0">
                <LogBlock lines={detail.recent_events.map((event) => `${formatTime(event.timestamp_ms)} ${event.level}: ${event.message}`)} empty="No events yet." />
              </TabsContent>
            </Tabs>
          </div>

          <div className="mt-5 rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-semibold text-foreground">Output Paths</h2>
            <div className="mt-3 grid gap-2">
              {Object.entries(job.output_paths).map(([label, value]) => (
                <div key={label} className="flex items-center justify-between gap-3 rounded border border-border bg-background/40 px-3 py-2 text-xs">
                  <span className="text-muted-foreground">{label}</span>
                  <span className="max-w-2xl truncate font-mono text-foreground">{value}</span>
                </div>
              ))}
              {!Object.keys(job.output_paths).length && <div className="text-sm text-muted-foreground">No declared output paths.</div>}
            </div>
          </div>
        </div>
      ) : (
        <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
          Loading job details...
        </div>
      )}
    </div>
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

function InfoCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-card p-3">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className="mt-1 truncate font-mono text-sm text-foreground">{value}</div>
    </div>
  )
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-border bg-background/40 p-2">
      <div className="font-mono text-[10px] uppercase text-muted-foreground">{label}</div>
      <div className="font-mono text-lg text-foreground">{value.toLocaleString()}</div>
    </div>
  )
}

function LogBlock({ lines, empty }: { lines: string[]; empty: string }) {
  return (
    <pre className="max-h-[34rem] overflow-auto bg-background p-4 text-xs leading-relaxed text-foreground">
      {lines.length ? lines.join("\n") : empty}
    </pre>
  )
}

function formatKind(kind: string) {
  return kind.replaceAll("_", " ")
}

function formatTime(value?: number) {
  if (!value) return "not set"
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(Number(value)))
}
