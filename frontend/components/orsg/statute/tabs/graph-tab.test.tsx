import { render, screen, waitFor } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { StatutePageResponse } from "@/lib/types"
import { GraphTab } from "./graph-tab"

const getGraphNeighborhoodMock = vi.fn()

vi.mock("@/lib/api", () => ({
  getGraphNeighborhood: (...args: unknown[]) => getGraphNeighborhoodMock(...args),
}))

function statutePage(): StatutePageResponse {
  return {
    identity: {
      canonical_id: "or:ors:90.100",
      citation: "ORS 90.100",
      title: "Definitions",
      jurisdiction: "Oregon",
      corpus: "or:ors",
      chapter: "90",
      status: "active",
      edition: 2025,
    },
    current_version: {
      version_id: "OR:ORS:90.100@2025",
      effective_date: "2025",
      end_date: null,
      is_current: true,
      text: "Definitions.",
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
  }
}

describe("GraphTab", () => {
  it("shows stable loading UI and preserves statute focus in the graph explorer link", async () => {
    getGraphNeighborhoodMock.mockResolvedValueOnce({
      center: { id: "or:ors:90.100" },
      nodes: [
        { id: "or:ors:90.100", label: "ORS 90.100", type: "LegalTextIdentity", status: "active" },
        { id: "or:ors:105.168", label: "ORS 105.168", type: "LegalTextIdentity", status: "active" },
      ],
      edges: [{ id: "edge:1", source: "or:ors:105.168", target: "or:ors:90.100", type: "CITES" }],
    })

    render(<GraphTab data={statutePage()} />)

    expect(screen.getByText("Loading graph neighborhood")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "open in graph explorer →" })).toHaveAttribute(
      "href",
      "/graph?focus=or%3Aors%3A90.100",
    )

    await waitFor(() => {
      expect(screen.queryByText("Loading graph neighborhood")).not.toBeInTheDocument()
    })
    expect(screen.getByText("ORS 90.100")).toBeInTheDocument()
    expect(getGraphNeighborhoodMock).toHaveBeenCalledWith({
      citation: "ORS 90.100",
      depth: 1,
      limit: 80,
      mode: "legal",
    })
  })
})
