"use client"

import { useState } from "react"
import type { StatutePageResponse } from "@/lib/types"
import { cn } from "@/lib/utils"
import { TextTab } from "./tabs/text-tab"
import { ProvisionTreeTab } from "./tabs/provision-tree-tab"
import { CitationsTab } from "./tabs/citations-tab"
import { DefinitionsTab } from "./tabs/definitions-tab"
import { DeadlinesTab } from "./tabs/deadlines-tab"
import { ExceptionsTab } from "./tabs/exceptions-tab"
import { ChunksTab } from "./tabs/chunks-tab"
import { VersionsTab } from "./tabs/versions-tab"
import { SourceTab } from "./tabs/source-tab"
import { GraphTab } from "./tabs/graph-tab"
import { QCTab } from "./tabs/qc-tab"

const TABS = [
  { id: "text", label: "Text" },
  { id: "tree", label: "Provision tree" },
  { id: "citations", label: "Citations" },
  { id: "definitions", label: "Definitions" },
  { id: "deadlines", label: "Deadlines" },
  { id: "exceptions", label: "Exceptions" },
  { id: "chunks", label: "Chunks" },
  { id: "versions", label: "Versions" },
  { id: "source", label: "Source" },
  { id: "graph", label: "Graph" },
  { id: "qc", label: "QC" },
] as const

type TabId = (typeof TABS)[number]["id"]

export function StatuteTabs({ data }: { data: StatutePageResponse }) {
  const [active, setActive] = useState<TabId>("text")

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center gap-0 overflow-x-auto border-b border-border bg-card px-4 scrollbar-thin">
        {TABS.map((tab) => {
          const count = getTabCount(tab.id, data)
          return (
            <button
              key={tab.id}
              onClick={() => setActive(tab.id)}
              className={cn(
                "relative flex items-center gap-1.5 whitespace-nowrap px-3 py-2.5 text-xs font-medium transition-colors",
                active === tab.id
                  ? "text-primary"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {tab.label}
              {count !== null && (
                <span
                  className={cn(
                    "rounded px-1 font-mono text-[10px] tabular-nums",
                    active === tab.id
                      ? "bg-primary/15 text-primary"
                      : "bg-muted text-muted-foreground",
                  )}
                >
                  {count}
                </span>
              )}
              {active === tab.id && (
                <span className="absolute inset-x-0 bottom-0 h-0.5 bg-primary" />
              )}
            </button>
          )
        })}
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-thin">
        {active === "text" && <TextTab data={data} />}
        {active === "tree" && <ProvisionTreeTab data={data} />}
        {active === "citations" && <CitationsTab data={data} />}
        {active === "definitions" && <DefinitionsTab data={data} />}
        {active === "deadlines" && <DeadlinesTab data={data} />}
        {active === "exceptions" && <ExceptionsTab data={data} />}
        {active === "chunks" && <ChunksTab data={data} />}
        {active === "versions" && <VersionsTab data={data} />}
        {active === "source" && <SourceTab data={data} />}
        {active === "graph" && <GraphTab data={data} />}
        {active === "qc" && <QCTab data={data} />}
      </div>
    </div>
  )
}

function getTabCount(id: TabId, data: StatutePageResponse): number | null {
  switch (id) {
    case "tree":
      return countProvisions(data.provisions)
    case "citations":
      return data.outbound_citations.length + data.inbound_citations.length
    case "definitions":
      return data.definitions.length
    case "deadlines":
      return data.deadlines.length
    case "exceptions":
      return data.exceptions.length
    case "chunks":
      return data.chunks.length
    case "versions":
      return data.versions.length
    case "qc":
      return data.qc.notes.length
    default:
      return null
  }
}

function countProvisions(provisions: any[]): number {
  let count = 0
  function walk(p: any) {
    count++
    if (p.children) p.children.forEach(walk)
  }
  provisions.forEach(walk)
  return count
}
