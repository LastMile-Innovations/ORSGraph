import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import type { StatutePageResponse } from "@/lib/types"
import { SourceTab } from "./source-tab"
import { TextTab } from "./text-tab"

function statutePage(sourceDocuments: StatutePageResponse["source_documents"] = []): StatutePageResponse {
  return {
    identity: {
      canonical_id: "or:ors:90.300",
      citation: "ORS 90.300",
      title: "Security deposits",
      jurisdiction: "Oregon",
      corpus: "or:ors",
      chapter: "90",
      status: "active",
      edition: 2025,
    },
    current_version: {
      version_id: "or:ors:90.300@2025",
      effective_date: "2025",
      end_date: null,
      is_current: true,
      text: "A landlord may require payment of a security deposit.",
      source_documents: [],
    },
    versions: [],
    source_documents: sourceDocuments,
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

describe("statute source URL display", () => {
  it("does not render a fake source anchor when the Text tab has no source URL", () => {
    render(<TextTab data={statutePage()} />)

    expect(screen.getByText("Not available")).toBeInTheDocument()
    expect(screen.queryByRole("link", { name: /oregonlegislature/i })).not.toBeInTheDocument()
  })

  it("hides the Source tab open-original link when the source URL is blank", () => {
    render(
      <SourceTab
        data={statutePage([
          {
            source_id: "ors090",
            url: "",
            retrieved_at: "",
            raw_hash: "",
            normalized_hash: "",
            edition_year: 2025,
            parser_profile: "",
            parser_warnings: [],
          },
        ])}
      />,
    )

    expect(screen.queryByRole("link", { name: /open original/i })).not.toBeInTheDocument()
    expect(screen.getAllByText("Not available").length).toBeGreaterThan(0)
  })

  it("normalizes source anchors when a source URL is present", () => {
    render(
      <SourceTab
        data={statutePage([
          {
            source_id: "ors090",
            url: "oregonlegislature.gov/bills_laws/ors/ors090.html",
            retrieved_at: "2026-05-01",
            raw_hash: "raw",
            normalized_hash: "normalized",
            edition_year: 2025,
            parser_profile: "ors-html",
            parser_warnings: [],
          },
        ])}
      />,
    )

    expect(screen.getByRole("link", { name: /open original/i })).toHaveAttribute(
      "href",
      "https://oregonlegislature.gov/bills_laws/ors/ors090.html",
    )
  })
})
