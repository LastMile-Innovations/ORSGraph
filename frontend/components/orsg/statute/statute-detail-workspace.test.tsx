import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { StatutePageResponse } from "@/lib/types"
import { StatuteDetailWorkspace } from "./statute-detail-workspace"

const replaceMock = vi.fn()
const refreshMock = vi.fn()
const getSemanticsMock = vi.fn()

vi.mock("next/navigation", () => ({
  usePathname: () => "/statutes/or%3Aors%3A90.320",
  useRouter: () => ({ replace: replaceMock, refresh: refreshMock }),
  useSearchParams: () => new URLSearchParams(),
}))

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api")
  return {
    ...actual,
    getChunks: vi.fn(),
    getCitations: vi.fn(),
    getHistory: vi.fn(),
    getSemantics: (...args: unknown[]) => getSemanticsMock(...args),
    saveSidebarStatute: vi.fn(),
  }
})

vi.mock("@/lib/casebuilder/api", () => ({
  attachAuthority: vi.fn(),
  getMatterSummariesState: vi.fn(),
}))

function statutePage(): StatutePageResponse {
  return {
    identity: {
      canonical_id: "or:ors:90.320",
      citation: "ORS 90.320",
      title: "Landlord to maintain premises in habitable condition",
      jurisdiction: "Oregon",
      corpus: "or:ors",
      chapter: "90",
      status: "active",
      edition: 2025,
    },
    current_version: {
      version_id: "OR:ORS:90.320@2025",
      effective_date: "2025",
      end_date: null,
      is_current: true,
      text: "A landlord shall maintain the dwelling unit.",
      source_documents: [],
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
    summary_counts: {
      provision_count: 0,
      citation_counts: { outbound: 0, inbound: 0 },
      semantic_counts: {
        obligations: 0,
        exceptions: 0,
        deadlines: 0,
        penalties: 0,
        definitions: 2,
      },
    },
  }
}

describe("StatuteDetailWorkspace", () => {
  beforeEach(() => {
    replaceMock.mockReset()
    refreshMock.mockReset()
    getSemanticsMock.mockReset()
  })

  it("shares lazy-loaded semantic data with tabs and the intelligence inspector", async () => {
    const user = userEvent.setup()
    getSemanticsMock.mockResolvedValueOnce({
      citation: "ORS 90.320",
      obligations: [],
      exceptions: [],
      deadlines: [],
      penalties: [],
      definitions: [{
        term: "Access code",
        text: "A means of unlocking access control systems.",
        source_provision: "ORS 90.320(1)(m)",
        scope: "ORS 90.320",
      }],
    })

    render(<StatuteDetailWorkspace data={statutePage()} initialTab="text" />)

    expect(screen.getByText("Definitions are expected; open or reload the tab to refresh extracted terms.")).toBeInTheDocument()

    await user.click(screen.getByRole("tab", { name: /Definitions\s*2/ }))

    expect(await screen.findByText("Access code")).toBeInTheDocument()
    expect(screen.getAllByText("A means of unlocking access control systems.").length).toBeGreaterThan(1)

    await waitFor(() => {
      expect(screen.queryByText("Definitions are expected; open or reload the tab to refresh extracted terms.")).not.toBeInTheDocument()
    })
    expect(getSemanticsMock).toHaveBeenCalledWith("or:ors:90.320")
  })

  it("reconciles expected semantic counts when lazy-loaded semantics are empty", async () => {
    const user = userEvent.setup()
    getSemanticsMock.mockResolvedValueOnce({
      citation: "ORS 90.320",
      obligations: [],
      exceptions: [],
      deadlines: [],
      penalties: [],
      definitions: [],
    })

    render(<StatuteDetailWorkspace data={statutePage()} initialTab="text" />)

    await user.click(screen.getByRole("tab", { name: /Definitions\s*2/ }))

    expect(await screen.findByText("No definitions detected for this statute.")).toBeInTheDocument()
    await waitFor(() => {
      expect(screen.getByRole("tab", { name: /Definitions\s*0/ })).toBeInTheDocument()
    })
    expect(screen.queryByText("Definitions are expected; open or reload the tab to refresh extracted terms.")).not.toBeInTheDocument()
  })
})
