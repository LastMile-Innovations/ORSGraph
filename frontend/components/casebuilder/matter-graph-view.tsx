"use client"

import Link from "next/link"
import { useMemo, useState } from "react"
import { AlertTriangle, GitGraphIcon, Layers3, Search } from "lucide-react"
import type { CaseGraphResponse, Matter } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"

export function MatterGraphView({
  matter,
  graph,
  error,
}: {
  matter: Matter
  graph: CaseGraphResponse | null
  error?: string
}) {
  const [mode, setMode] = useState("overview")
  const [query, setQuery] = useState("")

  const visible = useMemo(() => {
    const nodes = graph?.nodes ?? []
    const edges = graph?.edges ?? []
    const q = query.trim().toLowerCase()
    const allowedKinds =
      mode === "overview"
        ? null
        : new Set(
            mode === "evidence"
              ? ["matter", "document", "fact", "evidence", "claim", "element"]
              : mode === "claims"
                ? ["matter", "claim", "counterclaim", "defense", "element", "fact", "evidence", "authority"]
                : mode === "timeline"
                  ? ["matter", "event", "timeline_suggestion", "deadline", "fact", "document", "task"]
                  : mode === "authority"
                    ? ["matter", "claim", "element", "authority", "work_product"]
                    : mode === "work_product"
                      ? ["matter", "work_product", "fact", "evidence", "document", "authority"]
                        : mode === "markdown"
                          ? [
                              "matter",
                              "document",
                              "document_version",
                              "embedding_run",
                              "embedding_record",
                              "markdown_ast_document",
                              "markdown_ast_node",
                              "markdown_semantic_unit",
                            "text_chunk",
                            "source_span",
                          ]
                        : mode === "markdown_ast"
                          ? ["matter", "document", "document_version", "markdown_ast_document", "markdown_ast_node", "markdown_semantic_unit"]
                          : mode === "markdown_semantic"
                            ? ["matter", "document", "document_version", "markdown_ast_document", "markdown_semantic_unit", "markdown_ast_node", "case_entity", "entity_mention"]
                            : mode === "markdown_embeddings"
                              ? [
                                  "matter",
                                  "document",
                                  "document_version",
                                  "index_run",
                                  "embedding_run",
                                  "embedding_record",
                                  "markdown_ast_document",
                                  "markdown_ast_node",
                                  "markdown_semantic_unit",
                                  "text_chunk",
                                  "source_span",
                                ]
                            : mode === "entities"
                              ? ["matter", "document", "entity_mention", "case_entity", "party", "text_chunk", "source_span", "markdown_semantic_unit"]
                            : mode === "provenance"
                              ? [
                                  "matter",
                                  "document",
                                  "document_version",
                                  "index_run",
                                  "extraction_manifest",
                                  "source_span",
                                  "text_chunk",
                                  "evidence_span",
                                  "search_index_record",
                                  "embedding_run",
                                  "embedding_record",
                                  "markdown_ast_document",
                                  "markdown_ast_node",
                                  "markdown_semantic_unit",
                                  "fact",
                                  "event",
                                  "timeline_suggestion",
                                ]
                              : ["matter", "task", "deadline", "claim", "document"],
          )
    const filteredNodes = nodes.filter((node) => {
      if (allowedKinds && !allowedKinds.has(node.kind)) return false
      if (!q) return true
      return `${node.label} ${node.subtitle ?? ""} ${node.kind} ${node.status ?? ""}`.toLowerCase().includes(q)
    })
    const nodeIds = new Set(filteredNodes.map((node) => node.id))
    return {
      nodes: filteredNodes,
      edges: edges.filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)),
    }
  }, [graph, mode, query])

  const nodeById = useMemo(() => new Map((graph?.nodes ?? []).map((node) => [node.id, node])), [graph])

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <GitGraphIcon className="h-3.5 w-3.5 text-primary" />
              case graph
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Graph Viewer</h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              Derived matter graph for parties, documents, facts, evidence, claims, deadlines, tasks, authorities, and work product.
            </p>
          </div>
          <div className="rounded border border-border bg-background px-3 py-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            {visible.nodes.length} nodes / {visible.edges.length} edges
          </div>
        </div>
        {(error || graph?.warnings?.[0]) && (
          <div className="mt-3 flex items-start gap-2 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5" />
            <span>{error || graph?.warnings?.[0]}</span>
          </div>
        )}
        <div className="mt-4 flex flex-wrap items-center gap-2">
          {(graph?.modes ?? ["overview", "evidence", "claims", "timeline", "authority", "work_product", "tasks"]).map((item) => (
            <button
              key={item}
              type="button"
              onClick={() => setMode(item)}
              className={cn(
                "rounded border px-2.5 py-1 font-mono text-[10px] uppercase tracking-wider",
                mode === item ? "border-primary text-primary" : "border-border text-muted-foreground hover:border-primary/40 hover:text-primary",
              )}
            >
              {item.replace("_", " ")}
            </button>
          ))}
          <div className="ml-auto flex min-w-[220px] items-center gap-2 rounded border border-border bg-background px-2.5">
            <Search className="h-3.5 w-3.5 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search graph"
              className="min-w-0 flex-1 bg-transparent py-1.5 text-xs focus:outline-none"
            />
          </div>
        </div>
      </header>

      <main className="grid gap-4 px-6 py-6 xl:grid-cols-[minmax(0,1fr)_420px]">
        <section className="min-h-[520px] rounded border border-border bg-card p-4">
          <div className="mb-3 flex items-center gap-2 text-sm font-medium text-foreground">
            <Layers3 className="h-4 w-4 text-primary" />
            Matter graph map
          </div>
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
            {visible.nodes.map((node) => (
              <Link
                key={node.id}
                href={node.href || "#"}
                className={cn(
                  "min-h-28 rounded border bg-background p-3 transition-colors hover:border-primary/40",
                  toneForKind(node.kind),
                )}
              >
                <div className="flex items-start justify-between gap-2">
                  <span className="rounded bg-card px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {node.kind.replace("_", " ")}
                  </span>
                  {node.status && <span className="font-mono text-[10px] text-muted-foreground">{node.status}</span>}
                </div>
                <h2 className="mt-2 line-clamp-2 text-sm font-semibold leading-snug text-foreground">{node.label}</h2>
                {node.subtitle && <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{node.subtitle}</p>}
                {node.risk && <p className="mt-2 font-mono text-[10px] uppercase tracking-wider text-warning">{node.risk}</p>}
              </Link>
            ))}
          </div>
          {visible.nodes.length === 0 && (
            <div className="flex min-h-80 items-center justify-center rounded border border-dashed border-border text-sm text-muted-foreground">
              No graph nodes match this view.
            </div>
          )}
        </section>

        <aside className="rounded border border-border bg-card">
          <div className="border-b border-border px-4 py-3">
            <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">relationships</h2>
            <p className="mt-1 text-xs text-muted-foreground">{matter.name}</p>
          </div>
          <div className="max-h-[620px] divide-y divide-border overflow-y-auto">
            {visible.edges.map((edge) => {
              const source = nodeById.get(edge.source)
              const target = nodeById.get(edge.target)
              return (
                <div key={edge.id} className="p-3 text-xs">
                  <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{edge.label}</div>
                  <div className="mt-1 grid gap-1">
                    <span className="line-clamp-1 text-foreground">{source?.label ?? edge.source}</span>
                    <span className="text-muted-foreground">→</span>
                    <span className="line-clamp-1 text-foreground">{target?.label ?? edge.target}</span>
                  </div>
                </div>
              )
            })}
            {visible.edges.length === 0 && <div className="p-4 text-sm text-muted-foreground">No relationships in this view.</div>}
          </div>
        </aside>
      </main>
    </div>
  )
}

function toneForKind(kind: string) {
  if (kind === "matter") return "border-primary/40"
  if (kind === "claim" || kind === "element") return "border-case-claim/30"
  if (kind === "counterclaim") return "border-case-counterclaim/30"
  if (kind === "defense") return "border-case-defense/30"
  if (kind === "evidence" || kind === "fact" || kind === "evidence_span") return "border-case-evidence/30"
  if (kind === "timeline_suggestion") return "border-case-timeline/30"
  if (kind === "deadline" || kind === "task") return "border-case-deadline/30"
  if (kind === "authority") return "border-case-authority/30"
  if (kind === "work_product") return "border-case-work-product/30"
  if (kind === "markdown_ast_document" || kind === "markdown_ast_node") return "border-case-document/30"
  if (kind === "markdown_semantic_unit") return "border-case-work-product/30"
  if (kind === "embedding_run" || kind === "embedding_record") return "border-info/30"
  if (kind === "source_span" || kind === "text_chunk" || kind === "search_index_record") return "border-case-muted/30"
  if (kind === "entity_mention" || kind === "case_entity") return "border-case-entity/30"
  if (kind === "document_version" || kind === "index_run" || kind === "extraction_manifest") return "border-case-document/30"
  return "border-border"
}
