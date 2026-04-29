import { GraphInsightCard } from "@/lib/types"
import Link from "next/link"
import { cn } from "@/lib/utils"

export function GraphIntelligencePanel({ insights }: { insights: GraphInsightCard[] }) {
  if (!insights || insights.length === 0) return null

  return (
    <section className="mb-16">
      <h2 className="text-xl font-semibold text-zinc-100 mb-6">Graph Intelligence</h2>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {insights.map((insight, idx) => {
          const CardContent = (
            <div className={cn(
              "p-5 rounded-xl border border-zinc-800 h-full flex flex-col",
              insight.href ? "bg-zinc-900 hover:bg-zinc-800 hover:border-zinc-600 transition-colors" : "bg-zinc-900/50"
            )}>
              <h3 className="text-sm font-medium text-zinc-500 mb-1">{insight.title}</h3>
              <p className="text-xl font-semibold text-zinc-100 mb-2">{insight.value}</p>
              {insight.subtitle && (
                <p className="text-xs font-mono text-zinc-400 mt-auto pt-2 border-t border-zinc-800/50">{insight.subtitle}</p>
              )}
            </div>
          )

          if (insight.href) {
            return <Link key={idx} href={insight.href} className="block">{CardContent}</Link>
          }

          return <div key={idx}>{CardContent}</div>
        })}
      </div>
    </section>
  )
}
