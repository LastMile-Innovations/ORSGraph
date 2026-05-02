import { orsApiBaseUrl } from "./ors-api-url"

export type RuntimeState = "checking" | "connected" | "degraded" | "offline"

export interface RuntimeStatus {
  state: RuntimeState
  api: "connected" | "offline" | "unknown"
  neo4j: "connected" | "offline" | "unknown"
  version?: string
  checkedAt?: string
  message?: string
}

interface HealthResponse {
  ok?: boolean
  service?: string
  neo4j?: string
  version?: string
}

const API_BASE_URL = orsApiBaseUrl()
const HEALTH_TIMEOUT_MS = 3500

export const INITIAL_RUNTIME_STATUS: RuntimeStatus = {
  state: "checking",
  api: "unknown",
  neo4j: "unknown",
}

function normalizeNeo4jStatus(value: unknown): RuntimeStatus["neo4j"] {
  if (typeof value !== "string") return "unknown"
  const normalized = value.toLowerCase()
  if (normalized === "connected") return "connected"
  if (normalized === "offline" || normalized === "disconnected") return "offline"
  return "unknown"
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Health check failed"
}

export async function fetchRuntimeStatus(signal?: AbortSignal): Promise<RuntimeStatus> {
  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS)
  const abortFromParent = () => controller.abort()

  if (signal?.aborted) {
    controller.abort()
  } else {
    signal?.addEventListener("abort", abortFromParent, { once: true })
  }

  try {
    const response = await fetch(`${API_BASE_URL}/health`, {
      cache: "no-store",
      headers: { Accept: "application/json" },
      signal: controller.signal,
    })

    if (!response.ok) {
      throw new Error(`Health check returned ${response.status}`)
    }

    const health = (await response.json()) as HealthResponse
    const neo4j = normalizeNeo4jStatus(health.neo4j)
    const healthy = Boolean(health.ok) && neo4j === "connected"

    return {
      state: healthy ? "connected" : "degraded",
      api: "connected",
      neo4j,
      version: health.version,
      checkedAt: new Date().toISOString(),
      message: healthy ? undefined : "API reachable; graph storage needs attention.",
    }
  } catch (error) {
    if (signal?.aborted) throw error

    return {
      state: "offline",
      api: "offline",
      neo4j: "unknown",
      checkedAt: new Date().toISOString(),
      message: errorMessage(error),
    }
  } finally {
    window.clearTimeout(timeout)
    signal?.removeEventListener("abort", abortFromParent)
  }
}
