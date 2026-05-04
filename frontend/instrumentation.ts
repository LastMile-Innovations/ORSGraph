import type { Instrumentation } from "next"

type OnRequestError = Instrumentation.onRequestError
type RequestErrorRequest = Parameters<OnRequestError>[1]
type RequestErrorContext = Parameters<OnRequestError>[2] & {
  renderType?: "dynamic" | "dynamic-resume"
}

interface RequestErrorEvent {
  error: {
    name: string
    message: string
    digest?: string
  }
  request: {
    method: string
    path: string
  }
  context: {
    routerKind: RequestErrorContext["routerKind"]
    routePath: RequestErrorContext["routePath"]
    routeType: RequestErrorContext["routeType"]
    renderSource?: RequestErrorContext["renderSource"]
    revalidateReason: RequestErrorContext["revalidateReason"]
    renderType?: RequestErrorContext["renderType"]
  }
  runtime: string
}

export const onRequestError: OnRequestError = async (error, request, context) => {
  console.error("[ORSGraph] server request error", formatRequestErrorEvent(error, request, context))
}

export function formatRequestErrorEvent(
  error: unknown,
  request: RequestErrorRequest,
  context: RequestErrorContext,
): RequestErrorEvent {
  const normalizedError = normalizeError(error)

  return {
    error: normalizedError,
    request: {
      method: request.method,
      path: request.path,
    },
    context: {
      routerKind: context.routerKind,
      routePath: context.routePath,
      routeType: context.routeType,
      renderSource: context.renderSource,
      revalidateReason: context.revalidateReason,
      renderType: context.renderType,
    },
    runtime: process.env.NEXT_RUNTIME ?? "unknown",
  }
}

function normalizeError(error: unknown): RequestErrorEvent["error"] {
  if (error instanceof Error) {
    const digest = getDigest(error)
    return {
      name: error.name,
      message: error.message,
      ...(digest ? { digest } : {}),
    }
  }

  return {
    name: "NonErrorThrown",
    message: typeof error === "string" ? error : "A non-Error value was thrown.",
  }
}

function getDigest(error: Error) {
  const digest = (error as Error & { digest?: unknown }).digest
  return typeof digest === "string" && digest.length > 0 ? digest : undefined
}
