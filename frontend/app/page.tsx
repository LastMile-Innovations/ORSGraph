import { getHomePageData } from "@/lib/api"
import { HomeHero } from "@/components/home/HomeHero"
import { CorpusStatusPanel } from "@/components/home/CorpusStatusPanel"
import { ActionCardGrid } from "@/components/home/ActionCardGrid"
import { GraphIntelligencePanel } from "@/components/home/GraphIntelligencePanel"
import { FeaturedStatutesGrid } from "@/components/home/FeaturedStatutesGrid"
import { SystemHealthPanel } from "@/components/home/SystemHealthPanel"
import { HomeOfflineBanner } from "@/components/home/HomeOfflineBanner"
import { Shell } from "@/components/orsg/shell"

export const dynamic = "force-dynamic"

export default async function HomePage() {
  const data = await getHomePageData()

  return (
    <Shell>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin bg-zinc-950 text-zinc-100 min-h-screen">
        <HomeHero />
        
        <div className="max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 pb-24">
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
