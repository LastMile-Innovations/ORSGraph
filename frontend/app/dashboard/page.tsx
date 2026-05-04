import { getCachedHomePageState } from "@/lib/authority-server-cache"
import Link from "next/link"
import { HomeHero } from "@/components/home/HomeHero"
import { CorpusStatusPanel } from "@/components/home/CorpusStatusPanel"
import { ActionCardGrid } from "@/components/home/ActionCardGrid"
import { GraphIntelligencePanel } from "@/components/home/GraphIntelligencePanel"
import { FeaturedStatutesGrid } from "@/components/home/FeaturedStatutesGrid"
import { SystemHealthPanel } from "@/components/home/SystemHealthPanel"
import { HomeOfflineBanner } from "@/components/home/HomeOfflineBanner"
import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { getMatterSummariesState } from "@/lib/casebuilder/server-api"
import { newMatterHref } from "@/lib/casebuilder/routes"
import { ArrowRight, Briefcase } from "lucide-react"

export default async function DashboardPage() {
  const [state, matterState] = await Promise.all([getCachedHomePageState(), getMatterSummariesState()])
  const data = state.data

  return (
    <Shell hideLeftRail>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto scrollbar-thin bg-background text-foreground">
        <HomeHero corpus={data.corpus} health={data.health} build={data.build} dataSource={state.source} />

        <div className="mx-auto w-full max-w-7xl px-4 pb-20 sm:px-6 lg:px-8">
          <DataStateBanner source={state.source} error={state.error} label="Home data" className="mb-4 rounded-md border" />
          {data.health.api !== "connected" && <HomeOfflineBanner />}

          {matterState.source === "live" && matterState.data.length === 0 && (
            <section className="mb-6 flex flex-col gap-4 rounded-md border border-primary/30 bg-primary/10 p-5 sm:flex-row sm:items-center sm:justify-between">
              <div className="flex items-start gap-3">
                <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
                  <Briefcase className="h-5 w-5" />
                </span>
                <div>
                  <h2 className="text-base font-semibold tracking-normal">Create your first matter</h2>
                  <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                    Start with the complaint you received, the filing you need to build, or the evidence you need to organize.
                  </p>
                </div>
              </div>
              <Link
                href={newMatterHref()}
                className="inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90"
              >
                Start matter
                <ArrowRight className="h-4 w-4" />
              </Link>
            </section>
          )}

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
