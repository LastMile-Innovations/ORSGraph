import { renderWorkProductAliasPage } from "../work-product-alias-page"

export default async function DeclarationAliasPage({ params }: PageProps<"/matters/[id]/declaration">) {
  return renderWorkProductAliasPage(params, "declaration")
}
