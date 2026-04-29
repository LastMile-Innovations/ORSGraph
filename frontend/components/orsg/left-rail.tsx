"use client"

import Link from "next/link"
import { useState } from "react"
import { cn } from "@/lib/utils"
import { recentItems, savedSearches, statuteIndex } from "@/lib/mock-data"
import { ChevronDown, ChevronRight, Folder, BookOpen, Search, Clock, Star } from "lucide-react"

function Section({
  title,
  defaultOpen = true,
  children,
  icon: Icon,
  count,
}: {
  title: string
  defaultOpen?: boolean
  children: React.ReactNode
  icon: typeof Folder
  count?: number
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div className="border-b border-sidebar-border">
      <button
        onClick={() => setOpen(!open)}
        className="flex w-full items-center justify-between px-3 py-2 text-left text-[11px] font-mono uppercase tracking-wider text-muted-foreground hover:bg-sidebar-accent"
      >
        <span className="flex items-center gap-1.5">
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
          <Icon className="h-3 w-3" />
          {title}
        </span>
        {count !== undefined && (
          <span className="font-mono text-[10px] tabular-nums">{count}</span>
        )}
      </button>
      {open && <div className="pb-2">{children}</div>}
    </div>
  )
}

// Group statute index by chapter for the corpus tree
function byChapter() {
  const grouped: Record<string, typeof statuteIndex> = {}
  for (const s of statuteIndex) {
    if (!grouped[s.chapter]) grouped[s.chapter] = []
    grouped[s.chapter].push(s)
  }
  return grouped
}

export function LeftRail() {
  const chapters = byChapter()
  return (
    <aside className="flex h-full w-60 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <div className="flex-1 overflow-y-auto scrollbar-thin">
        <Section title="Corpus" icon={BookOpen} count={statuteIndex.length}>
          <div className="px-3 pb-1 text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
            Oregon Revised Statutes / 2025
          </div>
          {Object.entries(chapters).map(([chapter, items]) => (
            <ChapterTree key={chapter} chapter={chapter} items={items} />
          ))}
        </Section>

        <Section title="Saved searches" icon={Search} count={savedSearches.length}>
          <div className="space-y-0.5 px-1">
            {savedSearches.map((s) => (
              <Link
                key={s.id}
                href={`/search?q=${encodeURIComponent(s.query)}`}
                className="flex items-center justify-between rounded px-2 py-1 text-xs hover:bg-sidebar-accent"
              >
                <span className="truncate text-foreground">{s.query}</span>
                <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                  {s.results}
                </span>
              </Link>
            ))}
          </div>
        </Section>

        <Section title="Saved statutes" icon={Star} count={3}>
          <div className="space-y-0.5 px-1">
            {recentItems.slice(0, 3).map((item) => (
              <Link
                key={item.canonical_id}
                href={`/statutes/${item.canonical_id}`}
                className="flex flex-col rounded px-2 py-1 hover:bg-sidebar-accent"
              >
                <span className="font-mono text-xs text-primary">{item.citation}</span>
                <span className="truncate text-[11px] text-muted-foreground">{item.title}</span>
              </Link>
            ))}
          </div>
        </Section>

        <Section title="Recent" icon={Clock} count={recentItems.length}>
          <div className="space-y-0.5 px-1">
            {recentItems.map((item) => (
              <Link
                key={item.canonical_id}
                href={`/statutes/${item.canonical_id}`}
                className="flex flex-col rounded px-2 py-1 hover:bg-sidebar-accent"
              >
                <span className="font-mono text-xs text-primary">{item.citation}</span>
                <span className="truncate text-[11px] text-muted-foreground">{item.title}</span>
              </Link>
            ))}
          </div>
        </Section>
      </div>

      <div className="border-t border-sidebar-border bg-sidebar p-3">
        <div className="space-y-1 font-mono text-[10px] tabular-nums text-muted-foreground">
          <div className="flex justify-between">
            <span>jurisdiction</span>
            <span className="text-foreground">Oregon</span>
          </div>
          <div className="flex justify-between">
            <span>edition</span>
            <span className="text-foreground">2025</span>
          </div>
          <div className="flex justify-between">
            <span>matter</span>
            <span className="text-muted-foreground">none</span>
          </div>
        </div>
      </div>
    </aside>
  )
}

function ChapterTree({ chapter, items }: { chapter: string; items: typeof statuteIndex }) {
  const [open, setOpen] = useState(chapter === "3")
  return (
    <div>
      <button
        onClick={() => setOpen(!open)}
        className={cn(
          "flex w-full items-center gap-1 px-3 py-1 text-left text-xs hover:bg-sidebar-accent",
        )}
      >
        {open ? (
          <ChevronDown className="h-3 w-3 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 text-muted-foreground" />
        )}
        <Folder className="h-3 w-3 text-muted-foreground" />
        <span className="font-mono">Chapter {chapter}</span>
        <span className="ml-auto font-mono text-[10px] tabular-nums text-muted-foreground">
          {items.length}
        </span>
      </button>
      {open && (
        <div className="ml-5 border-l border-sidebar-border">
          {items.map((s) => (
            <Link
              key={s.canonical_id}
              href={`/statutes/${s.canonical_id}`}
              className="flex flex-col gap-0 px-2 py-1 hover:bg-sidebar-accent"
            >
              <span className="font-mono text-[11px] text-primary">{s.citation}</span>
              <span className="truncate text-[10px] text-muted-foreground">{s.title}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
