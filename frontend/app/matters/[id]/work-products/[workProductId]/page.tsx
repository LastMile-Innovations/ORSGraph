import { renderWorkProductPage } from "./work-product-page"

export default async function WorkProductPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]">) {
  return renderWorkProductPage(params, "overview")
}
