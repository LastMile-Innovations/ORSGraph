const BLOCKED_ELEMENTS = [
  "base",
  "embed",
  "form",
  "iframe",
  "link",
  "meta",
  "object",
  "script",
  "style",
]

const BLOCKED_ELEMENT_PATTERN = new RegExp(
  `<\\s*(${BLOCKED_ELEMENTS.join("|")})(?:\\s[^>]*)?>[\\s\\S]*?<\\s*\\/\\s*\\1\\s*>`,
  "gi",
)
const BLOCKED_VOID_ELEMENT_PATTERN = new RegExp(
  `<\\s*(?:${BLOCKED_ELEMENTS.join("|")})(?:\\s[^>]*)?\\/?>`,
  "gi",
)
const EVENT_HANDLER_ATTRIBUTE_PATTERN = /\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi
const SCRIPT_URL_ATTRIBUTE_PATTERN = /\s+(href|src|xlink:href)\s*=\s*(["'])\s*javascript:[\s\S]*?\2/gi

export function sanitizePreviewHtml(html: string) {
  return html
    .replace(BLOCKED_ELEMENT_PATTERN, "")
    .replace(BLOCKED_VOID_ELEMENT_PATTERN, "")
    .replace(EVENT_HANDLER_ATTRIBUTE_PATTERN, "")
    .replace(SCRIPT_URL_ATTRIBUTE_PATTERN, "")
}
