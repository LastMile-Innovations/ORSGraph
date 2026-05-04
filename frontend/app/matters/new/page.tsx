import { Shell } from "@/components/orsg/shell"
import { NewMatterClient } from "@/components/casebuilder/new-matter-client"

type NewMatterPageProps = Omit<PageProps<"/matters/new">, "searchParams"> & {
  searchParams: Promise<{ intent?: string }>
}

export default async function NewMatterPage({
  searchParams,
}: NewMatterPageProps) {
  const { intent } = await searchParams
  return (
    <Shell hideLeftRail>
      <NewMatterClient initialIntent={intent === "build" ? "build" : intent === "fight" ? "fight" : "blank"} />
    </Shell>
  )
}
