import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { SourceDetailClient } from "@/components/orsg/sources/source-detail-client"
import { getSourceById, sourceIndex } from "@/lib/mock-sources"
import { notFound } from "next/navigation"

export default async function SourceDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const source = getSourceById(decoded)
  if (!source) notFound()
  return (
    <Shell>
      <DataStateBanner source="demo" label="Source detail demo" />
      <SourceDetailClient source={source} otherSources={sourceIndex.filter((s) => s.source_id !== source.source_id).slice(0, 6)} />
    </Shell>
  )
}
