import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { MatterExportPanel } from "@/components/casebuilder/matter-export-panel"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function ExportPage({ params }: PageProps<"/matters/[id]/export">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="export" dataState={matterState}>
      <MatterExportPanel matter={matter} />
    </MatterShell>
  )
}
