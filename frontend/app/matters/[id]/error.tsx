"use client"

import { RouteErrorBoundary } from "@/components/orsg/route-error-boundary"
import { casebuilderHomeHref } from "@/lib/casebuilder/routes"

export default function MatterError({
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
      title="CaseBuilder hit a page error"
      homeHref={casebuilderHomeHref()}
      homeLabel="All matters"
    />
  )
}
