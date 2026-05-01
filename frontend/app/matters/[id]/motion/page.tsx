import { renderWorkProductAliasPage } from "../work-product-alias-page"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function MotionAliasPage({ params }: PageProps) {
  return renderWorkProductAliasPage(params, "motion")
}
