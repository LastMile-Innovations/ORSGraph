import { notFound } from "next/navigation"
import { Shell } from "@/components/orsg/shell"
import { StatuteHeader } from "@/components/orsg/statute/statute-header"
import { StatuteTabs } from "@/components/orsg/statute/statute-tabs"
import { StatuteRightInspector } from "@/components/orsg/statute/statute-right-inspector"
import { getStatutePageData } from "@/lib/api"

export default async function StatutePage({
  params,
}: {
  params: Promise<{ id: string }>
}) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const data = await getStatutePageData(decoded)
  if (!data) notFound()

  return (
    <Shell rightPanel={<StatuteRightInspector data={data} />}>
      <div className="flex flex-1 flex-col overflow-hidden">
        <StatuteHeader data={data} />
        <StatuteTabs data={data} />
      </div>
    </Shell>
  )
}
