import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { getFullGraph, getGraphNeighborhood } from "@/lib/api"
import { GraphViewer } from "./GraphViewer"
import type { GraphViewerResponse } from "./types"

vi.mock("@/lib/api", () => ({
  getFullGraph: vi.fn(),
  getGraphNeighborhood: vi.fn(),
  getGraphPath: vi.fn(),
}))

vi.mock("./GraphCanvas", () => ({
  GraphCanvas: ({ viewScope }: { viewScope: string }) => (
    <div data-testid="graph-canvas">canvas:{viewScope}</div>
  ),
}))

const centerNode = {
  id: "or:ors:3.130",
  label: "ORS 3.130",
  type: "LegalTextIdentity",
  labels: ["LegalTextIdentity"],
  citation: "ORS 3.130",
  title: "Powers of court",
  chapter: "3",
  status: "active",
}

const neighborResponse: GraphViewerResponse = {
  center: centerNode,
  nodes: [centerNode],
  edges: [],
  layout: { name: "force" },
  stats: {
    nodeCount: 1,
    edgeCount: 0,
    truncated: false,
    warnings: [],
  },
}

const fullResponse: GraphViewerResponse = {
  center: null,
  nodes: [
    centerNode,
    {
      id: "or:ors:3.135",
      label: "ORS 3.135",
      type: "Provision",
      labels: ["Provision"],
      citation: "ORS 3.135",
    },
  ],
  edges: [{
    id: "edge:1",
    source: "or:ors:3.130",
    target: "or:ors:3.135",
    type: "CITES",
    label: "CITES",
    kind: "legal",
  }],
  layout: { name: "force" },
  stats: {
    nodeCount: 2,
    edgeCount: 1,
    truncated: false,
    warnings: [],
  },
}

describe("GraphViewer", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
    vi.unstubAllGlobals()
  })

  it("keeps dense graph controls behind the advanced drawer", async () => {
    vi.mocked(getGraphNeighborhood).mockResolvedValue(neighborResponse)

    render(<GraphViewer initialFocus="or:ors:3.130" />)

    expect(await screen.findByText("Neighborhood")).toBeInTheDocument()
    expect(screen.queryByText("Relationship families")).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole("button", { name: /open advanced graph controls/i }))

    expect(await screen.findByText("Relationship families")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /load full graph/i })).toBeInTheDocument()
  })

  it("confirms before loading the full graph and marks the scope", async () => {
    vi.mocked(getGraphNeighborhood).mockResolvedValue(neighborResponse)
    vi.mocked(getFullGraph).mockResolvedValue(fullResponse)

    render(<GraphViewer initialFocus="or:ors:3.130" />)

    await screen.findByTestId("graph-canvas")
    await userEvent.click(screen.getByRole("button", { name: /open advanced graph controls/i }))
    await userEvent.click(screen.getByRole("button", { name: /load full graph/i }))

    expect(await screen.findByRole("alertdialog")).toHaveTextContent("Render full graph?")

    await userEvent.click(screen.getByRole("button", { name: /^render full graph$/i }))

    await waitFor(() => {
      expect(getFullGraph).toHaveBeenCalledWith({
        limit: 250,
        edgeLimit: 750,
        includeChunks: false,
        includeSimilarity: false,
      })
    })
    expect(await screen.findByText("canvas:full")).toBeInTheDocument()
    expect(screen.getAllByText("Full graph").length).toBeGreaterThan(0)
  })

  it("surfaces full graph API failures in the simplified workspace", async () => {
    vi.mocked(getGraphNeighborhood).mockResolvedValue(neighborResponse)
    vi.mocked(getFullGraph).mockRejectedValue(new Error("boom"))

    render(<GraphViewer initialFocus="or:ors:3.130" />)

    await screen.findByTestId("graph-canvas")
    await userEvent.click(screen.getByRole("button", { name: /open advanced graph controls/i }))
    await userEvent.click(screen.getByRole("button", { name: /load full graph/i }))
    await userEvent.click(await screen.findByRole("button", { name: /^render full graph$/i }))

    expect((await screen.findAllByText(/Full graph unavailable: boom/)).length).toBeGreaterThan(0)
  })
})
