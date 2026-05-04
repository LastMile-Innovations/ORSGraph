import { describe, expect, it } from "vitest"
import { sanitizePreviewHtml } from "./safe-html"

describe("sanitizePreviewHtml", () => {
  it("removes script-like elements from generated previews", () => {
    const html = sanitizePreviewHtml(`
      <article>
        <h1>Preview</h1>
        <script>alert("x")</script>
        <iframe src="https://example.com/embed"></iframe>
        <object data="/file"></object>
        <p>Safe content</p>
      </article>
    `)

    expect(html).toContain("<h1>Preview</h1>")
    expect(html).toContain("<p>Safe content</p>")
    expect(html).not.toMatch(/script|iframe|object/i)
  })

  it("removes inline handlers and javascript URLs", () => {
    const html = sanitizePreviewHtml(`
      <a href="javascript:alert('x')" onclick="alert('x')">bad link</a>
      <img src='javascript:alert("x")' onerror="alert('x')" alt="bad image">
    `)

    expect(html).toContain("bad link")
    expect(html).toContain("bad image")
    expect(html).not.toMatch(/javascript:|onclick|onerror/i)
  })
})
