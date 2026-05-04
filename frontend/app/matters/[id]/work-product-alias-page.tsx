import { notFound, redirect } from "next/navigation"
import { getMatterState, getWorkProductsState } from "@/lib/casebuilder/server-api"
import { matterWorkProductHref, newWorkProductHref } from "@/lib/casebuilder/routes"

interface WorkProductAliasParams {
  id: string
}

export async function renderWorkProductAliasPage(
  params: Promise<WorkProductAliasParams>,
  productType: string,
) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const workProductsState = await getWorkProductsState(matter.id)
  const existing = workProductsState.data.find((product) => product.product_type === productType)

  if (existing) {
    redirect(matterWorkProductHref(matter.id, existing.id, "editor"))
  }

  redirect(newWorkProductHref(matter.id, productType))
}
