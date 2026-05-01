import type { StatutePageResponse } from "@/lib/types"
import { TrustBadge } from "@/components/orsg/badges"
import Link from "next/link"
import type { ReactNode } from "react"

export function TextTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="mx-auto w-full max-w-4xl px-4 py-6 sm:px-6 lg:py-8">
      <div className="mb-4 flex items-center gap-2">
        <TrustBadge level="official" />
        <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
          version {data.current_version.version_id} · effective {data.current_version.effective_date}
        </span>
      </div>
      <article className="legal-text whitespace-pre-wrap text-[15px] leading-relaxed text-foreground sm:text-base">
        <LinkedStatuteText text={data.current_version.text} />
      </article>
      <div className="mt-8 border-t border-border pt-4 text-xs text-muted-foreground">
        Text shown is the official source as parsed from{" "}
        <a
          href={data.source_documents[0]?.url}
          target="_blank"
          rel="noreferrer"
          className="text-primary hover:underline"
        >
          oregonlegislature.gov
        </a>
        . Citations are linked to their resolved targets in the graph.
      </div>
    </div>
  )
}

function LinkedStatuteText({ text }: { text: string }) {
  const citationPattern = /(ORS\s+\d{1,3}[A-Z]?\.\d{3}(?:\([A-Za-z0-9]+\))*)/g
  const parts: ReactNode[] = []
  let lastIndex = 0
  let match: RegExpExecArray | null

  while ((match = citationPattern.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index))
    }
    const citation = match[1]
    const canonical = `or:ors:${citation.replace(/^ORS\s+/i, "")}`
    parts.push(
      <Link
        key={`${citation}-${match.index}`}
        href={`/statutes/${encodeURIComponent(canonical)}`}
        className="font-mono text-primary underline decoration-dotted underline-offset-2 hover:decoration-solid"
      >
        {citation}
      </Link>,
    )
    lastIndex = citationPattern.lastIndex
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex))
  }

  return <>{parts}</>
}
