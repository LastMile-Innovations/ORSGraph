import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CaseBuilderUserSettingsResponse } from "@/lib/casebuilder/types"
import { WorkspaceSettingsClient } from "./workspace-settings-client"

const router = {
  refresh: vi.fn(),
}

const patchCaseBuilderSettings = vi.fn()

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

vi.mock("@/lib/casebuilder/api", () => ({
  patchCaseBuilderSettings: (...args: unknown[]) => patchCaseBuilderSettings(...args),
}))

describe("WorkspaceSettingsClient", () => {
  beforeEach(() => {
    router.refresh.mockReset()
    patchCaseBuilderSettings.mockReset()
    patchCaseBuilderSettings.mockResolvedValue({ data: initialSettings() })
  })

  it("tracks dirty state and saves workspace defaults", async () => {
    const user = userEvent.setup()
    render(<WorkspaceSettingsClient initial={initialSettings()} />)

    const save = screen.getByRole("button", { name: /save/i })
    expect(save).toBeDisabled()

    await user.type(screen.getByLabelText(/workspace label/i), "Trial desk")
    expect(save).toBeEnabled()
    await user.click(save)

    await waitFor(() => {
      expect(patchCaseBuilderSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          workspace_label: "Trial desk",
          default_jurisdiction: "Oregon",
          default_confidentiality: "private",
        }),
      )
      expect(router.refresh).toHaveBeenCalled()
    })
  })
})

function initialSettings(overrides: Partial<CaseBuilderUserSettingsResponse["settings"]> = {}): CaseBuilderUserSettingsResponse {
  return {
    principal: {
      subject: "user:test",
      email: "test@example.com",
      name: "Test User",
      roles: ["casebuilder"],
      is_service: false,
    },
    settings: {
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
    },
  }
}
