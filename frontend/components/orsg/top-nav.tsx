"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { cn } from "@/lib/utils"
import { ThemeToggle } from "./theme-toggle"
import { Search, MessageSquare, BookOpen, GitGraphIcon as Graph, ShieldCheck, Activity, Briefcase } from "lucide-react"

const NAV_ITEMS = [
  { href: "/search", label: "Search", icon: Search },
  { href: "/ask", label: "Ask", icon: MessageSquare },
  { href: "/statutes", label: "Statutes", icon: BookOpen },
  { href: "/graph", label: "Graph", icon: Graph },
  { href: "/qc", label: "QC", icon: ShieldCheck },
  { href: "/casebuilder", label: "Matters", icon: Briefcase },
]

export function TopNav() {
  const pathname = usePathname()

  return (
    <header className="flex h-12 items-center justify-between gap-3 border-b border-border bg-sidebar px-3 sm:px-4">
      <div className="flex min-w-0 items-center gap-3 sm:gap-6">
        <Link href="/" className="flex shrink-0 items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded bg-primary text-primary-foreground">
            <Activity className="h-3.5 w-3.5" strokeWidth={2.5} />
          </div>
          <span className="font-mono text-sm font-semibold tracking-tight">ORSGraph</span>
          <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            v0 / internal
          </span>
        </Link>

        <nav className="flex min-w-0 items-center gap-1 overflow-x-auto scrollbar-none">
          {NAV_ITEMS.map((item) => {
            const active =
              pathname === item.href ||
              (item.href !== "/" && pathname.startsWith(item.href))
            const Icon = item.icon
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "flex shrink-0 items-center gap-1.5 rounded px-2 py-1 text-xs font-medium transition-colors sm:px-2.5",
                  active
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
                title={item.label}
              >
                <Icon className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">{item.label}</span>
              </Link>
            )
          })}
        </nav>
      </div>

      <div className="flex items-center gap-3">
        <div className="hidden items-center gap-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground md:flex">
          <span className="flex h-1.5 w-1.5 rounded-full bg-muted-foreground" />
          api unknown
          <span className="text-border">|</span>
          <span className="flex h-1.5 w-1.5 rounded-full bg-muted-foreground" />
          qc unknown
          <span className="text-border">|</span>
          <span>edition 2025</span>
        </div>
        <ThemeToggle />
      </div>
    </header>
  )
}
