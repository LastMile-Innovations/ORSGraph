import { notFound } from "next/navigation"
import { MatterExportPanel } from "@/components/casebuilder/matter-export-panel"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function ExportPage({ params }: PageProps<"/matters/[id]/export">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return <MatterExportPanel matter={matter} />
}
