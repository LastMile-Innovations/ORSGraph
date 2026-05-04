import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"
import type { CaseGraphResponse, Matter } from "@/lib/casebuilder/types"
import { MatterGraphView } from "./matter-graph-view"

const matter = {
  id: "matter:smith-abc",
  matter_id: "matter:smith-abc",
  name: "Smith v. ABC",
} as Matter

const graph: CaseGraphResponse = {
  matter_id: "matter:smith-abc",
  generated_at: "2026-05-02T00:00:00Z",
  modes: ["overview", "markdown", "markdown_ast", "entities", "provenance"],
  warnings: [],
  nodes: [
    {
      id: "matter:smith-abc",
      kind: "matter",
      label: "Smith v. ABC",
      metadata: {},
    },
    {
      id: "doc:markdown",
      kind: "document",
      label: "Facts.md",
      subtitle: "facts.md",
      metadata: {},
    },
    {
      id: "markdown-ast:doc_markdown:a",
      kind: "markdown_ast_document",
      label: "Markdown AST",
      subtitle: "pulldown-cmark-0.13",
      metadata: {},
    },
    {
      id: "markdown-node:doc_markdown:heading",
      kind: "markdown_ast_node",
      label: "# Facts",
      subtitle: "Facts",
      metadata: {},
    },
    {
      id: "chunk:markdown:1",
      kind: "text_chunk",
      label: "Debra Paynter paid rent.",
      metadata: {},
    },
    {
      id: "entity-mention:markdown:1",
      kind: "entity_mention",
      label: "Debra Paynter",
      metadata: {},
    },
    {
      id: "case-entity:matter_smith_abc:debra",
      kind: "case_entity",
      label: "Debra Paynter",
      metadata: {},
    },
    {
      id: "party:other",
      kind: "party",
      label: "Other Party",
      metadata: {},
    },
  ],
  edges: [
    {
      id: "doc:markdown->markdown-ast:doc_markdown:a:has_markdown_ast",
      source: "doc:markdown",
      target: "markdown-ast:doc_markdown:a",
      kind: "has_markdown_ast",
      label: "has markdown ast",
      metadata: {},
    },
    {
      id: "markdown-ast:doc_markdown:a->markdown-node:doc_markdown:heading:contains",
      source: "markdown-ast:doc_markdown:a",
      target: "markdown-node:doc_markdown:heading",
      kind: "contains_ast_node",
      label: "contains",
      metadata: {},
    },
    {
      id: "entity-mention:markdown:1->case-entity:matter_smith_abc:debra:resolves",
      source: "entity-mention:markdown:1",
      target: "case-entity:matter_smith_abc:debra",
      kind: "resolves_to",
      label: "resolves to",
      metadata: {},
    },
  ],
}

describe("MatterGraphView Markdown modes", () => {
  it("filters Markdown AST and entity graph modes from backend modes", async () => {
    const user = userEvent.setup()
    render(<MatterGraphView matter={matter} graph={graph} />)

    await user.click(screen.getByRole("button", { name: /markdown ast/i }))

    expect(screen.getAllByText("Markdown AST").length).toBeGreaterThan(0)
    expect(screen.getAllByText("# Facts").length).toBeGreaterThan(0)
    expect(screen.queryByText("Other Party")).not.toBeInTheDocument()

    await user.click(screen.getByRole("button", { name: /entities/i }))

    expect(screen.getAllByText("Debra Paynter").length).toBeGreaterThan(0)
    expect(screen.queryAllByText("# Facts")).toHaveLength(0)
  })
})
