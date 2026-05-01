import { notFound } from "next/navigation"
import { Archive, FileText, PackageCheck } from "lucide-react"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function ExportPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="export" dataState={matterState}>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <PackageCheck className="h-3.5 w-3.5 text-primary" />
            filing packager
          </div>
          <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Exports</h1>
          <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
            Route-ready export surface for DOCX, PDF, exhibit lists, and filing packets. Court e-filing is intentionally out of scope for V0.
          </p>
        </header>

        <main className="grid grid-cols-1 gap-3 px-6 py-6 md:grid-cols-3">
          <ExportTile icon={FileText} title="DOCX" body="Deferred until the draft model and document rendering pipeline are finalized." />
          <ExportTile icon={Archive} title="PDF" body="Deferred with DOCX export; no court-ready formatting is implied in V0." />
          <ExportTile icon={PackageCheck} title="Filing packet" body="Deferred until exhibit packet generation and final QC are implemented." />
        </main>
      </div>
    </MatterShell>
  )
}

function ExportTile({ icon: Icon, title, body }: { icon: typeof FileText; title: string; body: string }) {
  return (
    <section className="rounded border border-dashed border-border bg-card p-4">
      <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
        <Icon className="h-4 w-4 text-primary" />
        {title}
      </div>
      <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{body}</p>
    </section>
  )
}
