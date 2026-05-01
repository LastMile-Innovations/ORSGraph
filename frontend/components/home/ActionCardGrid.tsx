import { HomeAction } from "@/lib/types"
import { ActionCard } from "./ActionCard"

export function ActionCardGrid({ actions }: { actions: HomeAction[] }) {
  if (!actions.length) return null

  return (
    <section className="mb-12">
      <h2 className="mb-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">graph workspace</h2>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {actions.map(action => (
          <ActionCard key={action.title} action={action} />
        ))}
      </div>
    </section>
  )
}
