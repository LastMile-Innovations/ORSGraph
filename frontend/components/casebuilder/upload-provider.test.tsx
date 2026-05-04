import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { useState } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
  CaseBuilderUploadProvider,
  type EnqueueMatterUploadsOptions,
  useCaseBuilderUploads,
} from "./upload-provider"

const mocks = vi.hoisted(() => ({
  completeFileUpload: vi.fn(),
  createFileUpload: vi.fn(),
  createMatterIndexJob: vi.fn(),
  getMatterIndexJob: vi.fn(),
  importDocumentComplaint: vi.fn(),
  putSignedUploadFile: vi.fn(),
  router: {
    refresh: vi.fn(),
  },
}))

vi.mock("next/navigation", () => ({
  useRouter: () => mocks.router,
}))

vi.mock("@/lib/casebuilder/api", () => ({
  completeFileUpload: (...args: unknown[]) => mocks.completeFileUpload(...args),
  createFileUpload: (...args: unknown[]) => mocks.createFileUpload(...args),
  createMatterIndexJob: (...args: unknown[]) => mocks.createMatterIndexJob(...args),
  getMatterIndexJob: (...args: unknown[]) => mocks.getMatterIndexJob(...args),
  importDocumentComplaint: (...args: unknown[]) => mocks.importDocumentComplaint(...args),
  putSignedUploadFile: (...args: unknown[]) => mocks.putSignedUploadFile(...args),
}))

describe("CaseBuilderUploadProvider", () => {
  beforeEach(() => {
    mocks.completeFileUpload.mockReset()
    mocks.createFileUpload.mockReset()
    mocks.createMatterIndexJob.mockReset()
    mocks.getMatterIndexJob.mockReset()
    mocks.importDocumentComplaint.mockReset()
    mocks.putSignedUploadFile.mockReset()
    mocks.router.refresh.mockReset()
    window.sessionStorage.clear()

    mocks.createFileUpload.mockResolvedValue({ data: uploadIntent("upload:one", "doc:one") })
    mocks.putSignedUploadFile.mockResolvedValue({ data: { etag: "etag-one" } })
    mocks.completeFileUpload.mockResolvedValue({ data: { document_id: "doc:one" } })
    mocks.createMatterIndexJob.mockResolvedValue({ data: indexJob("running") })
    mocks.getMatterIndexJob.mockResolvedValue({ data: indexJob("succeeded") })
    mocks.importDocumentComplaint.mockResolvedValue({ data: { imported: [] } })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("keeps queued rows when the current matter page unmounts", async () => {
    render(
      <CaseBuilderUploadProvider>
        <RouteSwitcher />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start page upload/i }))

    await waitFor(() => {
      expect(screen.getByTestId("page-a-rows")).toHaveTextContent("1")
    })

    fireEvent.click(screen.getByRole("button", { name: /switch page/i }))

    expect(screen.getByTestId("page-b-rows")).toHaveTextContent("1")
  })

  it("recovers interrupted uploads as retry-needed rows after refresh", async () => {
    window.sessionStorage.setItem(
      "casebuilder.uploads.v1",
      JSON.stringify({
        batches: [persistedBatch("folder:refresh", "uploading")],
        rows: [persistedRow("folder:refresh:row:0", "folder:refresh", "uploading")],
      }),
    )

    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["pdf"], "lease.pdf", { type: "application/pdf" })} />
      </CaseBuilderUploadProvider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("failed")
      expect(screen.getByText(/upload interrupted by refresh/i)).toBeInTheDocument()
    })
  })

  it("continues polling recovered server-side index jobs after refresh", async () => {
    window.sessionStorage.setItem(
      "casebuilder.uploads.v1",
      JSON.stringify({
        batches: [{ ...persistedBatch("folder:index", "indexing"), indexJobId: "index-job:one" }],
        rows: [
          {
            ...persistedRow("folder:index:row:0", "folder:index", "indexing"),
            documentId: "doc:one",
            indexJobId: "index-job:one",
          },
        ],
      }),
    )

    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["# Facts"], "facts.md", { type: "text/markdown" })} />
      </CaseBuilderUploadProvider>,
    )

    await waitFor(() => {
      expect(mocks.getMatterIndexJob).toHaveBeenCalledWith("matter:test", "index-job:one")
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("indexed")
    })
  })

  it("uploads markdown bytes to the signed URL and starts a background index job", async () => {
    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["# Facts"], "facts.md", { type: "text/markdown" })} />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start upload/i }))

    await waitFor(() => {
      expect(mocks.createFileUpload).toHaveBeenCalledWith(
        "matter:test",
        expect.objectContaining({
          filename: "facts.md",
          relative_path: "Evidence/facts.md",
          upload_batch_id: expect.stringMatching(/^folder:/),
        }),
      )
      expect(mocks.putSignedUploadFile).toHaveBeenCalledWith(
        expect.objectContaining({ url: "https://r2.example/upload:one" }),
        expect.any(File),
        expect.objectContaining({ onProgress: expect.any(Function), signal: expect.any(AbortSignal) }),
      )
      expect(mocks.completeFileUpload).toHaveBeenCalledWith(
        "matter:test",
        "upload:one",
        expect.objectContaining({ document_id: "doc:one", etag: "etag-one" }),
      )
      expect(mocks.createMatterIndexJob).toHaveBeenCalledWith("matter:test", {
        document_ids: ["doc:one"],
        upload_batch_id: expect.stringMatching(/^folder:/),
      })
    })
  })

  it("honors matter upload settings for fallback metadata and auto-indexing", async () => {
    render(
      <CaseBuilderUploadProvider>
        <UploadHarness
          file={new File(["# Notes"], "notes.md", { type: "text/markdown" })}
          options={{
            autoIndex: false,
            importComplaints: false,
            defaultConfidentiality: "sealed",
            defaultDocumentType: "exhibit",
          }}
        />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start upload/i }))

    await waitFor(() => {
      expect(mocks.createFileUpload).toHaveBeenCalledWith(
        "matter:test",
        expect.objectContaining({
          confidentiality: "sealed",
          document_type: "exhibit",
        }),
      )
      expect(mocks.createMatterIndexJob).not.toHaveBeenCalled()
      expect(mocks.importDocumentComplaint).not.toHaveBeenCalled()
    })
  })

  it("shows upload progress and speed while bytes are moving", async () => {
    const put = deferred<{ data: { etag: string } }>()
    mocks.putSignedUploadFile.mockImplementationOnce((_intent: unknown, _file: unknown, options: { onProgress?: (progress: unknown) => void }) => {
      options.onProgress?.({ loaded: 5, total: 10, speedBps: 1024, elapsedMs: 1000 })
      return put.promise
    })
    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["1234567890"], "facts.md", { type: "text/markdown" })} />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start upload/i }))

    await waitFor(() => {
      expect(screen.getByText(/5 B \/ 10 B/i)).toBeInTheDocument()
      expect(screen.getByText(/1.0 KB\/s/i)).toBeInTheDocument()
    })

    put.resolve({ data: { etag: "etag-progress" } })
  })

  it("cancels a row before signed file bytes are sent", async () => {
    const intent = deferred<{ data: ReturnType<typeof uploadIntent> }>()
    mocks.createFileUpload.mockReturnValue(intent.promise)
    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["pdf"], "lease.pdf", { type: "application/pdf" })} />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start upload/i }))
    await waitFor(() => {
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("preparing")
    })

    fireEvent.click(screen.getByRole("button", { name: /cancel first row/i }))
    intent.resolve({ data: uploadIntent("upload:cancel", "doc:cancel") })

    await waitFor(() => {
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("canceled")
      expect(mocks.putSignedUploadFile).not.toHaveBeenCalled()
    })
  })

  it("retries a failed row without failing the whole provider queue", async () => {
    mocks.putSignedUploadFile
      .mockResolvedValueOnce({ data: null, error: "R2 unavailable" })
      .mockResolvedValueOnce({ data: { etag: "etag-two" } })

    render(
      <CaseBuilderUploadProvider>
        <UploadHarness file={new File(["pdf"], "lease.pdf", { type: "application/pdf" })} />
      </CaseBuilderUploadProvider>,
    )

    fireEvent.click(screen.getByRole("button", { name: /start upload/i }))

    await waitFor(() => {
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("failed")
    })
    fireEvent.click(screen.getByRole("button", { name: /retry first row/i }))

    await waitFor(() => {
      expect(screen.getByTestId("row-statuses")).toHaveTextContent("view_only")
      expect(mocks.putSignedUploadFile).toHaveBeenCalledTimes(2)
    })
  })
})

function RouteSwitcher() {
  const [page, setPage] = useState<"a" | "b">("a")
  return (
    <>
      {page === "a" ? <PageA /> : <PageB />}
      <button type="button" onClick={() => setPage("b")}>
        switch page
      </button>
    </>
  )
}

function PageA() {
  const { enqueueMatterUploads, rows } = useCaseBuilderUploads()
  return (
    <>
      <button
        type="button"
        onClick={() =>
          enqueueMatterUploads("matter:test", [
            { file: new File(["text"], "note.txt", { type: "text/plain" }), folder: "Evidence", relativePath: "Evidence/note.txt" },
          ])
        }
      >
        start page upload
      </button>
      <div data-testid="page-a-rows">{rows.length}</div>
    </>
  )
}

function PageB() {
  const { rows } = useCaseBuilderUploads()
  return <div data-testid="page-b-rows">{rows.length}</div>
}

function UploadHarness({ file, options }: { file: File; options?: EnqueueMatterUploadsOptions }) {
  const { cancelRow, enqueueMatterUploads, retryRow, rows } = useCaseBuilderUploads()
  return (
    <>
      <button
        type="button"
        onClick={() =>
          enqueueMatterUploads("matter:test", [{ file, folder: "Evidence", relativePath: `Evidence/${file.name}` }], options)
        }
      >
        start upload
      </button>
      <button type="button" onClick={() => rows[0] && cancelRow(rows[0].id)}>
        cancel first row
      </button>
      <button type="button" onClick={() => rows[0] && retryRow(rows[0].id)}>
        retry first row
      </button>
      <div data-testid="row-statuses">{rows.map((row) => row.status).join(",")}</div>
    </>
  )
}

function uploadIntent(uploadId: string, documentId: string) {
  return {
    upload_id: uploadId,
    document_id: documentId,
    method: "PUT",
    url: `https://r2.example/${uploadId}`,
    expires_at: "999999",
    headers: { "content-type": "application/octet-stream" },
  }
}

function indexJob(status: "running" | "succeeded") {
  return {
    index_job_id: "index-job:one",
    id: "index-job:one",
    matter_id: "matter:test",
    upload_batch_id: "batch:test",
    document_ids: ["doc:one"],
    limit: 250,
    status,
    stage: status,
    requested: 1,
    processed: status === "succeeded" ? 1 : 0,
    skipped: 0,
    failed: 0,
    produced_timeline_suggestions: 0,
    results:
      status === "succeeded"
        ? [{ document_id: "doc:one", status: "indexed", message: "Indexed markdown." }]
        : [],
    summary: null,
    warnings: [],
    error_code: null,
    error_message: null,
    retryable: true,
    created_at: "2026-05-04T00:00:00Z",
    started_at: null,
    completed_at: status === "succeeded" ? "2026-05-04T00:00:01Z" : null,
  }
}

function persistedBatch(id: string, status: "uploading" | "indexing") {
  return {
    id,
    matterId: "matter:test",
    uploadBatchId: id,
    label: "Recovered upload",
    status,
    createdAt: Date.now(),
    rowIds: [`${id}:row:0`],
    message: "Recovered",
  }
}

function persistedRow(id: string, batchId: string, status: "uploading" | "indexing") {
  return {
    id,
    batchId,
    matterId: "matter:test",
    relativePath: "Evidence/facts.md",
    folder: "Evidence",
    status,
    message: "Recovered",
    bytes: 7,
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((next) => {
    resolve = next
  })
  return { promise, resolve }
}
