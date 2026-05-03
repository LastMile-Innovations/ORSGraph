"use client"

import Link from "next/link"
import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useEffect, useMemo, useState } from "react"
import type { FormEvent, ReactNode } from "react"
import {
  BookOpen,
  Briefcase,
  ChevronDown,
  ChevronRight,
  Clock,
  Folder,
  Loader2,
  Plus,
  Search,
  Star,
  Trash2,
  WifiOff,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { DataState } from "@/lib/data-state"
import {
  deleteSidebarSearch,
  deleteSidebarStatute,
  recordSidebarRecentStatute,
  saveSidebarSearch,
  saveSidebarStatute,
  type SidebarChapter,
  type SidebarData,
  type SidebarSavedSearch,
  type SidebarStatute,
} from "@/lib/api"
import { matterHref } from "@/lib/casebuilder/routes"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"

const OPEN_STATE_KEY = "orsgraph:left-rail:open"
const DEFAULT_OPEN: Record<string, boolean> = {
  corpus: true,
  "saved-searches": true,
  "saved-statutes": true,
  recent: true,
}

interface LeftRailProps {
  initialState: DataState<SidebarData | null> | null
  className?: string
  onNavigate?: () => void
}

export function LeftRail({ initialState, className, onNavigate }: LeftRailProps) {
  const pathname = usePathname() || "/"
  const searchParams = useSearchParams()
  const router = useRouter()
  const [state, setState] = useState(initialState)
  const [openSections, setOpenSections] = useState(DEFAULT_OPEN)
  const [railQuery, setRailQuery] = useState("")
  const [pendingAction, setPendingAction] = useState<string | null>(null)
  const currentSearchQuery = pathname === "/search" ? (searchParams.get("q") ?? "").trim() : ""
  const currentStatuteId = useMemo(() => statuteIdFromPath(pathname), [pathname])
  const data = state?.data
  const canWrite = state?.source === "live"

  useEffect(() => {
    setState(initialState)
  }, [initialState])

  useEffect(() => {
    try {
      const saved = window.localStorage.getItem(OPEN_STATE_KEY)
      if (saved) setOpenSections({ ...DEFAULT_OPEN, ...JSON.parse(saved) })
    } catch {
      setOpenSections(DEFAULT_OPEN)
    }
  }, [])

  useEffect(() => {
    try {
      window.localStorage.setItem(OPEN_STATE_KEY, JSON.stringify(openSections))
    } catch {
      // Collapsed state is a preference; rendering should never depend on storage.
    }
  }, [openSections])

  useEffect(() => {
    if (!canWrite || !currentStatuteId) return
    let disposed = false
    recordSidebarRecentStatute(currentStatuteId)
      .then((statute) => {
        if (disposed) return
        setState((previous) => updateSidebarData(previous, (draft) => ({
          ...draft,
          recent_statutes: uniqueStatutes([statute, ...draft.recent_statutes]).slice(0, 12),
        })))
      })
      .catch(() => undefined)
    return () => {
      disposed = true
    }
  }, [canWrite, currentStatuteId])

  const savedSearchActive = Boolean(
    currentSearchQuery &&
      data?.saved_searches.some((item) => item.query.toLowerCase() === currentSearchQuery.toLowerCase()),
  )
  const savedStatuteActive = Boolean(
    currentStatuteId &&
      data?.saved_statutes.some((item) => statuteMatches(item, currentStatuteId)),
  )

  function toggleSection(id: string) {
    setOpenSections((current) => ({ ...current, [id]: !current[id] }))
  }

  function submitRailSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const trimmed = railQuery.trim()
    if (!trimmed) return
    router.push(`/search?q=${encodeURIComponent(trimmed)}`)
    onNavigate?.()
    setRailQuery("")
  }

  async function saveCurrentSearch() {
    if (!canWrite || !currentSearchQuery || savedSearchActive) return
    setPendingAction("save-search")
    try {
      const saved = await saveSidebarSearch({ query: currentSearchQuery })
      setState((previous) => updateSidebarData(previous, (draft) => ({
        ...draft,
        saved_searches: uniqueSearches([saved, ...draft.saved_searches]).slice(0, 12),
      })))
    } finally {
      setPendingAction(null)
    }
  }

  async function removeSearch(search: SidebarSavedSearch) {
    if (!canWrite) return
    setPendingAction(search.saved_search_id)
    try {
      await deleteSidebarSearch(search.saved_search_id)
      setState((previous) => updateSidebarData(previous, (draft) => ({
        ...draft,
        saved_searches: draft.saved_searches.filter((item) => item.saved_search_id !== search.saved_search_id),
      })))
    } finally {
      setPendingAction(null)
    }
  }

  async function saveCurrentStatute() {
    if (!canWrite || !currentStatuteId || savedStatuteActive) return
    setPendingAction("save-statute")
    try {
      const saved = await saveSidebarStatute(currentStatuteId)
      setState((previous) => updateSidebarData(previous, (draft) => ({
        ...draft,
        saved_statutes: uniqueStatutes([saved, ...draft.saved_statutes]).slice(0, 12),
      })))
    } finally {
      setPendingAction(null)
    }
  }

  async function removeStatute(statute: SidebarStatute) {
    if (!canWrite) return
    setPendingAction(statute.canonical_id)
    try {
      await deleteSidebarStatute(statute.canonical_id)
      setState((previous) => updateSidebarData(previous, (draft) => ({
        ...draft,
        saved_statutes: draft.saved_statutes.filter((item) => item.canonical_id !== statute.canonical_id),
      })))
    } finally {
      setPendingAction(null)
    }
  }

  if (!data) {
    return (
      <aside className={cn("flex h-full w-64 flex-col border-r border-sidebar-border bg-sidebar p-3 text-sidebar-foreground", className)}>
        <RailUnavailable />
      </aside>
    )
  }

  const source = state?.source ?? "error"

  return (
    <aside className={cn("flex h-full w-64 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar text-sidebar-foreground", className)}>
      <div className="border-b border-sidebar-border p-3">
        <form onSubmit={submitRailSearch} className="flex items-center gap-2 rounded-md border border-sidebar-border bg-background px-2 focus-within:border-primary">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <label className="sr-only" htmlFor="left-rail-search">Search ORSGraph</label>
          <input
            id="left-rail-search"
            value={railQuery}
            onChange={(event) => setRailQuery(event.target.value)}
            placeholder="Search ORS..."
            className="min-w-0 flex-1 bg-transparent py-1.5 text-xs outline-none placeholder:text-muted-foreground"
          />
        </form>
        <div className="mt-2 flex items-center justify-between font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <span>{data.corpus.jurisdiction} {data.corpus.corpus}</span>
          <span>{formatCount(data.corpus.total_statutes)}</span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-thin">
        <Section
          id="corpus"
          title="Corpus"
          icon={BookOpen}
          count={data.corpus.chapters.length}
          open={openSections.corpus}
          onToggle={toggleSection}
        >
          <div className="px-3 pb-1 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            {data.corpus.edition_year} edition
          </div>
          <div className="space-y-px">
            {data.corpus.chapters.map((chapter, index) => (
              <ChapterTree
                key={chapter.chapter}
                chapter={chapter}
                defaultOpen={chapterHasActiveStatute(chapter, currentStatuteId) || index === 0}
                currentStatuteId={currentStatuteId}
                onNavigate={onNavigate}
              />
            ))}
          </div>
        </Section>

        <Section
          id="saved-searches"
          title="Saved searches"
          icon={Search}
          count={data.saved_searches.length}
          open={openSections["saved-searches"]}
          onToggle={toggleSection}
          action={
            currentSearchQuery ? (
              <RailIconButton
                label={savedSearchActive ? "Search saved" : "Save current search"}
                disabled={!canWrite || savedSearchActive || pendingAction === "save-search"}
                onClick={saveCurrentSearch}
              >
                {pendingAction === "save-search" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plus className="h-3 w-3" />}
              </RailIconButton>
            ) : null
          }
        >
          <SidebarListEmpty show={data.saved_searches.length === 0} label="No saved searches" />
          <div className="space-y-px px-1">
            {data.saved_searches.map((search) => (
              <div key={search.saved_search_id} className="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-sidebar-accent">
                <Link href={`/search?q=${encodeURIComponent(search.query)}`} className="min-w-0 flex-1" onClick={onNavigate}>
                  <span className="block truncate text-xs text-foreground">{search.query}</span>
                </Link>
                {search.results > 0 && (
                  <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{search.results}</span>
                )}
                <RailIconButton
                  label="Remove saved search"
                  disabled={!canWrite || pendingAction === search.saved_search_id}
                  onClick={() => removeSearch(search)}
                  muted
                >
                  {pendingAction === search.saved_search_id ? <Loader2 className="h-3 w-3 animate-spin" /> : <Trash2 className="h-3 w-3" />}
                </RailIconButton>
              </div>
            ))}
          </div>
        </Section>

        <Section
          id="saved-statutes"
          title="Saved statutes"
          icon={Star}
          count={data.saved_statutes.length}
          open={openSections["saved-statutes"]}
          onToggle={toggleSection}
          action={
            currentStatuteId ? (
              <RailIconButton
                label={savedStatuteActive ? "Statute saved" : "Save current statute"}
                disabled={!canWrite || savedStatuteActive || pendingAction === "save-statute"}
                onClick={saveCurrentStatute}
              >
                {pendingAction === "save-statute" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plus className="h-3 w-3" />}
              </RailIconButton>
            ) : null
          }
        >
          <SidebarListEmpty show={data.saved_statutes.length === 0} label="No saved statutes" />
          <StatuteList
            statutes={data.saved_statutes}
            currentStatuteId={currentStatuteId}
            pendingAction={pendingAction}
            canWrite={canWrite}
            onRemove={removeStatute}
            onNavigate={onNavigate}
          />
        </Section>

        <Section
          id="recent"
          title="Recent"
          icon={Clock}
          count={data.recent_statutes.length}
          open={openSections.recent}
          onToggle={toggleSection}
        >
          <SidebarListEmpty show={data.recent_statutes.length === 0} label="No recent statutes" />
          <StatuteList statutes={data.recent_statutes} currentStatuteId={currentStatuteId} onNavigate={onNavigate} />
        </Section>
      </div>

      <div className="border-t border-sidebar-border bg-sidebar p-3">
        <div className="space-y-1 font-mono text-[10px] tabular-nums text-muted-foreground">
          <div className="flex items-center justify-between">
            <span>source</span>
            <span className={cn("flex items-center gap-1.5", source === "live" ? "text-success" : "text-warning")}>
              {source !== "live" && <WifiOff className="h-3 w-3" />}
              {source}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span>edition</span>
            <span className="text-foreground">{data.corpus.edition_year}</span>
          </div>
          <div className="flex items-center justify-between gap-3">
            <span>matter</span>
            {data.active_matter ? (
              <Link href={matterHref(data.active_matter.matter_id)} className="flex min-w-0 items-center gap-1 text-foreground hover:text-primary" onClick={onNavigate}>
                <Briefcase className="h-3 w-3 shrink-0" />
                <span className="truncate">{data.active_matter.name}</span>
              </Link>
            ) : (
              <span className="text-muted-foreground">none</span>
            )}
          </div>
        </div>
      </div>
    </aside>
  )
}

function Section({
  id,
  title,
  icon: Icon,
  count,
  open,
  onToggle,
  action,
  children,
}: {
  id: string
  title: string
  icon: typeof Folder
  count?: number
  open: boolean
  onToggle: (id: string) => void
  action?: ReactNode
  children: ReactNode
}) {
  return (
    <section className="border-b border-sidebar-border">
      <div className="flex items-center">
        <button
          type="button"
          aria-expanded={open}
          onClick={() => onToggle(id)}
          className="flex min-w-0 flex-1 items-center justify-between px-3 py-2 text-left text-[11px] font-mono uppercase tracking-wider text-muted-foreground hover:bg-sidebar-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
        >
          <span className="flex min-w-0 items-center gap-1.5">
            {open ? <ChevronDown className="h-3 w-3 shrink-0" /> : <ChevronRight className="h-3 w-3 shrink-0" />}
            <Icon className="h-3 w-3 shrink-0" />
            <span className="truncate">{title}</span>
          </span>
          {typeof count === "number" && <span className="ml-2 font-mono text-[10px] tabular-nums">{count}</span>}
        </button>
        {action && <div className="pr-2">{action}</div>}
      </div>
      {open && <div className="pb-2">{children}</div>}
    </section>
  )
}

function ChapterTree({
  chapter,
  defaultOpen,
  currentStatuteId,
  onNavigate,
}: {
  chapter: SidebarChapter
  defaultOpen: boolean
  currentStatuteId: string | null
  onNavigate?: () => void
}) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <div>
      <div className="flex items-center">
        <button
          type="button"
          onClick={() => setOpen((value) => !value)}
          aria-expanded={open}
          className="flex min-w-0 flex-1 items-center gap-1 px-3 py-1.5 text-left text-xs text-sidebar-foreground hover:bg-sidebar-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
        >
          {open ? <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" /> : <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />}
          <Folder className="h-3 w-3 shrink-0 text-muted-foreground" />
          <span className="truncate font-mono">Chapter {chapter.chapter}</span>
          <span className="ml-auto font-mono text-[10px] tabular-nums text-muted-foreground">{chapter.count}</span>
        </button>
      </div>
      {open && (
        <div className="ml-5 border-l border-sidebar-border">
          {chapter.items.map((statute) => (
            <StatuteLink key={statute.canonical_id} statute={statute} active={statuteMatches(statute, currentStatuteId)} onNavigate={onNavigate} />
          ))}
          {chapter.count > chapter.items.length && (
            <Link
              href={`/statutes?chapter=${encodeURIComponent(chapter.chapter)}`}
              onClick={onNavigate}
              className="flex px-2 py-1 font-mono text-[10px] uppercase tracking-widest text-muted-foreground hover:bg-sidebar-accent hover:text-foreground"
            >
              {chapter.count - chapter.items.length} more
            </Link>
          )}
        </div>
      )}
    </div>
  )
}

function StatuteList({
  statutes,
  currentStatuteId,
  pendingAction,
  canWrite,
  onRemove,
  onNavigate,
}: {
  statutes: SidebarStatute[]
  currentStatuteId: string | null
  pendingAction?: string | null
  canWrite?: boolean
  onRemove?: (statute: SidebarStatute) => void
  onNavigate?: () => void
}) {
  return (
    <div className="space-y-px px-1">
      {statutes.map((statute) => (
        <div key={statute.canonical_id} className="group flex items-center gap-1 rounded-md hover:bg-sidebar-accent">
          <StatuteLink statute={statute} active={statuteMatches(statute, currentStatuteId)} className="min-w-0 flex-1" onNavigate={onNavigate} />
          {onRemove && (
            <RailIconButton
              label="Remove saved statute"
              disabled={!canWrite || pendingAction === statute.canonical_id}
              onClick={() => onRemove(statute)}
              muted
            >
              {pendingAction === statute.canonical_id ? <Loader2 className="h-3 w-3 animate-spin" /> : <Trash2 className="h-3 w-3" />}
            </RailIconButton>
          )}
        </div>
      ))}
    </div>
  )
}

function StatuteLink({
  statute,
  active,
  className,
  onNavigate,
}: {
  statute: SidebarStatute
  active: boolean
  className?: string
  onNavigate?: () => void
}) {
  return (
    <Link
      href={`/statutes/${encodeURIComponent(statute.canonical_id)}`}
      aria-current={active ? "page" : undefined}
      onClick={onNavigate}
      className={cn(
        "flex min-w-0 flex-col rounded px-2 py-1 transition-colors",
        active ? "bg-primary/10 text-primary" : "hover:bg-sidebar-accent",
        className,
      )}
    >
      <span className="truncate font-mono text-[11px]">{statute.citation}</span>
      <span className={cn("truncate text-[10px]", active ? "text-primary/80" : "text-muted-foreground")}>{statute.title}</span>
    </Link>
  )
}

function RailIconButton({
  label,
  disabled,
  muted,
  onClick,
  children,
}: {
  label: string
  disabled?: boolean
  muted?: boolean
  onClick: () => void
  children: ReactNode
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={label}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation()
            onClick()
          }}
          className={cn(
            "flex h-6 w-6 shrink-0 items-center justify-center rounded-md outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/60 disabled:pointer-events-none disabled:opacity-40",
            muted ? "text-muted-foreground hover:bg-background hover:text-foreground" : "text-primary hover:bg-primary/10",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  )
}

function SidebarListEmpty({ show, label }: { show: boolean; label: string }) {
  if (!show) return null
  return <div className="px-3 py-2 text-xs text-muted-foreground">{label}</div>
}

function RailUnavailable() {
  return (
    <div className="flex h-full flex-col justify-between text-xs text-muted-foreground">
      <div>
        <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest">
          <WifiOff className="h-3 w-3" />
          Sidebar unavailable
        </div>
        <p className="leading-relaxed">The navigation service did not return usable data.</p>
      </div>
    </div>
  )
}

function updateSidebarData(
  previous: DataState<SidebarData | null> | null,
  updater: (data: SidebarData) => SidebarData,
): DataState<SidebarData | null> | null {
  if (!previous?.data) return previous
  return { ...previous, data: updater(previous.data) }
}

function uniqueSearches(searches: SidebarSavedSearch[]) {
  const seen = new Set<string>()
  return searches.filter((search) => {
    const key = search.query.toLowerCase()
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function uniqueStatutes(statutes: SidebarStatute[]) {
  const seen = new Set<string>()
  return statutes.filter((statute) => {
    if (seen.has(statute.canonical_id)) return false
    seen.add(statute.canonical_id)
    return true
  })
}

function statuteIdFromPath(pathname: string) {
  const match = pathname.match(/^\/statutes\/([^/]+)/)
  if (!match) return null
  try {
    return decodeURIComponent(match[1])
  } catch {
    return match[1]
  }
}

function statuteMatches(statute: SidebarStatute, currentStatuteId: string | null) {
  return Boolean(
    currentStatuteId &&
      (statute.canonical_id === currentStatuteId || statute.citation === currentStatuteId),
  )
}

function chapterHasActiveStatute(chapter: SidebarChapter, currentStatuteId: string | null) {
  return chapter.items.some((statute) => statuteMatches(statute, currentStatuteId))
}

function formatCount(value: number) {
  return new Intl.NumberFormat(undefined, { notation: value > 9999 ? "compact" : "standard" }).format(value)
}
