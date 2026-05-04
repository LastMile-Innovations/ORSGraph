import { TopNavBoundary } from "@/components/orsg/top-nav-boundary"

export default function MatterLoading() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopNavBoundary />
      <div className="flex flex-1 overflow-hidden">
        <aside className="hidden w-60 border-r border-sidebar-border bg-sidebar p-3 md:block">
          <div className="h-4 w-24 animate-pulse rounded bg-muted" />
          <div className="mt-4 h-8 w-40 animate-pulse rounded bg-muted" />
          <div className="mt-6 space-y-2">
            {Array.from({ length: 10 }).map((_, index) => (
              <div key={index} className="h-6 animate-pulse rounded bg-muted/70" />
            ))}
          </div>
        </aside>
        <main className="flex-1 overflow-hidden">
          <div className="border-b border-border bg-card px-6 py-5">
            <div className="h-3 w-36 animate-pulse rounded bg-muted" />
            <div className="mt-3 h-7 w-80 max-w-full animate-pulse rounded bg-muted" />
            <div className="mt-3 h-3 w-64 max-w-full animate-pulse rounded bg-muted" />
          </div>
          <div className="grid grid-cols-1 gap-4 p-6 xl:grid-cols-3">
            {Array.from({ length: 6 }).map((_, index) => (
              <div key={index} className="h-40 animate-pulse rounded border border-border bg-card" />
            ))}
          </div>
        </main>
      </div>
    </div>
  )
}
