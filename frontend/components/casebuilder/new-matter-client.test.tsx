import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
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
const runMatterIndex = vi.fn()
const uploadBinaryFile = vi.fn()
const uploadTextFile = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  createMatter: (...args: unknown[]) => createMatter(...args),
  runMatterIndex: (...args: unknown[]) => runMatterIndex(...args),
  uploadBinaryFile: (...args: unknown[]) => uploadBinaryFile(...args),
  uploadTextFile: (...args: unknown[]) => uploadTextFile(...args),
}))

describe("NewMatterClient intake flow", () => {
  beforeEach(() => {
    router.push.mockReset()
    createMatter.mockReset()
    runMatterIndex.mockReset()
    uploadBinaryFile.mockReset()
    uploadTextFile.mockReset()

    createMatter.mockResolvedValue({ data: matter() })
    runMatterIndex.mockImplementation((_matterId: string, input: { document_ids?: string[] }) => ({
      data: {
        results: (input.document_ids ?? []).map((documentId) => ({
          document_id: documentId,
          status: "indexed",
          message: "Indexed",
        })),
      },
    }))
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
    expect(uploadTextFile).not.toHaveBeenCalled()
    expect(uploadBinaryFile).not.toHaveBeenCalled()
    expect(runMatterIndex).not.toHaveBeenCalled()
  })

  it("uploads the build-mode story, indexes it, then routes", async () => {
    const user = userEvent.setup()
    uploadTextFile.mockResolvedValue({ data: document("doc:story") })
    render(<NewMatterClient initialIntent="build" />)

    await user.type(screen.getByLabelText(/matter name/i), "Story matter")
    await user.type(screen.getByPlaceholderText(/tell us the story/i), "Tenant reported mold on April 1.")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      expect(uploadTextFile).toHaveBeenCalledWith(
        "matter:intake-test",
        expect.objectContaining({
          filename: "case-narrative.md",
          mime_type: "text/markdown",
          relative_path: "Intake/case-narrative.md",
          upload_batch_id: expect.stringMatching(/^batch:/),
          text: "Tenant reported mold on April 1.",
        }),
      )
      expect(runMatterIndex).toHaveBeenCalledWith("matter:intake-test", { document_ids: ["doc:story"] })
      expect(router.push).toHaveBeenCalledWith("/casebuilder/matters/intake-test")
    })
  })

  it("preserves folder-relative paths for uploaded files", async () => {
    const user = userEvent.setup()
    uploadBinaryFile.mockResolvedValue({ data: document("doc:receipt") })
    const { container } = render(<NewMatterClient initialIntent="blank" />)
    const file = new File(["rent"], "receipt.pdf", { type: "application/pdf" })
    Object.defineProperty(file, "webkitRelativePath", {
      value: "Receipts/April/receipt.pdf",
    })

    const input = container.querySelector<HTMLInputElement>("#file-input")
    expect(input).not.toBeNull()
    fireEvent.change(input as HTMLInputElement, { target: { files: [file] } })

    await user.type(screen.getByLabelText(/matter name/i), "Folder matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await waitFor(() => {
      expect(uploadBinaryFile).toHaveBeenCalledWith(
        "matter:intake-test",
        file,
        expect.objectContaining({
          confidentiality: "private",
          relative_path: "Receipts/April/receipt.pdf",
          upload_batch_id: expect.stringMatching(/^folder:/),
        }),
      )
      expect(runMatterIndex).not.toHaveBeenCalled()
      expect(router.push).toHaveBeenCalledWith("/casebuilder/matters/intake-test")
    })
  })

  it("keeps the user on intake when one upload fails and allows retry", async () => {
    const user = userEvent.setup()
    uploadBinaryFile
      .mockResolvedValueOnce({ data: document("doc:ok") })
      .mockResolvedValueOnce({ data: null, error: "Upload is too large" })
      .mockResolvedValueOnce({ data: document("doc:retry") })
    const { container } = render(<NewMatterClient initialIntent="blank" />)
    const files = [
      new File(["ok"], "ok.md", { type: "text/markdown" }),
      new File(["large"], "large.md", { type: "text/markdown" }),
    ]

    fireEvent.change(container.querySelector<HTMLInputElement>("#file-input") as HTMLInputElement, {
      target: { files },
    })
    await user.type(screen.getByLabelText(/matter name/i), "Partial matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await screen.findByText(/need attention/i)
    expect(router.push).not.toHaveBeenCalled()
    expect(screen.getByRole("link", { name: /continue/i })).toHaveAttribute(
      "href",
      "/casebuilder/matters/intake-test",
    )

    const failedRow = within(screen.getAllByText("large.md").at(-1)?.closest("div") as HTMLElement)
    await user.click(failedRow.getByRole("button", { name: /retry/i }))

    await waitFor(() => {
      expect(uploadBinaryFile).toHaveBeenCalledTimes(3)
      expect(runMatterIndex).toHaveBeenLastCalledWith("matter:intake-test", { document_ids: ["doc:retry"] })
      expect(screen.getAllByText("indexed").length).toBeGreaterThanOrEqual(2)
    })
  })

  it("retries indexing without uploading the already-stored document again", async () => {
    const user = userEvent.setup()
    uploadBinaryFile.mockResolvedValue({ data: document("doc:index") })
    runMatterIndex
      .mockResolvedValueOnce({ data: null, error: "Index service unavailable" })
      .mockResolvedValueOnce({
        data: {
          results: [{ document_id: "doc:index", status: "indexed", message: "Indexed" }],
        },
      })
    const { container } = render(<NewMatterClient initialIntent="blank" />)

    fireEvent.change(container.querySelector<HTMLInputElement>("#file-input") as HTMLInputElement, {
      target: { files: [new File(["text"], "index.md", { type: "text/markdown" })] },
    })
    await user.type(screen.getByLabelText(/matter name/i), "Index matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await screen.findByText(/index service unavailable/i)
    await user.click(screen.getByRole("button", { name: /retry/i }))

    await waitFor(() => {
      expect(uploadBinaryFile).toHaveBeenCalledTimes(1)
      expect(runMatterIndex).toHaveBeenCalledTimes(2)
      expect(screen.getByText("indexed")).toBeInTheDocument()
    })
  })

  it("keeps intake open when an index run omits a stored document result", async () => {
    const user = userEvent.setup()
    uploadBinaryFile.mockResolvedValue({ data: document("doc:missing-index-result") })
    runMatterIndex
      .mockResolvedValueOnce({ data: { results: [] } })
      .mockResolvedValueOnce({
        data: {
          results: [{ document_id: "doc:missing-index-result", status: "indexed", message: "Indexed" }],
        },
      })
    const { container } = render(<NewMatterClient initialIntent="blank" />)

    fireEvent.change(container.querySelector<HTMLInputElement>("#file-input") as HTMLInputElement, {
      target: { files: [new File(["text"], "unknown-index.md", { type: "text/markdown" })] },
    })
    await user.type(screen.getByLabelText(/matter name/i), "Missing result matter")
    await user.click(screen.getByRole("button", { name: /create matter/i }))

    await screen.findByText(/did not return a result/i)
    expect(router.push).not.toHaveBeenCalled()

    await user.click(screen.getByRole("button", { name: /retry/i }))

    await waitFor(() => {
      expect(uploadBinaryFile).toHaveBeenCalledTimes(1)
      expect(runMatterIndex).toHaveBeenCalledTimes(2)
      expect(screen.getByText("indexed")).toBeInTheDocument()
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

function document(documentId: string) {
  return {
    id: documentId,
    document_id: documentId,
    storage_status: "stored",
  }
}
