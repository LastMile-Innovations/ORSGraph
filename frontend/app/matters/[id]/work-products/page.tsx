import { notFound } from "next/navigation"
import { WorkProductDashboard } from "@/components/casebuilder/work-product-dashboard"
import { getMatterState, getWorkProductsState } from "@/lib/casebuilder/server-api"

export default async function WorkProductsPage({ params }: PageProps<"/matters/[id]/work-products">) {
  const { id } = await params
  const [matterState, workProductsState] = await Promise.all([
    getMatterState(id),
    getWorkProductsState(id, { includeDocumentAst: true }),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return <WorkProductDashboard matter={matter} workProducts={workProductsState.data} />
}
