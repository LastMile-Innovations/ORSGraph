import { Shell } from "@/components/orsg/shell"
import { SourcesClient } from "@/components/orsg/sources/sources-client"
import { sourceIndex } from "@/lib/mock-sources"

export default function SourcesPage() {
  return (
    <Shell>
      <SourcesClient sources={sourceIndex} />
    </Shell>
  )
}
