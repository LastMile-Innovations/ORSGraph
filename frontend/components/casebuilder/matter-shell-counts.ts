import type { Matter } from "@/lib/casebuilder/types"

export function matterShellCounts(matter: Matter) {
  const claims = matter.claims.filter((claim) => claim.kind !== "defense")

  return {
    documents: matter.documents.length,
    facts: matter.facts.length,
    events: matter.timeline.length,
    evidence: matter.evidence.length,
    claims: claims.length,
    defenses: matter.defenses.length,
    drafts: matter.drafts.length,
    deadlines: matter.deadlines.filter((deadline) => deadline.status === "open").length,
    tasks: matter.tasks.filter((task) => task.status !== "done").length,
    workProducts: matter.work_products.length,
  }
}
