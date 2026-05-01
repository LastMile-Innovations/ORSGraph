import { RouteNotFoundState } from "@/components/orsg/route-state"

export default function NotFound() {
  return (
    <RouteNotFoundState
      title="Source not found"
      message="That source document is not available in the current source index."
      homeHref="/sources"
      homeLabel="Sources"
    />
  )
}
