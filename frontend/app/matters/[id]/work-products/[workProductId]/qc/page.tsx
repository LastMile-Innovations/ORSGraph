import { renderWorkProductPage } from "../work-product-page"

export default async function WorkProductQcPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]/qc">) {
  return renderWorkProductPage(params, "qc")
}
