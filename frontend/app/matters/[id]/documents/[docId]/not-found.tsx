import Link from "next/link"
import { FileQuestion } from "lucide-react"
import { TopNav } from "@/components/orsg/top-nav"
import { Button } from "@/components/ui/button"
import { casebuilderHomeHref } from "@/lib/casebuilder/routes"

export default function DocumentNotFound() {
  return (
    <div className="flex min-h-screen flex-col bg-background">
      <TopNav />
      <main className="flex flex-1 items-center justify-center px-6 py-12">
        <section className="max-w-md rounded border border-border bg-card p-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <FileQuestion className="h-3.5 w-3.5" />
            document not found
          </div>
          <h1 className="mt-2 text-lg font-semibold text-foreground">
            No document matches this route.
          </h1>
          <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
            The document may not exist yet, may have been removed from the matter, or the link may
            be using an old identifier.
          </p>
          <Button asChild size="sm" className="mt-4">
            <Link href={casebuilderHomeHref()}>All matters</Link>
          </Button>
        </section>
      </main>
    </div>
  )
}
