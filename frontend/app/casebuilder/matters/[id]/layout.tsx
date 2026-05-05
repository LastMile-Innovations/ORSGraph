import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { matterShellCounts } from "@/components/casebuilder/matter-shell-counts"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function CaseBuilderMatterLayout({
  children,
  params,
}: LayoutProps<"/casebuilder/matters/[id]">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} dataState={matterState} counts={matterShellCounts(matter)}>
      {children}
    </MatterShell>
  )
}
