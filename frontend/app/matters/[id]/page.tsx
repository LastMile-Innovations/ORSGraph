import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { MatterDashboard } from "@/components/casebuilder/matter-dashboard"
import {
  getClaimsByMatter,
  getDeadlinesByMatter,
  getDefensesByMatter,
  getDocumentsByMatter,
  getDraftsByMatter,
  getEventsByMatter,
  getEvidenceByMatter,
  getFactsByMatter,
  getMatterById,
  getPartiesByMatter,
  getTasksByMatter,
} from "@/lib/casebuilder/mock-matters"

export default async function MatterDashboardPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()

  const documents = getDocumentsByMatter(id)
  const parties = getPartiesByMatter(id)
  const facts = getFactsByMatter(id)
  const events = getEventsByMatter(id)
  const evidence = getEvidenceByMatter(id)
  const claims = getClaimsByMatter(id)
  const defenses = getDefensesByMatter(id)
  const deadlines = getDeadlinesByMatter(id)
  const tasks = getTasksByMatter(id)
  const drafts = getDraftsByMatter(id)

  return (
    <MatterShell
      matter={matter}
      counts={{
        documents: documents.length,
        facts: facts.length,
        events: events.length,
        evidence: evidence.length,
        claims: claims.length,
        defenses: defenses.length,
        drafts: drafts.length,
        deadlines: deadlines.filter((d) => d.status === "open").length,
        tasks: tasks.filter((t) => t.status !== "done").length,
      }}
    >
      <MatterDashboard
        matter={matter}
        parties={parties}
        documents={documents}
        facts={facts}
        events={events}
        claims={claims}
        defenses={defenses}
        deadlines={deadlines}
        tasks={tasks}
        drafts={drafts}
      />
    </MatterShell>
  )
}
