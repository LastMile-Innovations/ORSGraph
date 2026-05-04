import { describe, expect, it } from "vitest"
import { filesToUploadCandidates, folderFromRelativePath, normalizeUploadRelativePath } from "./upload-folders"

describe("folder upload helpers", () => {
  it("preserves safe browser folder paths", () => {
    const file = new File(["rent"], "receipt.txt", { type: "text/plain" })
    Object.defineProperty(file, "webkitRelativePath", {
      value: "Receipts/April/receipt.txt",
    })

    expect(normalizeUploadRelativePath(file)).toBe("Receipts/April/receipt.txt")
    expect(filesToUploadCandidates([file])[0]).toMatchObject({
      relativePath: "Receipts/April/receipt.txt",
      folder: "Receipts",
    })
  })

  it("drops unsafe path segments client side before the backend enforces the contract", () => {
    const file = new File(["secret"], "secret.txt")

    expect(normalizeUploadRelativePath(file, "../Evidence/./secret.txt")).toBe("Evidence/secret.txt")
    expect(folderFromRelativePath("Evidence/secret.txt")).toBe("Evidence")
  })

  it("replaces colon separators before upload intent validation", () => {
    const file = new File(["shot"], "error:upload.png", { type: "image/png" })

    expect(normalizeUploadRelativePath(file, "Screenshots/May 4 15:01:39/error:upload.png")).toBe(
      "Screenshots/May 4 15_01_39/error_upload.png",
    )
    expect(filesToUploadCandidates([file])[0]).toMatchObject({
      relativePath: "error_upload.png",
      folder: "Uploads",
    })
  })
})
