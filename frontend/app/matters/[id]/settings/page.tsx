import { notFound } from "next/navigation"
import { MatterSettingsClient } from "@/components/casebuilder/matter-settings-client"
import { getMatterSettingsState, getMatterState } from "@/lib/casebuilder/server-api"

export default async function MatterSettingsPage({ params }: PageProps<"/matters/[id]/settings">) {
  const { id } = await params
  const [matterState, settingsState] = await Promise.all([
    getMatterState(id),
    getMatterSettingsState(id),
  ])
  const matter = matterState.data
  const settings = settingsState.data
  if (!matter || !settings) notFound()

  return <MatterSettingsClient initial={settings} />
}
