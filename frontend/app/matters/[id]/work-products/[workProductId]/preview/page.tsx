import { renderWorkProductPage } from "../work-product-page"

export default async function WorkProductPreviewPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]/preview">) {
  return renderWorkProductPage(params, "preview")
}
