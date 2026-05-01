"use client"

import Link from "next/link"
import { AlertTriangle, RotateCcw } from "lucide-react"
import { TopNav } from "@/components/orsg/top-nav"
import { Button } from "@/components/ui/button"
import { casebuilderHomeHref } from "@/lib/casebuilder/routes"

export default function MatterError({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <div className="flex min-h-screen flex-col bg-background">
      <TopNav />
      <main className="flex flex-1 items-center justify-center px-6 py-12">
        <section className="max-w-md rounded border border-border bg-card p-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-destructive">
            <AlertTriangle className="h-3.5 w-3.5" />
            matter failed to load
          </div>
          <h1 className="mt-2 text-lg font-semibold text-foreground">CaseBuilder hit a page error.</h1>
          <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
            {error.message || "The matter route could not render. Retry the route or return to the matters index."}
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button size="sm" onClick={reset} className="gap-1.5">
              <RotateCcw className="h-3.5 w-3.5" />
              Retry
            </Button>
            <Button asChild variant="outline" size="sm">
              <Link href={casebuilderHomeHref()}>All matters</Link>
            </Button>
          </div>
        </section>
      </main>
    </div>
  )
}
