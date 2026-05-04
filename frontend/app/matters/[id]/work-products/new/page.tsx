import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { WorkProductDashboard } from "@/components/casebuilder/work-product-dashboard"
import { getMatterState, getWorkProductsState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
  searchParams?: Promise<{ type?: string | string[] }>
}

export default async function NewWorkProductPage({ params, searchParams }: PageProps) {
  const { id } = await params
  const query = searchParams ? await searchParams : {}
  const initialProductType = Array.isArray(query.type) ? query.type[0] : query.type
  const [matterState, workProductsState] = await Promise.all([
    getMatterState(id),
    getWorkProductsState(id, { includeDocumentAst: true }),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell
      matter={matter}
      activeSection="work-products"
      dataState={matterState.source === "live" ? workProductsState : matterState}
      counts={{ workProducts: workProductsState.data.length }}
    >
      <WorkProductDashboard
        matter={matter}
        workProducts={workProductsState.data}
        initialCreate
        initialProductType={initialProductType}
      />
    </MatterShell>
  )
}
