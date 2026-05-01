export type DataSource = "live" | "mock" | "demo" | "offline" | "empty" | "error"

export const DEMO_MODE = process.env.NEXT_PUBLIC_ORS_DEMO_MODE === "true"

export interface DataState<T> {
  source: DataSource
  data: T
  error?: string
}

export function dataErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Unknown error"
}

export function classifyFallbackSource(error: unknown): DataSource {
  const message = dataErrorMessage(error).toLowerCase()
  if (
    message.includes("fetch failed") ||
    message.includes("econnrefused") ||
    message.includes("enotfound") ||
    message.includes("network") ||
    message.includes("timed out") ||
    message.includes("timeout") ||
    message.includes("aborted")
  ) {
    return "offline"
  }
  return "mock"
}

export function classifyApiFailureSource(error: unknown): DataSource {
  return classifyFallbackSource(error) === "offline" ? "offline" : "error"
}

export function isFallbackSource(source?: DataSource): boolean {
  return Boolean(source && source !== "live")
}
