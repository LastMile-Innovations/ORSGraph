import { renderWorkProductPage } from "../work-product-page"

interface PageProps {
  params: Promise<{ id: string; workProductId: string }>
}

export default async function WorkProductExportPage({ params }: PageProps) {
  return renderWorkProductPage(params, "export")
}
