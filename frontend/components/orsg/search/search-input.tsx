"use client"

import { CornerDownRight, ExternalLink, Loader2, Search } from "lucide-react"
import { useState, useEffect, useRef } from "react"
import { searchSuggest } from "@/lib/api"
import type { SuggestResult } from "@/lib/types"
import { cn } from "@/lib/utils"

interface SearchInputProps {
  value: string
  onChange: (value: string) => void
  onKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void
  onSelectSuggestion?: (value: SuggestResult) => void
  tookMs?: number
  totalResults?: number
}

export function SearchInput({ value, onChange, onKeyDown, onSelectSuggestion, tookMs, totalResults }: SearchInputProps) {
  const [suggestions, setSuggestions] = useState<SuggestResult[]>([])
  const [isSuggesting, setIsSuggesting] = useState(false)
  const [showDropdown, setShowDropdown] = useState(false)
  const [activeIndex, setActiveIndex] = useState(-1)
  const [submittedValue, setSubmittedValue] = useState<string | null>(null)
  const dropdownRef = useRef<HTMLDivElement>(null)
  const suggestRequestRef = useRef(0)

  useEffect(() => {
    const requestId = ++suggestRequestRef.current
    const controller = new AbortController()
    const fetchSuggestions = async () => {
      if (value.length < 2) {
        setSuggestions([])
        setShowDropdown(false)
        setIsSuggesting(false)
        return
      }

      setIsSuggesting(true)
      try {
        const res = await searchSuggest(value, 10, controller.signal)
        if (requestId !== suggestRequestRef.current) return
        setSuggestions(res)
        setShowDropdown(res.length > 0 && value !== submittedValue)
        setActiveIndex(-1)
      } catch (err) {
        if (requestId !== suggestRequestRef.current) return
        if (err instanceof Error && err.name === "AbortError") return
        console.error("Suggest failed:", err)
      } finally {
        if (requestId === suggestRequestRef.current) {
          setIsSuggesting(false)
        }
      }
    }

    const timer = setTimeout(fetchSuggestions, 300)
    return () => {
      controller.abort()
      clearTimeout(timer)
    }
  }, [value, submittedValue])

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowDropdown(false)
        setActiveIndex(-1)
      }
    }
    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [])

  const selectSuggestion = (suggestion: SuggestResult) => {
    onChange(suggestion.label)
    setSubmittedValue(suggestion.label)
    setShowDropdown(false)
    setActiveIndex(-1)
    onSelectSuggestion?.(suggestion)
  }

  return (
    <div className="relative flex flex-col gap-2">
      <div className="flex items-center gap-2 rounded border border-border bg-background px-3 focus-within:border-primary">
        {isSuggesting ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" /> : <Search className="h-4 w-4 text-muted-foreground" />}
        <input
          value={value}
          onChange={(e) => {
            setSubmittedValue(null)
            onChange(e.target.value)
          }}
          onKeyDown={(e) => {
            if (showDropdown && suggestions.length > 0) {
              if (e.key === "ArrowDown") {
                e.preventDefault()
                setActiveIndex((current) => (current + 1) % suggestions.length)
                return
              }
              if (e.key === "ArrowUp") {
                e.preventDefault()
                setActiveIndex((current) => (current <= 0 ? suggestions.length - 1 : current - 1))
                return
              }
              if (e.key === "Enter" && activeIndex >= 0) {
                e.preventDefault()
                selectSuggestion(suggestions[activeIndex])
                return
              }
              if (e.key === "Escape") {
                e.preventDefault()
                setShowDropdown(false)
                setActiveIndex(-1)
                return
              }
            }
            if (e.key === "Enter") {
              setSubmittedValue(value)
              setShowDropdown(false)
              setActiveIndex(-1)
            }
            onKeyDown?.(e)
          }}
          onFocus={() => suggestions.length > 0 && value !== submittedValue && setShowDropdown(true)}
          placeholder="Search statutes, provisions, definitions, deadlines, penalties..."
          className="flex-1 bg-transparent py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
        />
        {(tookMs !== undefined || totalResults !== undefined) && (
          <span className="hidden font-mono text-[10px] uppercase tracking-wide text-muted-foreground sm:inline">
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
            {suggestions.map((s, idx) => {
              const opensAuthority = s.href && !s.href.startsWith("/search")
              return (
              <button
                key={`${s.kind}:${s.label}:${idx}`}
                onClick={() => selectSuggestion(s)}
                onMouseEnter={() => setActiveIndex(idx)}
                className={cn(
                  "flex min-h-12 items-center gap-3 px-4 py-2 text-left transition-colors",
                  activeIndex === idx ? "bg-muted text-foreground" : "hover:bg-muted",
                )}
              >
                <span className="w-24 flex-none rounded bg-muted/50 px-1.5 py-0.5 text-center font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  {s.kind}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium">{s.label}</span>
                  {(s.citation || s.canonical_id) && (
                    <span className="block truncate font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                      {s.citation || s.canonical_id}
                    </span>
                  )}
                </span>
                {opensAuthority ? (
                  <ExternalLink className="h-3.5 w-3.5 flex-none text-primary" />
                ) : (
                  <CornerDownRight className="h-3.5 w-3.5 flex-none text-muted-foreground" />
                )}
              </button>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
