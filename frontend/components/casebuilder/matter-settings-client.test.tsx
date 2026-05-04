import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CaseBuilderMatterSettingsResponse } from "@/lib/casebuilder/types"
import { MatterSettingsClient } from "./matter-settings-client"

const router = {
  refresh: vi.fn(),
}

const patchMatterConfig = vi.fn()

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

vi.mock("@/lib/casebuilder/api", () => ({
  patchMatterConfig: (...args: unknown[]) => patchMatterConfig(...args),
}))

describe("MatterSettingsClient", () => {
  beforeEach(() => {
    router.refresh.mockReset()
    patchMatterConfig.mockReset()
    patchMatterConfig.mockResolvedValue({ data: initialSettings({ default_confidentiality: null }) })
  })

  it("saves inherited matter config overrides and can clear them back to inherit", async () => {
    const user = userEvent.setup()
    render(<MatterSettingsClient initial={initialSettings({ default_confidentiality: "sealed" })} />)

    await user.click(screen.getByRole("tab", { name: /intake/i }))
    await user.selectOptions(screen.getByLabelText(/default confidentiality/i), "__inherit__")
    await user.click(screen.getByRole("button", { name: /save/i }))

    await waitFor(() => {
      expect(patchMatterConfig).toHaveBeenCalledWith(
        "matter:test",
        expect.objectContaining({
          matter: expect.objectContaining({ name: "Test Matter" }),
          settings: expect.objectContaining({ default_confidentiality: null }),
        }),
      )
      expect(router.refresh).toHaveBeenCalled()
    })
  })
})

function initialSettings(
  overrides: Partial<CaseBuilderMatterSettingsResponse["settings"]> = {},
): CaseBuilderMatterSettingsResponse {
  return {
    matter: {
      matter_id: "matter:test",
      name: "Test Matter",
      shortName: "Test Matter",
      matter_type: "civil",
      status: "intake",
      user_role: "neutral",
      jurisdiction: "Oregon",
      court: "Unassigned",
      case_number: null,
      owner_subject: "user:test",
      owner_email: "test@example.com",
      owner_name: "Test User",
      created_by_subject: "user:test",
      created_at: "2026-05-04T00:00:00Z",
      updated_at: "2026-05-04T00:00:00Z",
      document_count: 0,
      fact_count: 0,
      evidence_count: 0,
      claim_count: 0,
      draft_count: 0,
      open_task_count: 0,
      next_deadline: null,
    },
    settings: {
      settings_id: "casebuilder-matter-settings:matter:test",
      matter_id: "matter:test",
      owner_subject: "user:test",
      default_confidentiality: null,
      default_document_type: null,
      auto_index_uploads: null,
      auto_import_complaints: null,
      preserve_folder_paths: null,
      timeline_suggestions_enabled: null,
      ai_timeline_enrichment_enabled: null,
      transcript_redact_pii: null,
      transcript_speaker_labels: null,
      transcript_default_view: null,
      transcript_prompt_preset: null,
      transcript_remove_audio_tags: null,
      export_default_format: null,
      export_include_exhibits: null,
      export_include_qc_report: null,
      created_at: "2026-05-04T00:00:00Z",
      updated_at: "2026-05-04T00:00:00Z",
      ...overrides,
    },
    effective: {
      default_confidentiality: overrides.default_confidentiality ?? "private",
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
    },
  }
}
