"use client"

import { RouteErrorState } from "@/components/orsg/route-state"

export default function Error({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <RouteErrorState
      title="ORSGraph could not render this page"
      message={error.message}
      reset={reset}
      homeHref="/"
      homeLabel="Home"
    />
  )
}
