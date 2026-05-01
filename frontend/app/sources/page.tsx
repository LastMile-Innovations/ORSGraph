import { Shell } from "@/components/orsg/shell"
import { SourcesClient } from "@/components/orsg/sources/sources-client"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { sourceIndex } from "@/lib/mock-sources"

export default function SourcesPage() {
  return (
    <Shell>
      <DataStateBanner source="demo" label="Source index demo" />
      <SourcesClient sources={sourceIndex} />
    </Shell>
  )
}
