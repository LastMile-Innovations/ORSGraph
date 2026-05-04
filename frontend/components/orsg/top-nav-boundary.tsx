import { Suspense, type ReactNode } from "react"
import { TopNav } from "./top-nav"

export function TopNavBoundary({ leftRailTrigger }: { leftRailTrigger?: ReactNode }) {
  return (
    <Suspense fallback={<TopNavFallback />}>
      <TopNav leftRailTrigger={leftRailTrigger} />
    </Suspense>
  )
}

function TopNavFallback() {
  return (
    <header
      className="sticky top-0 z-40 flex h-14 shrink-0 items-center gap-2 border-b border-sidebar-border bg-sidebar/95 px-3 text-sidebar-foreground shadow-sm shadow-black/5 backdrop-blur sm:px-4"
      aria-hidden="true"
    >
      <div className="h-8 w-8 rounded bg-sidebar-accent/70 md:hidden" />
      <div className="h-7 w-36 rounded bg-sidebar-accent/70" />
      <div className="ml-auto hidden h-8 w-64 rounded bg-sidebar-accent/70 md:block" />
      <div className="h-8 w-8 rounded bg-sidebar-accent/70" />
    </header>
  )
}
