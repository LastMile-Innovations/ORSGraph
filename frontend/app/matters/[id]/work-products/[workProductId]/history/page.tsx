import { renderWorkProductPage } from "../work-product-page"

interface PageProps {
  params: Promise<{ id: string; workProductId: string }>
}

export default async function WorkProductHistoryPage({ params }: PageProps) {
  return renderWorkProductPage(params, "history")
}
