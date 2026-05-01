import { RouteNotFoundState } from "@/components/orsg/route-state"

export default function NotFound() {
  return (
    <RouteNotFoundState
      title="Statute not found"
      message="That statute is not available in the current ORS corpus or demo fallback."
      homeHref="/statutes"
      homeLabel="Statute index"
    />
  )
}
