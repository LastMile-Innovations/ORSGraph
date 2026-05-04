type NavigationType = "push" | "replace" | "traverse"
type ClientInstrumentationCategory = "lifecycle" | "navigation" | "error"

interface ClientInstrumentationEvent {
  category: ClientInstrumentationCategory
  name: string
  timestamp: number
  detail?: Record<string, string | number | boolean | undefined>
}

declare global {
  interface Window {
    __ORSGraphClientInstrumentationInitialized?: boolean
  }
}

const CLIENT_INIT_MARK = "orsgraph-client-init"
const NAVIGATION_MARK_PREFIX = "orsgraph-nav-start"
const DEBUG_CLIENT_INSTRUMENTATION = process.env.NEXT_PUBLIC_ORSG_CLIENT_INSTRUMENTATION_DEBUG === "1"

try {
  initializeClientInstrumentation()
} catch (error) {
  reportClientInstrumentationEvent(formatClientErrorEvent("instrumentation_init_failed", error))
}

export function onRouterTransitionStart(url: string, navigationType: NavigationType) {
  try {
    const path = sanitizeInstrumentationUrl(url)
    markPerformance(`${NAVIGATION_MARK_PREFIX}-${Date.now()}`)
    reportClientInstrumentationEvent({
      category: "navigation",
      name: "router_transition_start",
      timestamp: Date.now(),
      detail: {
        path,
        navigationType,
      },
    })
  } catch (error) {
    reportClientInstrumentationEvent(formatClientErrorEvent("navigation_instrumentation_failed", error))
  }
}

export function initializeClientInstrumentation() {
  if (typeof window === "undefined") return
  if (window.__ORSGraphClientInstrumentationInitialized) return

  window.__ORSGraphClientInstrumentationInitialized = true
  markPerformance(CLIENT_INIT_MARK)
  reportClientInstrumentationEvent({
    category: "lifecycle",
    name: "client_instrumentation_initialized",
    timestamp: Date.now(),
  })

  window.addEventListener("error", handleWindowError)
  window.addEventListener("unhandledrejection", handleUnhandledRejection)
}

export function sanitizeInstrumentationUrl(url: string) {
  const base =
    typeof window !== "undefined" && window.location?.origin
      ? window.location.origin
      : "http://localhost"
  try {
    return new URL(url, base).pathname
  } catch {
    return "[invalid-url]"
  }
}

export function formatClientErrorEvent(name: string, error: unknown): ClientInstrumentationEvent {
  const normalizedError = normalizeErrorLike(error)
  return {
    category: "error",
    name,
    timestamp: Date.now(),
    detail: normalizedError,
  }
}

export function reportClientInstrumentationEvent(event: ClientInstrumentationEvent) {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("orsgraph:client-instrumentation", { detail: event }))
  }

  if (DEBUG_CLIENT_INSTRUMENTATION) {
    console.info("[ORSGraph] client instrumentation", event)
  }
}

function handleWindowError(event: ErrorEvent) {
  reportClientInstrumentationEvent(formatClientErrorEvent("window_error", event.error ?? event.message))
}

function handleUnhandledRejection(event: PromiseRejectionEvent) {
  reportClientInstrumentationEvent(formatClientErrorEvent("unhandled_rejection", event.reason))
}

function normalizeErrorLike(error: unknown): Record<string, string> {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
    }
  }

  if (typeof error === "string") {
    return {
      name: "Error",
      message: error,
    }
  }

  return {
    name: "UnknownError",
    message: "Client instrumentation captured a non-Error value.",
  }
}

function markPerformance(name: string) {
  if (typeof performance === "undefined" || typeof performance.mark !== "function") return
  performance.mark(name)
}

