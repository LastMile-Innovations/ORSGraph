import { renderWorkProductPage } from "../work-product-page"

export default async function WorkProductHistoryPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]/history">) {
  return renderWorkProductPage(params, "history")
}
