import { Shell } from "@/components/orsg/shell"
import { NewMatterClient } from "@/components/casebuilder/new-matter-client"

export default async function NewMatterPage({
  searchParams,
}: {
  searchParams: Promise<{ intent?: string }>
}) {
  const { intent } = await searchParams
  return (
    <Shell hideLeftRail>
      <NewMatterClient initialIntent={intent === "build" ? "build" : intent === "fight" ? "fight" : "blank"} />
    </Shell>
  )
}
