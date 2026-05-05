import { notFound } from "next/navigation"
import { Suspense } from "react"
import { TimelineView } from "@/components/casebuilder/timeline-view"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function TimelinePage({ params }: PageProps<"/matters/[id]/timeline">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <Suspense fallback={<TimelineFallback />}>
      <TimelineView matter={matter} />
    </Suspense>
  )
}

function TimelineFallback() {
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background">
      <div className="border-b border-border bg-card px-6 py-4">
        <div className="h-4 w-32 animate-pulse rounded bg-muted" />
        <div className="mt-3 h-7 w-56 animate-pulse rounded bg-muted" />
      </div>
      <div className="grid min-h-0 flex-1 gap-4 p-6 lg:grid-cols-[minmax(0,1fr)_24rem]">
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, index) => (
            <div key={index} className="h-24 animate-pulse rounded border border-border bg-card" />
          ))}
        </div>
        <div className="hidden h-full animate-pulse rounded border border-border bg-card lg:block" />
      </div>
    </div>
  )
}
