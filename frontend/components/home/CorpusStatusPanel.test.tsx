import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import type { CorpusStatus } from "@/lib/types"
import { CorpusStatusPanel } from "./CorpusStatusPanel"

const corpus: CorpusStatus = {
  editionYear: 2026,
  source: "ORS fixture",
  counts: {
    sections: 12,
    versions: 14,
    provisions: 31,
    retrievalChunks: 42,
    citationMentions: 20,
    citesEdges: 18,
    semanticNodes: 9,
    sourceNotes: 0,
    amendments: 0,
    sessionLaws: 0,
    neo4jNodes: 100,
    neo4jRelationships: 200,
  },
  citations: {
    total: 20,
    resolved: 10,
    unresolved: 10,
    citesEdges: 18,
    coveragePercent: 79.44,
  },
  embeddings: {
    embedded: 30,
    totalEligible: 40,
    coveragePercent: 75,
    status: "partial",
  },
}

describe("CorpusStatusPanel", () => {
  it("renders source metadata and coverage metrics", () => {
    render(<CorpusStatusPanel corpus={corpus} />)

    expect(screen.getByText("ORS fixture / 2026")).toBeInTheDocument()
    expect(screen.getByText("79.4%")).toBeInTheDocument()
    expect(screen.getByText("10 resolved")).toBeInTheDocument()
    expect(screen.getByText("partial")).toHaveClass("text-warning")
  })

  it("surfaces warning-state graph metrics without corpus QC status", () => {
    render(<CorpusStatusPanel corpus={corpus} />)

    expect(screen.getByText("fast traversal citation edges")).toHaveClass("text-warning")
    expect(screen.queryByText("QC Status")).not.toBeInTheDocument()
    expect(screen.queryByText("WARNING")).not.toBeInTheDocument()
    expect(screen.getByText("10")).toBeInTheDocument()
  })

  it("renders healthy coverage without exposing QC state", () => {
    const healthyCorpus: CorpusStatus = {
      ...corpus,
      counts: {
        ...corpus.counts,
        citesEdges: corpus.counts.citationMentions,
      },
      citations: {
        ...corpus.citations,
        unresolved: 0,
        coveragePercent: 100,
      },
      embeddings: {
        ...corpus.embeddings,
        coveragePercent: 100,
        status: "complete",
      },
    }

    render(<CorpusStatusPanel corpus={healthyCorpus} />)

    expect(screen.getByText("complete")).toHaveClass("text-success")
    expect(screen.queryByText("PASS")).not.toBeInTheDocument()
    expect(screen.queryByText("FAIL")).not.toBeInTheDocument()
  })
})
