import { FeaturedStatute } from "@/lib/types"
import Link from "next/link"
import { Book } from "lucide-react"

export function FeaturedStatutesGrid({ statutes }: { statutes: FeaturedStatute[] }) {
  if (!statutes || statutes.length === 0) return null

  return (
    <section className="mb-16">
      <h2 className="text-xl font-semibold text-zinc-100 mb-6">Featured Statutes</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {statutes.map(statute => (
          <Link 
            key={statute.citation} 
            href={statute.href}
            className="flex flex-col p-5 bg-zinc-900 border border-zinc-800 rounded-xl hover:bg-zinc-800 hover:border-zinc-600 transition-colors"
          >
            <div className="flex items-start justify-between mb-2">
              <div className="flex items-center gap-2 text-indigo-400 font-medium">
                <Book className="w-4 h-4" />
                {statute.citation}
              </div>
              <span className="text-xs text-zinc-500 font-mono">{statute.chapter}</span>
            </div>
            <h3 className="text-zinc-100 font-semibold mb-4">{statute.title}</h3>
            
            <div className="flex flex-wrap gap-2 mt-auto">
              {statute.semanticTypes.map(type => (
                <span key={type} className="px-2 py-0.5 text-[10px] uppercase tracking-wider font-mono rounded bg-zinc-950 text-zinc-400 border border-zinc-800">
                  {type}
                </span>
              ))}
              {statute.status === "active" && (
                <span className="px-2 py-0.5 text-[10px] uppercase tracking-wider font-mono rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  Open
                </span>
              )}
            </div>
          </Link>
        ))}
      </div>
    </section>
  )
}
