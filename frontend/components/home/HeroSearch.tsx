"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { openSearch } from "@/lib/api"
import { Search } from "lucide-react"

export function HeroSearch() {
  const [query, setQuery] = useState("")
  const [isLoading, setIsLoading] = useState(false)
  const router = useRouter()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return

    setIsLoading(true)
    try {
      const isCitationLike = /^[A-Za-z]+\s*\d+/i.test(query) || /^\d+\.\d+/i.test(query) || /^chapter\s*\d+/i.test(query)
      
      if (isCitationLike) {
        const response = await openSearch(query)
        if (response.matched && response.href) {
          router.push(response.href)
          return
        }
      }
      
      router.push(`/search?q=${encodeURIComponent(query)}`)
    } catch {
      router.push(`/search?q=${encodeURIComponent(query)}`)
    } finally {
      setIsLoading(false)
    }
  }

  const handleChipClick = (chip: string) => {
    setQuery(chip)
  }

  const chips = [
    "ORS 90.300",
    "landlord notice",
    "security deposit deadline",
    "civil penalty",
    "operative date",
    "Department of Revenue tax"
  ]

  return (
    <div className="w-full max-w-3xl mx-auto mt-8">
      <form onSubmit={handleSubmit} className="relative group">
        <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
          <Search className="h-5 w-5 text-zinc-400 group-focus-within:text-zinc-200 transition-colors" />
        </div>
        <input
          type="text"
          className="block w-full pl-11 pr-4 py-4 bg-zinc-900/50 border border-zinc-800 rounded-xl text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500 transition-all text-lg shadow-lg"
          placeholder="Search ORS citations, duties, deadlines, definitions, penalties..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={isLoading}
        />
        <button 
          type="submit" 
          disabled={isLoading || !query.trim()}
          className="absolute inset-y-2 right-2 px-4 bg-indigo-600 hover:bg-indigo-500 disabled:bg-zinc-800 disabled:text-zinc-500 text-white rounded-lg font-medium transition-colors"
        >
          {isLoading ? "Loading..." : "Search"}
        </button>
      </form>
      
      <div className="flex flex-wrap gap-2 mt-4 justify-center">
        {chips.map((chip) => (
          <button
            key={chip}
            onClick={() => handleChipClick(chip)}
            className="px-3 py-1.5 text-xs font-mono text-zinc-400 bg-zinc-900/50 border border-zinc-800/80 hover:border-zinc-600 hover:text-zinc-200 rounded-full transition-colors"
          >
            {chip}
          </button>
        ))}
      </div>
    </div>
  )
}
