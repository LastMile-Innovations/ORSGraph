"use client"

import Link from "next/link"
import { AlertTriangle, ArrowLeft, Loader2, Search } from "lucide-react"
import { Button } from "@/components/ui/button"

interface RouteStateProps {
  title: string
  message?: string
  homeHref?: string
  homeLabel?: string
  reset?: () => void
}

export function RouteLoadingState({ title = "Loading", message }: Partial<RouteStateProps>) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-md rounded border border-border bg-card p-6 text-center">
        <Loader2 className="mx-auto h-6 w-6 animate-spin text-primary" />
        <h1 className="mt-4 text-base font-semibold text-foreground">{title}</h1>
        {message && <p className="mt-1 text-sm text-muted-foreground">{message}</p>}
      </div>
    </main>
  )
}

export function RouteErrorState({
  title,
  message = "The page hit an unexpected error while loading.",
  homeHref = "/",
  homeLabel = "Home",
  reset,
}: RouteStateProps) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-lg rounded border border-border bg-card p-6">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 h-5 w-5 flex-none text-destructive" />
          <div className="min-w-0">
            <h1 className="text-base font-semibold text-foreground">{title}</h1>
            <p className="mt-1 text-sm text-muted-foreground">{message}</p>
            <div className="mt-4 flex flex-wrap gap-2">
              {reset && (
                <Button type="button" size="sm" onClick={reset}>
                  Try again
                </Button>
              )}
              <Button asChild size="sm" variant="outline">
                <Link href={homeHref}>
                  <ArrowLeft className="h-3.5 w-3.5" />
                  {homeLabel}
                </Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    </main>
  )
}

export function RouteNotFoundState({
  title,
  message = "That record is not available in the current corpus.",
  homeHref = "/search",
  homeLabel = "Search",
}: RouteStateProps) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-lg rounded border border-border bg-card p-6">
        <div className="flex items-start gap-3">
          <Search className="mt-0.5 h-5 w-5 flex-none text-primary" />
          <div className="min-w-0">
            <h1 className="text-base font-semibold text-foreground">{title}</h1>
            <p className="mt-1 text-sm text-muted-foreground">{message}</p>
            <Button asChild size="sm" className="mt-4">
              <Link href={homeHref}>{homeLabel}</Link>
            </Button>
          </div>
        </div>
      </div>
    </main>
  )
}
