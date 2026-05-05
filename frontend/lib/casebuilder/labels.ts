export function cleanMatterLabel(value?: string | null, fallback = "current matter") {
  const trimmed = value?.trim()
  if (!trimmed) return fallback

  const withoutMarkdownLinks = trimmed.replace(/\[([^\]]+)]\((?:https?:\/\/)?[^)]+\)/gi, "$1")
  const withoutUrls = withoutMarkdownLinks
    .replace(/https?:\/\/\S+/gi, "")
    .replace(/\b(?:frontend-[^\s)]+|localhost:\d+|127\.0\.0\.1:\d+)[^\s)]*/gi, "")
  const cleaned = withoutUrls
    .replace(/^matter\s+/i, "")
    .replace(/[()[\]]/g, " ")
    .replace(/\s+/g, " ")
    .trim()

  return cleaned || fallback
}
