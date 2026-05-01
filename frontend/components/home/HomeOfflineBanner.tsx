import { AlertCircle } from "lucide-react"

export function HomeOfflineBanner() {
  return (
    <div className="mb-6 flex items-center gap-3 rounded-md border border-warning/30 bg-warning/10 p-3 text-warning">
      <AlertCircle className="h-4 w-4 flex-shrink-0" />
      <p className="text-sm font-medium">
        Live home data is unavailable. The page is showing read-only fallback data.
      </p>
    </div>
  )
}
