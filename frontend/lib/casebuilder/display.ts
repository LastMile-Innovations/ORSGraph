export function formatOptionalDate(value?: string | null, fallback = "not set") {
  const trimmed = value?.trim()
  if (!trimmed) return fallback

  const date = new Date(trimmed)
  if (!Number.isFinite(date.getTime())) return fallback

  return date.toLocaleDateString()
}
