import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { Matter, TimelineSuggestion } from "@/lib/casebuilder/types"
import { TimelineView } from "./timeline-view"

const router = {
  push: vi.fn(),
  refresh: vi.fn(),
  replace: vi.fn(),
}

vi.mock("next/navigation", () => ({
  usePathname: () => "/casebuilder/matters/smith-abc/timeline",
  useRouter: () => router,
  useSearchParams: () => new URLSearchParams(),
}))

const patchTimelineSuggestion = vi.fn()
const approveTimelineSuggestion = vi.fn()
const createTimelineEvent = vi.fn()
const suggestTimeline = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  approveTimelineSuggestion: (...args: unknown[]) => approveTimelineSuggestion(...args),
  createTimelineEvent: (...args: unknown[]) => createTimelineEvent(...args),
  patchTimelineSuggestion: (...args: unknown[]) => patchTimelineSuggestion(...args),
  suggestTimeline: (...args: unknown[]) => suggestTimeline(...args),
}))

const suggestion: TimelineSuggestion = {
  suggestion_id: "timeline-suggestion:april-1",
  id: "timeline-suggestion:april-1",
  matter_id: "matter:smith-abc",
  date: "2026-04-01",
  date_text: "April 1, 2026",
  date_confidence: 0.9,
  title: "Tenant reported mold",
  description: "Tenant reported mold on April 1, 2026.",
  kind: "notice",
  source_type: "document_index",
  source_document_id: "doc:mold",
  source_span_ids: ["span:doc:mold:1"],
  text_chunk_ids: ["chunk:doc:mold:1"],
  markdown_ast_node_ids: ["markdown-node:doc_mold:abc"],
  linked_fact_ids: ["fact:mold"],
  linked_claim_ids: ["claim:habitability"],
  work_product_id: "work-product:memo",
  block_id: "block:notice",
  agent_run_id: "timeline-agent-run:doc:mold",
  index_run_id: "index-run:doc:mold",
  dedupe_key: "2026-04-01:doc:mold:tenant-reported-mold",
  cluster_id: "cluster:mold-notice",
  duplicate_of_suggestion_id: null,
  agent_explanation: "The quote describes a dated tenant notice.",
  agent_confidence: 0.86,
  status: "suggested",
  warnings: ["numeric_date_format_needs_review"],
  approved_event_id: null,
  created_at: "2026-05-02T00:00:00Z",
  updated_at: "2026-05-02T00:00:00Z",
}

const matter = {
  id: "matter:smith-abc",
  matter_id: "matter:smith-abc",
  documents: [
    {
      id: "doc:mold",
      document_id: "doc:mold",
      title: "Mold notice",
      kind: "notice",
      document_type: "notice",
      party: "Tenant",
      dateUploaded: "2026-04-02",
      summary: "Notice document",
    },
  ],
  facts: [
    {
      id: "fact:mold",
      fact_id: "fact:mold",
      statement: "Tenant reported mold on April 1, 2026.",
      tags: ["notice"],
      status: "alleged",
      confidence: 0.8,
      disputed: false,
      sourceDocumentIds: ["doc:mold"],
      citations: [],
    },
  ],
  claims: [
    {
      id: "claim:habitability",
      claim_id: "claim:habitability",
      title: "Habitability",
      kind: "claim",
    },
  ],
  timeline: [],
  timeline_suggestions: [suggestion],
  timeline_agent_runs: [
    {
      agent_run_id: "timeline-agent-run:doc:mold",
      id: "timeline-agent-run:doc:mold",
      matter_id: "matter:smith-abc",
      subject_type: "document_index",
      subject_id: "doc:mold",
      agent_type: "timeline_builder",
      scope_type: "document_index",
      scope_ids: ["doc:mold"],
      input_hash: "hash",
      pipeline_version: "timeline-harness-v1",
      extractor_version: "casebuilder-timeline-deterministic-v1",
      prompt_template_id: "timeline-enrichment-v1",
      provider: "disabled",
      model: null,
      mode: "deterministic",
      provider_mode: "template",
      status: "completed",
      message: "Timeline agent generated 1 deterministic reviewable suggestions in provider-free mode.",
      produced_suggestion_ids: ["timeline-suggestion:april-1"],
      warnings: ["Provider-free timeline agent recorded deterministic suggestions; no unsupported text was inserted."],
      started_at: "2026-05-02T00:00:00Z",
      completed_at: "2026-05-02T00:00:01Z",
      duration_ms: 10,
      error_code: null,
      error_message: null,
      deterministic_candidate_count: 1,
      provider_enriched_count: 0,
      provider_rejected_count: 0,
      duplicate_candidate_count: 0,
      stored_suggestion_count: 1,
      preserved_review_count: 0,
      created_at: "2026-05-02T00:00:00Z",
    },
  ],
  deadlines: [],
  milestones: [],
} as unknown as Matter

describe("TimelineView", () => {
  beforeEach(() => {
    router.push.mockReset()
    router.refresh.mockReset()
    router.replace.mockReset()
    patchTimelineSuggestion.mockReset()
    approveTimelineSuggestion.mockReset()
    createTimelineEvent.mockReset()
    suggestTimeline.mockReset()
  })

  it("renders source-backed suggestion metadata for review", () => {
    render(<TimelineView matter={matter} />)

    const card = screen.getByText("Tenant reported mold").closest("article")
    expect(card).not.toBeNull()
    const reviewCard = within(card as HTMLElement)

    expect(reviewCard.getByText("source date: April 1, 2026")).toBeInTheDocument()
    expect(reviewCard.getAllByText("Tenant reported mold on April 1, 2026.").length).toBeGreaterThan(0)
    expect(reviewCard.getByText("numeric_date_format_needs_review")).toBeInTheDocument()
    expect(reviewCard.getByText("span")).toBeInTheDocument()
    expect(reviewCard.getByText("chunk")).toBeInTheDocument()
    expect(reviewCard.getByText("Habitability")).toBeInTheDocument()
    expect(reviewCard.getByRole("link", { name: /source/i })).toHaveAttribute(
      "href",
      "/casebuilder/matters/smith-abc/documents/doc%3Amold#span%3Adoc%3Amold%3A1",
    )
  })

  it("edits suggestions before approval and blocks dirty approval", async () => {
    const user = userEvent.setup()
    patchTimelineSuggestion.mockResolvedValue({
      data: { ...suggestion, title: "Edited mold notice" },
    })
    render(<TimelineView matter={matter} />)

    const card = screen.getByText("Tenant reported mold").closest("article") as HTMLElement
    await user.click(within(card).getByRole("button", { name: /edit/i }))
    const titleInput = within(card).getByLabelText("Title")
    await user.clear(titleInput)
    await user.type(titleInput, "Edited mold notice")

    expect(within(card).getByRole("button", { name: /approve/i })).toBeDisabled()
    await user.click(within(card).getByRole("button", { name: /save changes/i }))

    expect(patchTimelineSuggestion).toHaveBeenCalledWith(
      "matter:smith-abc",
      "timeline-suggestion:april-1",
      expect.objectContaining({ title: "Edited mold notice", source_span_ids: ["span:doc:mold:1"] }),
    )
  })
})
