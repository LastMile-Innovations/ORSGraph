"use client"

import { RouteErrorBoundary } from "@/components/orsg/route-error-boundary"

export default function Error({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string }
  unstable_retry: () => void
}) {
  return (
    <RouteErrorBoundary
      error={error}
      unstable_retry={unstable_retry}
      title="Search could not load"
      homeHref="/search"
      homeLabel="Back to search"
    />
  )
}
