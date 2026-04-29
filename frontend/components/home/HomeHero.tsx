import { HeroSearch } from "./HeroSearch"
import Link from "next/link"

export function HomeHero() {
  return (
    <section className="relative pt-24 pb-16 px-4 sm:px-6 lg:px-8 text-center max-w-5xl mx-auto">
      <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-zinc-900 border border-zinc-800 text-xs font-mono text-zinc-400 mb-8">
        <span className="w-2 h-2 rounded-full bg-indigo-500"></span>
        ORSGraph · internal v0 / Oregon Revised Statutes · 2025 edition
      </div>
      
      <h1 className="text-4xl sm:text-5xl lg:text-6xl font-bold tracking-tight text-white mb-6">
        The legal operating environment <br className="hidden sm:block" />
        <span className="text-zinc-500">for Oregon law.</span>
      </h1>
      
      <p className="text-lg text-zinc-400 max-w-3xl mx-auto mb-10 leading-relaxed">
        Source-first statute intelligence powered by a dense Neo4j legal graph: provisions, citations, definitions, obligations, deadlines, penalties, amendments, source notes, retrieval chunks, and QC-visible graph operations.
      </p>

      <div className="flex items-center justify-center gap-4 mb-8">
        <Link 
          href="/ask" 
          className="px-6 py-2.5 bg-white text-zinc-950 hover:bg-zinc-200 rounded-lg font-medium transition-colors"
        >
          Ask ORSGraph
        </Link>
        <Link 
          href="/search" 
          className="px-6 py-2.5 bg-zinc-900 border border-zinc-800 hover:border-zinc-700 text-zinc-100 rounded-lg font-medium transition-colors"
        >
          Explore Search
        </Link>
      </div>

      <HeroSearch />
    </section>
  )
}
