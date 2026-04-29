"use client"

import { FormEvent, useState } from "react"
import { Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

export function GraphSearchBox({
  value,
  onSubmit,
}: {
  value: string
  onSubmit: (value: string) => void
}) {
  const [draft, setDraft] = useState(value)

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const next = draft.trim()
    if (next) onSubmit(next)
  }

  return (
    <form onSubmit={submit} className="flex gap-2">
      <div className="relative min-w-0 flex-1">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="ORS 90.300 or node id"
          className="h-9 pl-8 font-mono text-xs"
        />
      </div>
      <Button type="submit" size="sm" className="font-mono text-xs uppercase">
        Open
      </Button>
    </form>
  )
}
