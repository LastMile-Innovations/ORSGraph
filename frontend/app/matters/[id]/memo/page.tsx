import { renderWorkProductAliasPage } from "../work-product-alias-page"

export default async function MemoAliasPage({ params }: PageProps<"/matters/[id]/memo">) {
  return renderWorkProductAliasPage(params, "legal_memo")
}
