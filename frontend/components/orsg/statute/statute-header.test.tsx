import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { StatutePageResponse } from "@/lib/types"
import { StatuteHeader } from "./statute-header"

const refreshMock = vi.fn()
const saveSidebarStatuteMock = vi.fn()

vi.mock("next/navigation", () => ({
  useRouter: () => ({ refresh: refreshMock }),
}))

vi.mock("@/lib/api", () => ({
  saveSidebarStatute: (...args: unknown[]) => saveSidebarStatuteMock(...args),
}))

vi.mock("@/lib/casebuilder/api", () => ({
  attachAuthority: vi.fn(),
  getMatterSummariesState: vi.fn(),
}))

function statutePage(): StatutePageResponse {
  return {
    identity: {
      canonical_id: "or:ors:3.130",
      citation: "ORS 3.130",
      title: "Powers of court",
      jurisdiction: "Oregon",
      corpus: "or:ors",
      chapter: "3",
      status: "active",
      edition: 2025,
    },
    current_version: {
      version_id: "or:ors:3.130:v2025",
      effective_date: "2025-01-01",
      end_date: null,
      is_current: true,
      text: "The court may exercise powers provided by law.",
    },
    versions: [],
    source_documents: [],
    provisions: [],
    chunks: [],
    definitions: [],
    exceptions: [],
    deadlines: [],
    penalties: [],
    inbound_citations: [],
    outbound_citations: [],
    qc: { status: "pass", passed_checks: 1, total_checks: 1, notes: [] },
    summary_counts: {
      provision_count: 0,
      citation_counts: { outbound: 0, inbound: 0 },
      semantic_counts: {
        obligations: 0,
        exceptions: 0,
        deadlines: 0,
        penalties: 0,
        definitions: 0,
      },
    },
  } as unknown as StatutePageResponse
}

describe("StatuteHeader", () => {
  it("refreshes shell data after saving a statute", async () => {
    saveSidebarStatuteMock.mockResolvedValueOnce({
      canonical_id: "or:ors:3.130",
      citation: "ORS 3.130",
      title: "Powers of court",
      chapter: "3",
      status: "active",
      edition_year: 2025,
    })

    render(<StatuteHeader data={statutePage()} />)

    fireEvent.click(screen.getByRole("button", { name: "Save" }))

    expect(await screen.findByText("Saved.")).toBeInTheDocument()
    expect(saveSidebarStatuteMock).toHaveBeenCalledWith("or:ors:3.130")
    expect(refreshMock).toHaveBeenCalledTimes(1)
  })
})
