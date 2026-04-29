"use client"

import { Search, Loader2 } from "lucide-react"
import { useState, useEffect, useRef } from "react"
import { searchSuggest } from "@/lib/api"
import type { SuggestResult } from "@/lib/types"
import { cn } from "@/lib/utils"

interface SearchInputProps {
  value: string
  onChange: (value: string) => void
  onKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void
  onSelectSuggestion?: (value: string) => void
  tookMs?: number
  totalResults?: number
}

export function SearchInput({ value, onChange, onKeyDown, onSelectSuggestion, tookMs, totalResults }: SearchInputProps) {
  const [suggestions, setSuggestions] = useState<SuggestResult[]>([])
  const [isSuggesting, setIsSuggesting] = useState(false)
  const [showDropdown, setShowDropdown] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const fetchSuggestions = async () => {
      if (value.length < 2) {
        setSuggestions([])
        return
      }

      setIsSuggesting(true)
      try {
        const res = await searchSuggest(value)
        setSuggestions(res)
        setShowDropdown(res.length > 0)
      } catch (err) {
        console.error("Suggest failed:", err)
      } finally {
        setIsSuggesting(false)
      }
    }

    const timer = setTimeout(fetchSuggestions, 300)
    return () => clearTimeout(timer)
  }, [value])

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowDropdown(false)
      }
    }
    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [])

  return (
    <div className="relative flex flex-col gap-2">
      <div className="flex items-center gap-2 rounded border border-border bg-background px-3 focus-within:border-primary">
        {isSuggesting ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" /> : <Search className="h-4 w-4 text-muted-foreground" />}
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') setShowDropdown(false)
            onKeyDown?.(e)
          }}
          onFocus={() => suggestions.length > 0 && setShowDropdown(true)}
          placeholder="Search statutes, provisions, definitions, deadlines, penalties..."
          className="flex-1 bg-transparent py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
        />
        {(tookMs !== undefined || totalResults !== undefined) && (
          <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
            {tookMs !== undefined ? `${tookMs}ms · ` : ""}
            {totalResults ?? 0} results
          </span>
        )}
      </div>

      {showDropdown && suggestions.length > 0 && (
        <div 
          ref={dropdownRef}
          className="absolute top-full left-0 right-0 z-50 mt-1 rounded-md border border-border bg-card shadow-lg overflow-hidden"
        >
          <div className="flex flex-col py-1">
            {suggestions.map((s, idx) => (
              <button
                key={idx}
                onClick={() => {
                  onChange(s.label)
                  setShowDropdown(false)
                  onSelectSuggestion?.(s.label)
                }}
                className="flex items-center gap-3 px-4 py-2 hover:bg-muted text-left transition-colors"
              >
                <span className="rounded bg-muted/50 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground w-20 text-center">
                  {s.kind}
                </span>
                <span className="text-sm font-medium flex-1 truncate">{s.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
