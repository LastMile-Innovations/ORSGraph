import { getHomePageState } from "@/lib/api"
import { HomeHero } from "@/components/home/HomeHero"
import { CorpusStatusPanel } from "@/components/home/CorpusStatusPanel"
import { ActionCardGrid } from "@/components/home/ActionCardGrid"
import { GraphIntelligencePanel } from "@/components/home/GraphIntelligencePanel"
import { FeaturedStatutesGrid } from "@/components/home/FeaturedStatutesGrid"
import { SystemHealthPanel } from "@/components/home/SystemHealthPanel"
import { HomeOfflineBanner } from "@/components/home/HomeOfflineBanner"
import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"

export const dynamic = "force-dynamic"

export default async function DashboardPage() {
  const state = await getHomePageState()
  const data = state.data

  return (
    <Shell hideLeftRail>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto scrollbar-thin bg-background text-foreground">
        <HomeHero corpus={data.corpus} health={data.health} build={data.build} dataSource={state.source} />

        <div className="mx-auto w-full max-w-7xl px-4 pb-20 sm:px-6 lg:px-8">
          <DataStateBanner source={state.source} error={state.error} label="Home data" className="mb-4 rounded-md border" />
          {data.health.api !== "connected" && <HomeOfflineBanner />}

          <ActionCardGrid actions={data.actions} />

          <CorpusStatusPanel corpus={data.corpus} />

          <GraphIntelligencePanel insights={data.insights} />

          <FeaturedStatutesGrid statutes={data.featuredStatutes} />

          <SystemHealthPanel health={data.health} corpus={data.corpus} />
        </div>
      </div>
    </Shell>
  )
}
