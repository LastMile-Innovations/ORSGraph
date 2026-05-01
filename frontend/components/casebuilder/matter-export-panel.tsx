"use client"

import { useState } from "react"
import { Archive, Download, FileText, PackageCheck } from "lucide-react"
import type { ExportPackage, Matter } from "@/lib/casebuilder/types"
import { exportMatterPackage } from "@/lib/casebuilder/api"
import { cn } from "@/lib/utils"

const EXPORTS = [
  { format: "docx", title: "DOCX", body: "Prepare a review-needed document package from current WorkProducts.", icon: FileText },
  { format: "pdf", title: "PDF", body: "Prepare a review-needed PDF package status with open-QC warnings.", icon: Archive },
  { format: "filing_packet", title: "Filing packet", body: "Bundle matter WorkProducts, exhibit/QC state, and package warnings.", icon: PackageCheck },
] as const

export function MatterExportPanel({ matter }: { matter: Matter }) {
  const [pending, setPending] = useState<string | null>(null)
  const [packages, setPackages] = useState<Record<string, ExportPackage>>({})
  const [message, setMessage] = useState<string | null>(null)

  async function run(format: string) {
    setPending(format)
    setMessage(null)
    const result = await exportMatterPackage(matter.id, format)
    setPending(null)
    if (!result.data?.result) {
      setMessage(result.error || result.data?.message || "Export package could not be prepared.")
      return
    }
    setPackages((current) => ({ ...current, [format]: result.data!.result! }))
    setMessage(result.data.message)
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <PackageCheck className="h-3.5 w-3.5 text-primary" />
          filing packager
        </div>
        <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Exports</h1>
        <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
          Prepare review-needed matter packages from current WorkProducts and QC state. Court e-filing remains out of scope.
        </p>
        {message && <div className="mt-3 rounded border border-border bg-background px-3 py-2 text-xs text-muted-foreground">{message}</div>}
      </header>

      <main className="grid grid-cols-1 gap-3 px-6 py-6 md:grid-cols-3">
        {EXPORTS.map(({ format, title, body, icon: Icon }) => {
          const prepared = packages[format]
          return (
            <section key={format} className="rounded border border-border bg-card p-4">
              <div className="flex items-start justify-between gap-3">
                <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                  <Icon className="h-4 w-4 text-primary" />
                  {title}
                </div>
                {prepared && <Status status={prepared.status} />}
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{body}</p>
              <button
                type="button"
                onClick={() => run(format)}
                disabled={pending !== null}
                className="mt-4 inline-flex items-center gap-1.5 rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:opacity-60"
              >
                <Download className="h-3.5 w-3.5" />
                {pending === format ? "preparing" : "prepare"}
              </button>
              {prepared && (
                <div className="mt-4 space-y-2 rounded border border-border bg-background p-3 text-xs">
                  <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {prepared.artifact_count} artifact(s) · {prepared.profile}
                  </div>
                  {prepared.warnings.map((warning) => (
                    <p key={warning} className="leading-relaxed text-warning">{warning}</p>
                  ))}
                </div>
              )}
            </section>
          )
        })}
      </main>
    </div>
  )
}

function Status({ status }: { status: string }) {
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider",
        status.includes("blocked") ? "bg-warning/15 text-warning" : "bg-primary/10 text-primary",
      )}
    >
      {status.replace(/_/g, " ")}
    </span>
  )
}
