import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CaseBuilderUserSettings } from "@/lib/casebuilder/types"
import { NewMatterClient } from "./new-matter-client"

const router = {
  push: vi.fn(),
}

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

vi.mock("@/lib/conversion-events", () => ({
  trackConversionEvent: vi.fn(),
}))

const createMatter = vi.fn()
const enqueueMatterIntake = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  createMatter: (...args: unknown[]) => createMatter(...args),
}))

vi.mock("./upload-provider", () => ({
  useCaseBuilderUploads: () => ({
    enqueueMatterIntake,
  }),
}))

describe("NewMatterClient intake flow", () => {
  beforeEach(() => {
    router.push.mockReset()
    createMatter.mockReset()
    enqueueMatterIntake.mockReset()

    createMatter.mockResolvedValue({ data: matter() })
    enqueueMatterIntake.mockImplementation((_matterId: string, candidates: unknown[], options?: { storyText?: string }) =>
      candidates.length > 0 || options?.storyText?.trim() ? "batch:intake" : null,
    )
  })

  it("creates a blank matter and routes directly to the dashboard", async () => {
    const user = userEvent.setup()
    render(<NewMatterClient initialIntent="blank" />)

    await user.type(screen.getByLabelText(/matter name/i), "  Intake Test  ")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      expect(createMatter).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "Intake Test",
          matter_type: "civil",
          user_role: "neutral",
        }),
      )
      expect(router.push).toHaveBeenCalledWith("/casebuilder/matters/intake-test")
    })
    expect(enqueueMatterIntake).toHaveBeenCalledWith("matter:intake-test", [], expect.objectContaining({
      label: "Matter intake",
      storyText: undefined,
    }))
  })

  it("enqueues the build-mode story and routes immediately", async () => {
    const user = userEvent.setup()
    render(<NewMatterClient initialIntent="build" />)

    await user.type(screen.getByLabelText(/matter name/i), "Story matter")
    await user.type(screen.getByPlaceholderText(/tell us the story/i), "Tenant reported mold on April 1.")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      expect(enqueueMatterIntake).toHaveBeenCalledWith("matter:intake-test", [], expect.objectContaining({
        label: "Matter intake",
        storyText: "Tenant reported mold on April 1.",
      }))
      expect(router.push).toHaveBeenCalledWith("/casebuilder/matters/intake-test")
    })
  })

  it("prefills creation and intake defaults from workspace settings", async () => {
    const user = userEvent.setup()
    render(
      <NewMatterClient
        initialIntent="blank"
        settings={workspaceSettings({
          default_matter_type: "landlord_tenant",
          default_user_role: "defendant",
          default_jurisdiction: "Washington",
          default_court: "Clark County Superior Court",
          default_confidentiality: "sealed",
          default_document_type: "exhibit",
          auto_index_uploads: false,
          auto_import_complaints: false,
        })}
      />,
    )

    await user.type(screen.getByLabelText(/matter name/i), "Settings matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      expect(createMatter).toHaveBeenCalledWith(
        expect.objectContaining({
          matter_type: "landlord_tenant",
          user_role: "defendant",
          jurisdiction: "Washington",
          court: "Clark County Superior Court",
          settings: undefined,
        }),
      )
      expect(enqueueMatterIntake).toHaveBeenCalledWith(
        "matter:intake-test",
        [],
        expect.objectContaining({
          autoIndex: false,
          importComplaints: false,
          defaultConfidentiality: "sealed",
          defaultDocumentType: "exhibit",
        }),
      )
    })
  })

  it("preserves folder-relative paths for uploaded files", async () => {
    const user = userEvent.setup()
    const { container } = render(<NewMatterClient initialIntent="blank" />)
    const file = new File(["rent"], "receipt.pdf", { type: "application/pdf" })
    Object.defineProperty(file, "webkitRelativePath", {
      configurable: true,
      value: "Receipts/April/receipt.pdf",
    })

    const input = container.querySelector<HTMLInputElement>("#file-input")
    expect(input).not.toBeNull()
    fireEvent.change(input as HTMLInputElement, { target: { files: [file] } })

    await user.type(screen.getByLabelText(/matter name/i), "Folder matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      const [matterId, candidates, options] = enqueueMatterIntake.mock.calls[0] as [
        string,
        { file: File; relativePath: string; folder: string }[],
        { label: string; storyText?: string },
      ]
      expect(matterId).toBe("matter:intake-test")
      expect(candidates).toHaveLength(1)
      expect(candidates[0]).toMatchObject({
        file,
        folder: "Receipts",
        relativePath: "Receipts/April/receipt.pdf",
      })
      expect(options).toEqual(expect.objectContaining({ label: "Matter intake", storyText: undefined }))
      expect(router.push).toHaveBeenCalledWith("/casebuilder/matters/intake-test")
    })
  })
})

function matter() {
  return {
    id: "matter:intake-test",
    matter_id: "matter:intake-test",
    name: "Intake Test",
    title: "Intake Test",
    matter_type: "civil",
    status: "intake",
    user_role: "neutral",
    jurisdiction: "Oregon",
    court: "Unassigned",
    case_number: null,
    created_at: "2026-05-03T00:00:00Z",
    updated_at: "2026-05-03T00:00:00Z",
    document_count: 0,
    fact_count: 0,
    evidence_count: 0,
    claim_count: 0,
    draft_count: 0,
    open_task_count: 0,
    next_deadline: null,
    documents: [],
    parties: [],
    facts: [],
    timeline: [],
    timeline_suggestions: [],
    timeline_agent_runs: [],
    claims: [],
    evidence: [],
    defenses: [],
    deadlines: [],
    tasks: [],
    drafts: [],
    work_products: [],
    fact_check_findings: [],
    citation_check_findings: [],
  }
}

function workspaceSettings(overrides: Partial<CaseBuilderUserSettings> = {}): CaseBuilderUserSettings {
  return {
    settings_id: "casebuilder-user-settings:user:test",
    subject: "user:test",
    workspace_label: null,
    display_name: null,
    default_matter_type: "civil",
    default_user_role: "neutral",
    default_jurisdiction: "Oregon",
    default_court: "Unassigned",
    default_confidentiality: "private",
    default_document_type: "other",
    auto_index_uploads: true,
    auto_import_complaints: true,
    preserve_folder_paths: true,
    timeline_suggestions_enabled: true,
    ai_timeline_enrichment_enabled: true,
    transcript_redact_pii: true,
    transcript_speaker_labels: true,
    transcript_default_view: "redacted",
    transcript_prompt_preset: "unclear",
    transcript_remove_audio_tags: true,
    export_default_format: "pdf",
    export_include_exhibits: true,
    export_include_qc_report: true,
    created_at: "2026-05-04T00:00:00Z",
    updated_at: "2026-05-04T00:00:00Z",
    ...overrides,
  }
}
