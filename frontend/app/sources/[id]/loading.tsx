import { RouteLoadingState } from "@/components/orsg/route-state"

export default function Loading() {
  return <RouteLoadingState title="Loading source" message="Fetching source metadata and parser diagnostics." />
}
