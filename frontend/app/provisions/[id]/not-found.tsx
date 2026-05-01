import { RouteNotFoundState } from "@/components/orsg/route-state"

export default function NotFound() {
  return (
    <RouteNotFoundState
      title="Provision not found"
      message="That provision ID is not available in the current corpus or fallback data."
      homeHref="/search"
      homeLabel="Search"
    />
  )
}
