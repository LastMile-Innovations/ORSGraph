import { RouteNotFoundState } from "@/components/orsg/route-state"

export default function NotFound() {
  return (
    <RouteNotFoundState
      title="Page not found"
      message="That route is not part of the current ORSGraph frontend."
      homeHref="/search"
      homeLabel="Search ORS"
    />
  )
}
