export function normalizeExternalUrl(value?: string | null) {
  const trimmed = value?.trim()
  if (!trimmed) return ""
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed)) return trimmed
  return `https://${trimmed.replace(/^\/+/, "")}`
}

export function displayExternalUrl(value?: string | null) {
  const normalized = normalizeExternalUrl(value)
  return normalized || "Not available"
}
