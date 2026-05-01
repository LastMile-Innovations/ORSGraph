"use client"

import { RouteErrorState } from "@/components/orsg/route-state"

export default function Error({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <RouteErrorState
      title="Provision could not load"
      message={error.message}
      reset={reset}
      homeHref="/search"
      homeLabel="Search"
    />
  )
}
