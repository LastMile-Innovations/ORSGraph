import { renderWorkProductAliasPage } from "../work-product-alias-page"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function DeclarationAliasPage({ params }: PageProps) {
  return renderWorkProductAliasPage(params, "declaration")
}
