import { Shell } from "@/components/orsg/shell"
import { NewMatterClient } from "@/components/casebuilder/new-matter-client"
import { getCaseBuilderSettingsState } from "@/lib/casebuilder/server-api"

type NewMatterPageProps = Omit<PageProps<"/matters/new">, "searchParams"> & {
  searchParams: Promise<{ intent?: string }>
}

export default async function NewMatterPage({
  searchParams,
}: NewMatterPageProps) {
  const { intent } = await searchParams
  const settingsState = await getCaseBuilderSettingsState()
  return (
    <Shell hideLeftRail>
      <NewMatterClient
        initialIntent={intent === "build" ? "build" : intent === "fight" ? "fight" : "blank"}
        settings={settingsState.data?.settings ?? null}
      />
    </Shell>
  )
}
