import { describe, expect, it } from "vitest"
import type { CaseDocument } from "./types"
import { buildDocumentTree, buildUploadPreviewRows, documentLibraryPath, filterDocumentsBySelection } from "./document-tree"

describe("CaseBuilder document tree helpers", () => {
  it("builds a nested active tree from library paths while keeping archive separate", () => {
    const docs = [
      doc("doc:1", "rent.txt", "Evidence/Receipts/rent.txt"),
      doc("doc:2", "notice.txt", "Evidence/Notices/notice.txt"),
      { ...doc("doc:3", "old.txt", "Evidence/Old/old.txt"), archived_at: "2026-05-04T00:00:00Z" },
    ]

    const tree = buildDocumentTree(docs)

    expect(tree.counts.active).toBe(2)
    expect(tree.children.map((node) => node.path)).toEqual(["Evidence"])
    expect(tree.children[0].children.map((node) => node.path)).toEqual(["Evidence/Notices", "Evidence/Receipts"])
    expect(filterDocumentsBySelection(docs, { kind: "archive" }).map((item) => item.document_id)).toEqual(["doc:3"])
  })

  it("falls back to folder plus filename for legacy flat documents", () => {
    expect(documentLibraryPath({ ...doc("doc:1", "rent.txt", null), folder: "Uploads" })).toBe("Uploads/rent.txt")
  })

  it("flags upload preview conflicts and inferred statuses", () => {
    const existing = [doc("doc:1", "rent.txt", "Evidence/rent.txt")]
    const file = new File(["rent"], "rent.txt", { type: "text/plain" })
    const image = new File(["scan"], "scan.png", { type: "image/png" })
    const rows = buildUploadPreviewRows(
      [
        { file, relativePath: "Evidence/rent.txt", folder: "Evidence" },
        { file, relativePath: "Evidence/rent.txt", folder: "Evidence" },
        { file: image, relativePath: "Photos/scan.png", folder: "Photos" },
      ],
      existing,
    )

    expect(rows[0].conflict).toBe("existing_path")
    expect(rows[1].conflict).toBe("existing_path")
    expect(rows[2]).toMatchObject({ inferredType: "photo", status: "ocr" })
  })
})

function doc(id: string, filename: string, libraryPath: string | null): CaseDocument {
  return {
    id,
    document_id: id,
    title: filename,
    filename,
    kind: "evidence",
    document_type: "evidence",
    pages: 1,
    pageCount: 1,
    bytes: 10,
    fileSize: "10 B",
    dateUploaded: "2026-05-04",
    uploaded_at: "2026-05-04T00:00:00Z",
    summary: "",
    status: "queued",
    processing_status: "queued",
    is_exhibit: false,
    facts_extracted: 0,
    citations_found: 0,
    contradictions_flagged: 0,
    entities: [],
    chunks: [],
    clauses: [],
    linkedFacts: [],
    issues: [],
    parties_mentioned: [],
    entities_mentioned: [],
    folder: "Evidence",
    storage_status: "stored",
    library_path: libraryPath,
  }
}
