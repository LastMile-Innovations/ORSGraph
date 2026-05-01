import { notFound } from "next/navigation"
import { GitGraphIcon, Layers3 } from "lucide-react"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function MatterGraphPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const modes = ["evidence graph", "claim-element graph", "timeline graph", "authority graph", "draft-support graph"]

  return (
    <MatterShell matter={matter} activeSection="graph" dataState={matterState}>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <GitGraphIcon className="h-3.5 w-3.5 text-primary" />
            case graph
          </div>
          <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Graph Viewer</h1>
          <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
            Route-ready V0.1 surface for visualizing parties, documents, facts, evidence, claims, drafts, deadlines, and ORS authority.
          </p>
        </header>

        <main className="px-6 py-6">
          <section className="rounded border border-dashed border-border bg-card p-6">
            <div className="flex items-center gap-2 text-sm font-medium text-foreground">
              <Layers3 className="h-4 w-4 text-primary" />
              Graph renderer deferred
            </div>
            <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
              The backing nodes and relationships are available through the CaseBuilder matter graph. The interactive canvas is planned after the V0 ingestion and validation loop is stable.
            </p>
            <div className="mt-4 flex flex-wrap gap-2">
              {modes.map((mode) => (
                <span key={mode} className="rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                  {mode}
                </span>
              ))}
            </div>
          </section>
        </main>
      </div>
    </MatterShell>
  )
}
