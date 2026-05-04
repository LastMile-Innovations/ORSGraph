const INTERNAL_URL_BASE = "https://orsgraph.local"

export function toSafeInternalHref(value: string | null | undefined) {
  if (!value) return null

  const trimmed = value.trim()
  if (!trimmed.startsWith("/") || trimmed.startsWith("//") || /[\u0000-\u001F\u007F]/.test(trimmed)) {
    return null
  }

  try {
    const url = new URL(trimmed, INTERNAL_URL_BASE)
    if (url.origin !== INTERNAL_URL_BASE) return null
    return `${url.pathname}${url.search}${url.hash}`
  } catch {
    return null
  }
}

export function safeCallbackHref(value: string | null | undefined, fallback = "/onboarding") {
  return toSafeInternalHref(value) ?? fallback
}
