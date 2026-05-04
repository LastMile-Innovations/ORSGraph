"use client"

import { useEffect } from "react"
import { RouteErrorState } from "./route-state"

export interface RouteErrorBoundaryProps {
  error: Error & { digest?: string }
  unstable_retry?: () => void
  reset?: () => void
  title: string
  homeHref: string
  homeLabel: string
}

export function RouteErrorBoundary({
  error,
  unstable_retry,
  reset,
  title,
  homeHref,
  homeLabel,
}: RouteErrorBoundaryProps) {
  useEffect(() => {
    console.error(error)
  }, [error])

  const retry = unstable_retry ?? reset
  const message = error.digest
    ? `${error.message || "The route could not render."} Error digest: ${error.digest}`
    : error.message || "The route could not render."

  return (
    <RouteErrorState
      title={title}
      message={message}
      reset={retry}
      homeHref={homeHref}
      homeLabel={homeLabel}
    />
  )
}
