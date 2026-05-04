import { renderWorkProductPage } from "../work-product-page"

export default async function WorkProductExportPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]/export">) {
  return renderWorkProductPage(params, "export")
}
