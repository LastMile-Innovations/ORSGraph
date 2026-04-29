import { HomeAction } from "@/lib/types"
import { ActionCard } from "./ActionCard"

export function ActionCardGrid({ actions }: { actions: HomeAction[] }) {
  return (
    <section className="mb-16">
      <h2 className="text-xl font-semibold text-zinc-100 mb-6">Explore the Graph</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {actions.map(action => (
          <ActionCard key={action.title} action={action} />
        ))}
      </div>
    </section>
  )
}
