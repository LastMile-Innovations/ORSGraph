import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { WorkProductWorkbench } from "@/components/casebuilder/work-product-workbench"
import { getMatterState, getWorkProductState } from "@/lib/casebuilder/server-api"
import type { WorkProductWorkspaceSection } from "@/lib/casebuilder/routes"

interface WorkProductPageParams {
  id: string
  workProductId: string
}

export async function renderWorkProductPage(
  params: Promise<WorkProductPageParams>,
  mode: WorkProductWorkspaceSection | "overview",
) {
  const { id, workProductId } = await params
  const [matterState, workProductState] = await Promise.all([
    getMatterState(id),
    getWorkProductState(id, workProductId, { includeDocumentAst: true }),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  const workProduct = workProductState.data
  if (!workProduct) notFound()

  return (
    <MatterShell
      matter={matter}
      activeSection="work-products"
      dataState={matterState.source === "live" ? workProductState : matterState}
      counts={{ workProducts: matter.work_products.length }}
    >
      <WorkProductWorkbench matter={matter} workProduct={workProduct} mode={mode} />
    </MatterShell>
  )
}
