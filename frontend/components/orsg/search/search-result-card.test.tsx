import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { SearchResultCard } from "./search-result-card"
import type { SearchResult } from "@/lib/types"

const baseResult: SearchResult = {
  id: "or:ors:90.320",
  kind: "provision",
  citation: "ORS 90.320",
  title: "Landlord to maintain premises",
  snippet: "A landlord shall at all times during the tenancy maintain the dwelling unit.",
  score: 4.25,
  href: "/statutes/or%3Aors%3A90.320",
}

describe("SearchResultCard", () => {
  it("skips null score channels instead of crashing", () => {
    render(
      <SearchResultCard
        result={{
          ...baseResult,
          vector_score: null,
          fulltext_score: null,
          graph_score: null,
          rerank_score: null,
          score_breakdown: {
            exact: null,
            keyword: null,
            vector: null,
            graph: null,
            expansion: null,
            rerank: null,
          },
        }}
      />,
    )

    expect(screen.getByRole("link", { name: /ors 90\.320/i })).toBeInTheDocument()
    expect(screen.getByText("relevance")).toBeInTheDocument()
    expect(screen.getByText("4.25")).toBeInTheDocument()
    expect(screen.queryByText("vector")).not.toBeInTheDocument()
  })

  it("ignores unsupported status values from the API", () => {
    render(
      <SearchResultCard
        result={{
          ...baseResult,
          status: "unknown",
        }}
      />,
    )

    expect(screen.queryByText("unknown")).not.toBeInTheDocument()
  })
})
