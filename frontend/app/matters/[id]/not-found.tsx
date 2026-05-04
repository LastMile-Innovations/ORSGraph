import Link from "next/link"
import { FileQuestion } from "lucide-react"
import { TopNavBoundary } from "@/components/orsg/top-nav-boundary"
import { Button } from "@/components/ui/button"
import { casebuilderHomeHref, newMatterHref } from "@/lib/casebuilder/routes"

export default function MatterNotFound() {
  return (
    <div className="flex min-h-screen flex-col bg-background">
      <TopNavBoundary />
      <main className="flex flex-1 items-center justify-center px-6 py-12">
        <section className="max-w-md rounded border border-border bg-card p-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <FileQuestion className="h-3.5 w-3.5" />
            matter not found
          </div>
          <h1 className="mt-2 text-lg font-semibold text-foreground">No matter matches this route.</h1>
          <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
            The matter may not exist yet, or the link may be using an old identifier.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button asChild size="sm">
              <Link href={casebuilderHomeHref()}>All matters</Link>
            </Button>
            <Button asChild variant="outline" size="sm">
              <Link href={newMatterHref()}>New matter</Link>
            </Button>
          </div>
        </section>
      </main>
    </div>
  )
}
