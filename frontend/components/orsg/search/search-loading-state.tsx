import { Skeleton } from "@/components/ui/skeleton"

export function SearchLoadingState() {
  return (
    <div className="flex-1 overflow-y-auto scrollbar-thin">
      <div className="border-b border-border px-6 py-2 bg-muted/20">
        <div className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground animate-pulse">
          Searching graph...
        </div>
      </div>
      <ul className="divide-y divide-border">
        {Array.from({ length: 5 }).map((_, i) => (
          <li key={i} className="px-6 py-6">
            <div className="flex items-start gap-3">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 mb-3">
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-5 w-24" />
                  <Skeleton className="h-4 w-48" />
                </div>
                <div className="space-y-2">
                  <Skeleton className="h-4 w-full" />
                  <Skeleton className="h-4 w-5/6" />
                </div>
                <div className="mt-4 flex items-center gap-4">
                  <Skeleton className="h-3 w-20" />
                  <Skeleton className="h-3 w-20" />
                  <Skeleton className="h-3 w-20" />
                </div>
              </div>
              <Skeleton className="h-12 w-1" />
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}
