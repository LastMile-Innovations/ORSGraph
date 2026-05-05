import { notFound } from "next/navigation"
import { WorkProductDashboard } from "@/components/casebuilder/work-product-dashboard"
import { getMatterState, getWorkProductsState } from "@/lib/casebuilder/server-api"

export default async function NewWorkProductPage({ params, searchParams }: PageProps<"/matters/[id]/work-products/new">) {
  const { id } = await params
  const query = await searchParams
  const initialProductType = Array.isArray(query.type) ? query.type[0] : query.type
  const [matterState, workProductsState] = await Promise.all([
    getMatterState(id),
    getWorkProductsState(id, { includeDocumentAst: true }),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <WorkProductDashboard
      matter={matter}
      workProducts={workProductsState.data}
      initialCreate
      initialProductType={initialProductType}
    />
  )
}
