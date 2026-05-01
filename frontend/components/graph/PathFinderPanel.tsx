"use client"

import { useEffect, useRef, useState } from "react"
import { GitBranch, Search } from "lucide-react"
import { getGraphPath, type GraphPathResponse } from "@/lib/api"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

export function PathFinderPanel({
  mode = "legal",
  initialFrom,
}: {
  mode?: string
  initialFrom?: string
}) {
  const [from, setFrom] = useState(initialFrom ?? "")
  const [to, setTo] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<GraphPathResponse | null>(null)
  const previousInitialFrom = useRef(initialFrom)

  useEffect(() => {
    if (!initialFrom) return
    setFrom((current) => {
      if (!current || current === previousInitialFrom.current) return initialFrom
      return current
    })
    previousInitialFrom.current = initialFrom
  }, [initialFrom])

  async function findPath(event: React.FormEvent) {
    event.preventDefault()
    if (!from.trim() || !to.trim()) return
    setLoading(true)
    setError(null)
    try {
      const next = await getGraphPath({ from: from.trim(), to: to.trim(), mode, limit: 3 })
      setResult(next)
    } catch (reason) {
      setResult(null)
      setError(reason instanceof Error ? reason.message : "Path search failed.")
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-3 rounded border border-border bg-card p-3 text-sm">
      <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        <GitBranch className="h-3.5 w-3.5" />
        Path finder
      </div>
      <form onSubmit={findPath} className="grid gap-2">
        <Input value={from} onChange={(event) => setFrom(event.target.value)} placeholder="from citation or node id" className="h-8 text-xs" />
        <Input value={to} onChange={(event) => setTo(event.target.value)} placeholder="to citation or node id" className="h-8 text-xs" />
        <Button type="submit" size="sm" disabled={loading || !from.trim() || !to.trim()} className="gap-1">
          <Search className="h-3.5 w-3.5" />
          {loading ? "Searching" : "Find path"}
        </Button>
      </form>
      {error && <p className="text-xs text-destructive">{error}</p>}
      {result && (
        <div className="rounded border border-border bg-background p-2 font-mono text-[11px]">
          {result.paths.length === 0 ? (
            <p className="text-muted-foreground">{result.stats.warnings[0] ?? "No path found."}</p>
          ) : (
            <ul className="space-y-2">
              {result.paths.map((path, index) => (
                <li key={index}>
                  <div className="text-muted-foreground">path {index + 1} · {path.length} hop{path.length === 1 ? "" : "s"}</div>
                  <div className="mt-1 break-all text-foreground">{path.node_ids.join(" -> ")}</div>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}
