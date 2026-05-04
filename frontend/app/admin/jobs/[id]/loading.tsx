import { TopNavBoundary } from "@/components/orsg/top-nav-boundary"

export default function AdminJobLoading() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TopNavBoundary />
      <div className="flex flex-1 overflow-hidden">
        <aside className="hidden w-72 shrink-0 border-r border-border bg-sidebar p-4 lg:block">
          <div className="h-4 w-28 animate-pulse rounded bg-muted" />
          <div className="mt-6 space-y-2">
            {Array.from({ length: 8 }).map((_, index) => (
              <div key={index} className="h-6 animate-pulse rounded bg-muted/70" />
            ))}
          </div>
        </aside>
        <main className="min-w-0 flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-6xl">
            <div className="mb-5">
              <div className="h-3 w-32 animate-pulse rounded bg-muted" />
              <div className="mt-3 h-8 w-80 max-w-full animate-pulse rounded bg-muted" />
              <div className="mt-3 h-3 w-64 max-w-full animate-pulse rounded bg-muted" />
            </div>
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
              <section className="h-96 animate-pulse rounded-md border border-border bg-card" />
              <aside className="h-96 animate-pulse rounded-md border border-border bg-card" />
            </div>
          </div>
        </main>
      </div>
    </div>
  )
}
