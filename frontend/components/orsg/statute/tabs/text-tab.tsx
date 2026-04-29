import type { StatutePageResponse } from "@/lib/types"
import { TrustBadge } from "@/components/orsg/badges"

export function TextTab({ data }: { data: StatutePageResponse }) {
  // Highlight ORS citations in the body so users see graph edges inline.
  const html = highlightCitations(data.current_version.text)

  return (
    <div className="mx-auto max-w-3xl px-6 py-8">
      <div className="mb-4 flex items-center gap-2">
        <TrustBadge level="official" />
        <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
          version {data.current_version.version_id} · effective {data.current_version.effective_date}
        </span>
      </div>
      <article
        className="legal-text whitespace-pre-wrap text-[15px] leading-relaxed text-foreground"
        dangerouslySetInnerHTML={{ __html: html }}
      />
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

function highlightCitations(text: string): string {
  // Simple regex highlight for ORS X.YYY references.
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /(ORS\s+\d+(?:[A-Z])?\.\d+(?:\([^)]+\))*)/g,
      '<a class="font-mono text-primary underline decoration-dotted underline-offset-2 hover:decoration-solid" href="/statutes/or:ors:$1">$1</a>',
    )
    .replace(/href="\/statutes\/or:ors:ORS\s+/g, 'href="/statutes/or:ors:')
}
