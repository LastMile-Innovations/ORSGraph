import { renderWorkProductAliasPage } from "../work-product-alias-page"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function MemoAliasPage({ params }: PageProps) {
  return renderWorkProductAliasPage(params, "legal_memo")
}
