import { GraphInsightCard } from "@/lib/types"
import Link from "next/link"
import { cn } from "@/lib/utils"

export function GraphIntelligencePanel({ insights }: { insights: GraphInsightCard[] }) {
  if (!insights || insights.length === 0) return null

  return (
    <section className="mb-12">
      <h2 className="mb-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">graph intelligence</h2>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {insights.map((insight, idx) => {
          const CardContent = (
            <div className={cn(
              "flex h-full min-h-32 flex-col rounded-md border p-4 transition-colors",
              insight.href ? "border-border bg-card hover:border-primary/40" : "border-border bg-card/70",
              insight.state === "warning" && "border-warning/30",
              insight.state === "error" && "border-destructive/30",
            )}>
              <h3 className="mb-1 text-sm font-medium text-muted-foreground">{insight.title}</h3>
              <p className="mb-2 text-xl font-semibold text-foreground">{insight.value}</p>
              {insight.subtitle && (
                <p className="mt-auto border-t border-border pt-2 font-mono text-[11px] text-muted-foreground">{insight.subtitle}</p>
              )}
            </div>
          )

          if (insight.href) {
            return <Link key={idx} href={insight.href} className="block focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60">{CardContent}</Link>
          }

          return <div key={idx}>{CardContent}</div>
        })}
      </div>
    </section>
  )
}
