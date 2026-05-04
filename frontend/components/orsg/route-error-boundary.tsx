"use client"

import { useEffect } from "react"
import { RouteErrorState } from "./route-state"

export interface RouteErrorBoundaryProps {
  error: Error & { digest?: string }
  unstable_retry: () => void
  title: string
  homeHref: string
  homeLabel: string
}

export function RouteErrorBoundary({
  error,
  unstable_retry,
  title,
  homeHref,
  homeLabel,
}: RouteErrorBoundaryProps) {
  useEffect(() => {
    console.error(error)
  }, [error])

  const message = error.digest
    ? `The route could not render. Error digest: ${error.digest}`
    : error.message || "The route could not render."

  return (
    <RouteErrorState
      title={title}
      message={message}
      retry={unstable_retry}
      homeHref={homeHref}
      homeLabel={homeLabel}
    />
  )
}
