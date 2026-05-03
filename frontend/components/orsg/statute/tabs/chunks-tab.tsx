import type { StatutePageResponse } from "@/lib/types"
import { ChunkTypeBadge } from "@/components/orsg/badges"
import { Check, X } from "lucide-react"

export function ChunksTab({ data }: { data: StatutePageResponse }) {
  if (data.chunks.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 py-16 text-sm text-muted-foreground">
        No retrieval chunks are attached to this statute.
      </div>
    )
  }

  return (
    <div className="px-6 py-6">
      <div className="mb-3 flex items-baseline gap-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <span>{data.chunks.length} chunks derived from this statute</span>
        <span className="text-border">·</span>
        <span>
          {data.chunks.filter((c) => c.embedded).length} embedded /{" "}
          {data.chunks.filter((c) => !c.embedded).length} pending
        </span>
      </div>

      <div className="overflow-hidden rounded border border-border bg-card">
        <table className="w-full text-sm">
          <thead className="border-b border-border bg-muted/40">
            <tr className="text-left font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="px-3 py-2 font-medium">chunk_id</th>
              <th className="px-3 py-2 font-medium">type</th>
              <th className="px-3 py-2 font-medium">source</th>
              <th className="px-3 py-2 font-medium">policy</th>
              <th className="px-3 py-2 text-right font-medium">weight</th>
              <th className="px-3 py-2 text-right font-medium">conf.</th>
              <th className="px-3 py-2 text-center font-medium">embed</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {data.chunks.map((chunk) => (
              <tr key={chunk.chunk_id} className="hover:bg-muted/30">
                <td className="px-3 py-2 font-mono text-[11px] text-foreground">
                  {chunk.chunk_id}
                </td>
                <td className="px-3 py-2">
                  <ChunkTypeBadge type={chunk.chunk_type} />
                </td>
                <td className="px-3 py-2 font-mono text-[11px]">
                  <span className="text-muted-foreground">{chunk.source_kind}: </span>
                  <span className="text-primary">{chunk.source_id}</span>
                </td>
                <td className="px-3 py-2 font-mono text-[10px] uppercase tracking-wide">
                  <div className="text-foreground">{chunk.embedding_policy}</div>
                  <div className="text-muted-foreground">{chunk.answer_policy}</div>
                </td>
                <td className="px-3 py-2 text-right font-mono tabular-nums text-foreground">
                  {chunk.search_weight.toFixed(2)}
                </td>
                <td className="px-3 py-2 text-right font-mono tabular-nums text-foreground">
                  {chunk.parser_confidence.toFixed(2)}
                </td>
                <td className="px-3 py-2 text-center">
                  {chunk.embedded ? (
                    <Check className="mx-auto h-3.5 w-3.5 text-success" />
                  ) : (
                    <X className="mx-auto h-3.5 w-3.5 text-muted-foreground" />
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-3 md:grid-cols-2">
        {data.chunks.slice(0, 4).map((chunk) => (
          <div
            key={chunk.chunk_id}
            className="rounded border border-border bg-card p-3"
          >
            <div className="flex items-center gap-2">
              <ChunkTypeBadge type={chunk.chunk_type} />
              <span className="font-mono text-[10px] text-muted-foreground">{chunk.chunk_id}</span>
            </div>
            <p className="mt-2 text-xs leading-relaxed text-foreground">{chunk.text}</p>
          </div>
        ))}
      </div>
    </div>
  )
}
