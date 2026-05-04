import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { WorkProductDashboard } from "@/components/casebuilder/work-product-dashboard"
import { getMatterState, getWorkProductsState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function WorkProductsPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const workProductsState = await getWorkProductsState(matter.id, { includeDocumentAst: true })

  return (
    <MatterShell
      matter={matter}
      activeSection="work-products"
      dataState={matterState.source === "live" ? workProductsState : matterState}
      counts={{ workProducts: workProductsState.data.length }}
    >
      <WorkProductDashboard matter={matter} workProducts={workProductsState.data} />
    </MatterShell>
  )
}
