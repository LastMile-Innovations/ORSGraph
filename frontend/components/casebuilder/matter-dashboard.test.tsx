import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { CaseDefense, MatterSummary } from "@/lib/casebuilder/types"
import { MatterDashboard } from "./matter-dashboard"

vi.mock("./delete-matter-button", () => ({
  DeleteMatterButton: () => <button type="button">Danger zone</button>,
}))

const matter = {
  matter_id: "matter:defense-only",
  name: "Defense Only",
  case_number: null,
  court: "Circuit Court",
  created_at: "2026-05-01T00:00:00Z",
  updated_at: "2026-05-02T00:00:00Z",
  jurisdiction: "Oregon",
  user_role: "defendant",
  status: "active",
  matter_type: "eviction_defense",
} as unknown as MatterSummary

const defense = {
  defense_id: "defense:habitability",
  matter_id: matter.matter_id,
  name: "Habitability defense",
  basis: "The premises were not habitable.",
  status: "draft",
  applies_to_claim_ids: [],
  required_facts: [],
  fact_ids: [],
  evidence_ids: [],
  authorities: [],
  viability: "medium",
} as unknown as CaseDefense

describe("MatterDashboard setup recommendations", () => {
  it("counts defenses as legal theories before recommending setup work", () => {
    render(
      <MatterDashboard
        matter={matter}
        parties={[]}
        documents={[]}
        facts={[]}
        events={[]}
        claims={[]}
        defenses={[defense]}
        deadlines={[]}
        tasks={[]}
        drafts={[]}
        timelineSuggestions={[]}
      />,
    )

    expect(screen.queryByText("Create claims or defenses")).not.toBeInTheDocument()
    expect(screen.getByText("Link authorities")).toBeInTheDocument()
    expect(screen.getByText("Legal theories exist, but none have source-backed authorities attached.")).toBeInTheDocument()
  })
})
