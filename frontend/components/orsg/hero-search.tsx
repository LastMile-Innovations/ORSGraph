"use client"

import { useRouter } from "next/navigation"
import { useState } from "react"
import { Search, Sparkles, ArrowRight } from "lucide-react"

const SUGGESTIONS = [
  "What Oregon laws define district attorney duties?",
  "30-day notice deadlines in ORS chapter 3",
  "Definition of public offense",
  "Penalties for official misconduct",
]

export function HeroSearch() {
  const router = useRouter()
  const [q, setQ] = useState("")

  function submit(value: string, mode: "ask" | "search") {
    if (!value.trim()) return
    if (mode === "ask") {
      router.push(`/ask?q=${encodeURIComponent(value)}`)
    } else {
      router.push(`/search?q=${encodeURIComponent(value)}`)
    }
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 rounded border border-border bg-background px-3 focus-within:border-primary">
        <Search className="h-4 w-4 text-muted-foreground" />
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit(q, e.shiftKey ? "search" : "ask")
          }}
          placeholder="What law controls...?"
          className="flex-1 bg-transparent py-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
        />
        <button
          onClick={() => submit(q, "search")}
          className="flex h-7 items-center gap-1 rounded border border-border px-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground hover:border-primary hover:text-primary"
        >
          search
          <ArrowRight className="h-3 w-3" />
        </button>
        <button
          onClick={() => submit(q, "ask")}
          className="flex h-7 items-center gap-1 rounded bg-primary px-2.5 font-mono text-[10px] uppercase tracking-wide text-primary-foreground hover:opacity-90"
        >
          <Sparkles className="h-3 w-3" />
          ask
        </button>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
          try:
        </span>
        {SUGGESTIONS.map((s) => (
          <button
            key={s}
            onClick={() => {
              setQ(s)
              submit(s, "ask")
            }}
            className="rounded border border-border bg-background px-2 py-1 text-xs text-muted-foreground hover:border-primary/40 hover:text-foreground"
          >
            {s}
          </button>
        ))}
      </div>
    </div>
  )
}
