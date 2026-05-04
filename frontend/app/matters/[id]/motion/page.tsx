import { renderWorkProductAliasPage } from "../work-product-alias-page"

export default async function MotionAliasPage({ params }: PageProps<"/matters/[id]/motion">) {
  return renderWorkProductAliasPage(params, "motion")
}
