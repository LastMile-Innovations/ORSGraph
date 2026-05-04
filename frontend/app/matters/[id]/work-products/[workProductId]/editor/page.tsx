import { renderWorkProductPage } from "../work-product-page"

export default async function WorkProductEditorPage({ params }: PageProps<"/matters/[id]/work-products/[workProductId]/editor">) {
  return renderWorkProductPage(params, "editor")
}
