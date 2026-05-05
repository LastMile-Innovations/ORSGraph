import { notFound } from "next/navigation"
import { MatterDashboard } from "@/components/casebuilder/matter-dashboard"
import { getMatterState } from "@/lib/casebuilder/server-api"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

export default async function MatterDashboardPage({ params }: PageProps<"/matters/[id]">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const documents = matter.documents
  const parties = matter.parties
  const facts = matter.facts
  const events = matter.timeline
  const claims = matter.claims.filter((claim) => claim.kind !== "defense")
  const defenses = matter.defenses
  const deadlines = matter.deadlines
  const tasks = matter.tasks
  const drafts = matter.drafts

  return (
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
      timelineSuggestions={matter.timeline_suggestions}
    />
  )
}
