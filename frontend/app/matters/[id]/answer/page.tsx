import { renderWorkProductAliasPage } from "../work-product-alias-page"

export default async function AnswerAliasPage({ params }: PageProps<"/matters/[id]/answer">) {
  return renderWorkProductAliasPage(params, "answer")
}
