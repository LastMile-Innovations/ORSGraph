import { AlertCircle } from "lucide-react"

export function HomeOfflineBanner() {
  return (
    <div className="bg-amber-500/10 border border-amber-500/20 text-amber-500 rounded-lg p-4 mb-8 flex items-center gap-3 max-w-5xl mx-auto">
      <AlertCircle className="w-5 h-5 flex-shrink-0" />
      <p className="text-sm font-medium">
        Using mock data. ORSGraph API is offline.
      </p>
    </div>
  )
}
