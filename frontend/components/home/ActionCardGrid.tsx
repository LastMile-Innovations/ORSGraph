import { HomeAction } from "@/lib/types"
import { ActionCard } from "./ActionCard"

export function ActionCardGrid({ actions }: { actions: HomeAction[] }) {
  if (!actions.length) return null

  return (
    <section className="mb-12">
      <div className="mb-4 flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">graph workspace</h2>
          <p className="mt-1 text-sm text-muted-foreground">Start from the surface that matches the job in front of you.</p>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {actions.map(action => (
          <ActionCard key={action.title} action={action} />
        ))}
      </div>
    </section>
  )
}
