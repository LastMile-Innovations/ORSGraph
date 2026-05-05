"use client"

import NextForm from "next/form"
import Link from "next/link"
import { signIn, signOut, useSession } from "next-auth/react"
import { usePathname, useRouter } from "next/navigation"
import { useCallback, useEffect, useMemo, useState } from "react"
import type { FormEvent, ReactNode } from "react"
import type { LucideIcon } from "lucide-react"
import {
  Activity,
  BookOpen,
  Briefcase,
  CheckCircle2,
  CircleAlert,
  CircleDashed,
  Database,
  GitGraphIcon,
  LayoutDashboard,
  LogIn,
  LogOut,
  Menu,
  MessageSquare,
  Plus,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  UserCircle,
  WifiOff,
} from "lucide-react"
import { cn } from "@/lib/utils"
import {
  fetchRuntimeStatus,
  INITIAL_RUNTIME_STATUS,
  type RuntimeStatus,
  type RuntimeState,
} from "@/lib/runtime-status"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"
import { ThemeToggle } from "./theme-toggle"

interface NavItem {
  href: string
  label: string
  icon: LucideIcon
  match: string[]
}

const NAV_ITEMS: NavItem[] = [
  { href: "/dashboard", label: "Home", icon: LayoutDashboard, match: ["/dashboard"] },
  { href: "/casebuilder", label: "Matters", icon: Briefcase, match: ["/casebuilder", "/matters"] },
  { href: "/search", label: "Search", icon: Search, match: ["/search"] },
  { href: "/ask", label: "Ask", icon: MessageSquare, match: ["/ask"] },
  { href: "/statutes", label: "Statutes", icon: BookOpen, match: ["/statutes", "/provisions"] },
  { href: "/graph", label: "Graph", icon: GitGraphIcon, match: ["/graph"] },
  { href: "/admin", label: "Admin", icon: SlidersHorizontal, match: ["/admin"] },
]

const STATUS_REFRESH_MS = 60_000

function isActiveItem(item: NavItem, pathname: string) {
  return item.match.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`))
}

function useRuntimeStatus() {
  const [status, setStatus] = useState<RuntimeStatus>(INITIAL_RUNTIME_STATUS)

  useEffect(() => {
    let disposed = false
    let inFlight: AbortController | undefined

    async function loadStatus() {
      if (document.visibilityState === "hidden") return

      inFlight?.abort()
      const controller = new AbortController()
      inFlight = controller

      try {
        const nextStatus = await fetchRuntimeStatus(controller.signal)
        if (!disposed && !controller.signal.aborted) setStatus(nextStatus)
      } catch {
        if (!disposed && !controller.signal.aborted) {
          setStatus({
            state: "offline",
            api: "offline",
            neo4j: "unknown",
            checkedAt: new Date().toISOString(),
            message: "Health check failed",
          })
        }
      }
    }

    loadStatus()

    const interval = window.setInterval(loadStatus, STATUS_REFRESH_MS)
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") loadStatus()
    }

    document.addEventListener("visibilitychange", handleVisibilityChange)

    return () => {
      disposed = true
      inFlight?.abort()
      window.clearInterval(interval)
      document.removeEventListener("visibilitychange", handleVisibilityChange)
    }
  }, [])

  return status
}

function runtimeStatusMeta(state: RuntimeState) {
  switch (state) {
    case "connected":
      return {
        label: "Connected",
        icon: CheckCircle2,
        dotClass: "bg-success",
        buttonClass: "text-success hover:bg-success/10 hover:text-success",
      }
    case "degraded":
      return {
        label: "Degraded",
        icon: CircleAlert,
        dotClass: "bg-warning",
        buttonClass: "text-warning hover:bg-warning/10 hover:text-warning",
      }
    case "offline":
      return {
        label: "Offline",
        icon: WifiOff,
        dotClass: "bg-destructive",
        buttonClass: "text-destructive hover:bg-destructive/10 hover:text-destructive",
      }
    case "checking":
    default:
      return {
        label: "Checking",
        icon: CircleDashed,
        dotClass: "bg-muted-foreground",
        buttonClass: "text-muted-foreground hover:bg-muted hover:text-foreground",
      }
  }
}

function formatCheckedAt(value?: string) {
  if (!value) return "not checked yet"
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value))
}

export function TopNav({ leftRailTrigger }: { leftRailTrigger?: ReactNode }) {
  const pathname = usePathname() || "/"
  const router = useRouter()
  const [query, setQuery] = useState("")
  const [mobileOpen, setMobileOpen] = useState(false)
  const status = useRuntimeStatus()
  const session = useSession()
  const activeLabel = useMemo(
    () => NAV_ITEMS.find((item) => isActiveItem(item, pathname))?.label ?? "Home",
    [pathname],
  )
  const searchPlaceholder = useMemo(() => getSearchPlaceholder(pathname, activeLabel), [pathname, activeLabel])

  useEffect(() => {
    setQuery("")
    setMobileOpen(false)
  }, [pathname])

  const submitSearch = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!query.trim()) {
        event.preventDefault()
        return
      }
      const exactCitationHref = exactOrsCitationHref(query)
      if (exactCitationHref) {
        event.preventDefault()
        setMobileOpen(false)
        setQuery("")
        router.push(exactCitationHref)
        return
      }
      setMobileOpen(false)
      window.requestAnimationFrame(() => setQuery(""))
    },
    [query, router],
  )

  return (
    <header className="sticky top-0 z-40 flex h-14 shrink-0 items-center gap-2 border-b border-sidebar-border bg-sidebar/95 px-3 text-sidebar-foreground shadow-sm shadow-black/5 backdrop-blur supports-[backdrop-filter]:bg-sidebar/85 sm:px-4">
      <a
        href="#app-main"
        className="sr-only rounded bg-background px-3 py-2 text-sm font-medium text-foreground shadow focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-50"
      >
        Skip to content
      </a>

      <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
        <SheetTrigger asChild>
          <Button variant="ghost" size="icon-sm" className="md:hidden" aria-label="Open navigation">
            <Menu className="h-4 w-4" />
          </Button>
        </SheetTrigger>
        <SheetContent side="left" className="w-[20rem] max-w-[86vw] gap-0 border-sidebar-border bg-sidebar p-0 text-sidebar-foreground">
          <SheetHeader className="border-b border-sidebar-border pr-12">
            <SheetTitle className="flex items-center gap-2 text-sm">
              <BrandMark />
              ORSGraph
            </SheetTitle>
            <SheetDescription className="font-mono text-[10px] uppercase tracking-widest">
              Oregon Revised Statutes / 2025
            </SheetDescription>
          </SheetHeader>

          <NextForm action="/search" onSubmit={submitSearch} className="border-b border-sidebar-border p-3">
            <label className="sr-only" htmlFor="mobile-header-search">
              Search ORSGraph
            </label>
            <div className="flex items-center gap-2 rounded-md border border-sidebar-border bg-background px-2 focus-within:border-primary">
              <Search className="h-3.5 w-3.5 text-muted-foreground" />
              <input
                id="mobile-header-search"
                name="q"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={searchPlaceholder}
                className="min-w-0 flex-1 bg-transparent py-2 text-sm outline-none placeholder:text-muted-foreground"
              />
              <Button type="submit" variant="ghost" size="icon-sm" className="h-7 w-7" aria-label="Search">
                <Search className="h-3.5 w-3.5" />
              </Button>
            </div>
          </NextForm>

          <nav aria-label="Primary navigation" className="flex flex-col gap-1 p-2">
            {NAV_ITEMS.map((item) => {
              const active = isActiveItem(item, pathname)
              return (
                <TopNavLink
                  key={item.href}
                  item={item}
                  active={active}
                  variant="mobile"
                  onNavigate={() => setMobileOpen(false)}
                />
              )
            })}
          </nav>

          <div className="mt-auto border-t border-sidebar-border p-3">
            <RuntimeStatusSummary status={status} />
          </div>
        </SheetContent>
      </Sheet>

      {leftRailTrigger}

      <Link href="/dashboard" className="group flex min-w-0 shrink-0 items-center gap-2 rounded-md pr-1 outline-none focus-visible:ring-2 focus-visible:ring-ring/60">
        <BrandMark />
        <span className="truncate font-mono text-sm font-semibold tracking-tight">ORSGraph</span>
        <span className="hidden font-mono text-[10px] uppercase tracking-widest text-muted-foreground xl:inline">
          workspace
        </span>
      </Link>

      <nav aria-label="Primary navigation" className="hidden min-w-0 items-center gap-1 md:flex">
        {NAV_ITEMS.map((item) => {
          const active = isActiveItem(item, pathname)
          return <TopNavLink key={item.href} item={item} active={active} />
        })}
      </nav>

      <div className="min-w-0 flex-1" />

      <NextForm action="/search" onSubmit={submitSearch} className="hidden w-full max-w-md items-center gap-2 rounded-md border border-sidebar-border bg-background px-2 shadow-sm focus-within:border-primary lg:flex">
        <label className="sr-only" htmlFor="header-search">
          Search ORSGraph
        </label>
        <Search className="h-3.5 w-3.5 text-muted-foreground" />
        <input
          id="header-search"
          name="q"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={searchPlaceholder}
          className="min-w-0 flex-1 bg-transparent py-1.5 text-sm outline-none placeholder:text-muted-foreground"
        />
        <Button type="submit" variant="ghost" size="icon-sm" className="h-7 w-7" aria-label="Search">
          <Search className="h-3.5 w-3.5" />
        </Button>
      </NextForm>

      <div className="flex shrink-0 items-center gap-1.5">
        <Button asChild variant="outline" size="sm" className="hidden h-8 gap-1.5 px-2.5 xl:inline-flex">
          <Link href="/casebuilder/new">
            <Plus className="h-3.5 w-3.5" />
            New matter
          </Link>
        </Button>
        <RuntimeStatusMenu status={status} />
        <AccountMenu
          status={session.status}
          accessStatus={session.data?.accessStatus}
          user={session.data?.user}
          roles={session.data?.roles ?? []}
        />
        <ThemeToggle />
      </div>
    </header>
  )
}

function exactOrsCitationHref(value: string) {
  const match = value.trim().match(/^ORS\s+([0-9A-Za-z]+(?:\.[0-9A-Za-z]+)?)$/i)
  if (!match) return null
  return `/statutes/${encodeURIComponent(`or:ors:${match[1]}`)}`
}

function getSearchPlaceholder(pathname: string, activeLabel: string) {
  if (pathname === "/search" || pathname.startsWith("/search/")) {
    return "Search statutes, provisions, or concepts..."
  }
  if (pathname === "/ask" || pathname.startsWith("/ask/")) {
    return "Search ORSGraph..."
  }
  if (pathname.startsWith("/casebuilder") || pathname.startsWith("/matters")) {
    return "Search matters, statutes, or evidence..."
  }
  if (pathname.startsWith("/admin")) {
    return "Search ORSGraph..."
  }
  return `Search ${activeLabel.toLowerCase()}...`
}

function AccountMenu({
  status,
  user,
  roles,
  accessStatus,
}: {
  status: "authenticated" | "loading" | "unauthenticated"
  accessStatus?: string
  user?: { name?: string | null; email?: string | null; image?: string | null }
  roles: string[]
}) {
  const label = status === "authenticated" ? user?.name || user?.email || "Account" : "Sign in"

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-8 gap-2 px-2" aria-label={label}>
          <UserCircle className="h-4 w-4" />
          <span className="hidden max-w-32 truncate text-xs lg:inline">{label}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel className="min-w-0 text-xs">
          <div className="truncate">{label}</div>
          {user?.email && <div className="truncate font-normal text-muted-foreground">{user.email}</div>}
        </DropdownMenuLabel>
        {roles.length > 0 && (
          <>
            <DropdownMenuSeparator />
            <div className="px-2 py-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              {roles.join(", ")}
            </div>
          </>
        )}
        <DropdownMenuSeparator />
        {status === "authenticated" && accessStatus && accessStatus !== "active" && (
          <>
            <DropdownMenuItem asChild>
              <Link href="/auth/pending">
                <ShieldCheck className="h-4 w-4" />
                Access {accessStatus}
              </Link>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
          </>
        )}
        {status === "authenticated" && (
          <DropdownMenuItem asChild>
            <Link href="/casebuilder/settings">
              <SlidersHorizontal className="h-4 w-4" />
              CaseBuilder settings
            </Link>
          </DropdownMenuItem>
        )}
        {status === "authenticated" ? (
          <DropdownMenuItem className="cursor-pointer" onClick={() => signOut({ callbackUrl: "/" })}>
            <LogOut className="h-4 w-4" />
            Sign out
          </DropdownMenuItem>
        ) : (
          <DropdownMenuItem className="cursor-pointer" onClick={() => signIn("zitadel", { callbackUrl: "/onboarding" })}>
            <LogIn className="h-4 w-4" />
            Sign in
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function BrandMark() {
  return (
    <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground shadow-sm shadow-primary/20">
      <Activity className="h-4 w-4" strokeWidth={2.5} />
    </span>
  )
}

function TopNavLink({
  item,
  active,
  onNavigate,
  variant = "desktop",
}: {
  item: NavItem
  active: boolean
  onNavigate?: () => void
  variant?: "desktop" | "mobile"
}) {
  const Icon = item.icon

  return (
    <Link
      href={item.href}
      aria-current={active ? "page" : undefined}
      onClick={onNavigate}
      className={cn(
        "group flex shrink-0 items-center gap-2 rounded-md font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/60",
        variant === "mobile" ? "px-3 py-2 text-sm" : "px-2 py-1.5 text-xs xl:px-2.5",
        active
          ? "bg-primary/15 text-primary ring-1 ring-primary/15"
          : "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
      )}
      title={item.label}
    >
      <Icon className={cn("shrink-0", variant === "mobile" ? "h-4 w-4" : "h-3.5 w-3.5")} />
      <span className={cn(variant === "desktop" && "hidden lg:inline")}>{item.label}</span>
      {active && variant === "mobile" && (
        <span className="ml-auto h-1.5 w-1.5 rounded-full bg-primary" aria-hidden="true" />
      )}
    </Link>
  )
}

function RuntimeStatusMenu({ status }: { status: RuntimeStatus }) {
  const meta = runtimeStatusMeta(status.state)
  const Icon = meta.icon

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={cn("h-8 gap-2 px-2 font-mono text-[10px] uppercase tracking-wide", meta.buttonClass)}
          aria-label={`Runtime status: ${meta.label}`}
        >
          <span className={cn("h-2 w-2 rounded-full", meta.dotClass)} aria-hidden="true" />
          <span className="hidden xl:inline">{meta.label}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel className="flex items-center gap-2 text-xs">
          <Icon className="h-3.5 w-3.5" />
          Runtime status
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <StatusRow icon={Activity} label="API" value={status.api} />
        <StatusRow icon={Database} label="Neo4j" value={status.neo4j} />
        <StatusRow icon={CircleDashed} label="Checked" value={formatCheckedAt(status.checkedAt)} />
        {status.version && <StatusRow icon={CheckCircle2} label="Version" value={status.version} />}
        {status.message && (
          <>
            <DropdownMenuSeparator />
            <div className="px-2 py-1.5 text-xs leading-relaxed text-muted-foreground">{status.message}</div>
          </>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem asChild className="cursor-pointer">
          <Link href="/admin">
            <SlidersHorizontal className="h-4 w-4" />
            Open Admin
          </Link>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function RuntimeStatusSummary({ status }: { status: RuntimeStatus }) {
  const meta = runtimeStatusMeta(status.state)

  return (
    <div className="space-y-2 text-xs">
      <div className="flex items-center justify-between gap-3">
        <span className="font-mono uppercase tracking-widest text-muted-foreground">Runtime</span>
        <span className={cn("flex items-center gap-1.5 font-medium", meta.buttonClass)}>
          <span className={cn("h-2 w-2 rounded-full", meta.dotClass)} aria-hidden="true" />
          {meta.label}
        </span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-muted-foreground">
        <span>API</span>
        <span className="text-right text-foreground">{status.api}</span>
        <span>Neo4j</span>
        <span className="text-right text-foreground">{status.neo4j}</span>
      </div>
    </div>
  )
}

function StatusRow({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon
  label: string
  value: string
}) {
  return (
    <div className="flex items-center gap-2 px-2 py-1.5 text-xs">
      <Icon className="h-3.5 w-3.5 text-muted-foreground" />
      <span className="text-muted-foreground">{label}</span>
      <span className="ml-auto max-w-32 truncate font-mono text-[11px] text-foreground">{value}</span>
    </div>
  )
}
