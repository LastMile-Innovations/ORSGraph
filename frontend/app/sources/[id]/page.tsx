import { Shell } from "@/components/orsg/shell"
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
      <SourceDetailClient source={source} otherSources={sourceIndex.filter((s) => s.source_id !== source.source_id).slice(0, 6)} />
    </Shell>
  )
}
