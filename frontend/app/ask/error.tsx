"use client"

import { RouteErrorState } from "@/components/orsg/route-state"

export default function Error({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <RouteErrorState
      title="Ask could not load"
      message={error.message}
      reset={reset}
      homeHref="/ask"
      homeLabel="Back to Ask"
    />
  )
}
