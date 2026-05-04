import { Shell } from "@/components/orsg/shell"
import { WorkspaceSettingsClient } from "@/components/casebuilder/workspace-settings-client"
import { getCaseBuilderSettingsState } from "@/lib/casebuilder/server-api"
import { DataStateBanner } from "@/components/casebuilder/data-state-banner"

export default async function CaseBuilderSettingsPage() {
  const settingsState = await getCaseBuilderSettingsState()

  return (
    <Shell hideLeftRail>
      <DataStateBanner source={settingsState.source} error={settingsState.error} />
      {settingsState.data ? (
        <WorkspaceSettingsClient initial={settingsState.data} />
      ) : (
        <div className="flex flex-1 items-center justify-center p-6 text-sm text-muted-foreground">
          CaseBuilder settings are unavailable.
        </div>
      )}
    </Shell>
  )
}
