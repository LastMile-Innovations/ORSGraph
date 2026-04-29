"use client"

import { useState } from "react"
import Link from "next/link"
import {
  Plus,
  Sparkles,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Circle,
  BookOpen,
  Scale,
  Shield,
  Gavel,
  ChevronRight,
} from "lucide-react"
import type { Matter, Claim, ClaimElement } from "@/lib/casebuilder/types"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import { ConfidenceBadge } from "./badges"
import { cn } from "@/lib/utils"

interface ClaimsBuilderProps {
  matter: Matter
}

const KIND_META: Record<Claim["kind"], { label: string; icon: typeof Scale; color: string }> = {
  claim: {
    label: "Cause of Action",
    icon: Scale,
    color: "text-blue-700 dark:text-blue-300 border-blue-500/40 bg-blue-500/10",
  },
  counterclaim: {
    label: "Counterclaim",
    icon: Gavel,
    color: "text-purple-700 dark:text-purple-300 border-purple-500/40 bg-purple-500/10",
  },
  defense: {
    label: "Affirmative Defense",
    icon: Shield,
    color: "text-emerald-700 dark:text-emerald-300 border-emerald-500/40 bg-emerald-500/10",
  },
}

export function ClaimsBuilder({ matter }: ClaimsBuilderProps) {
  const [tab, setTab] = useState<Claim["kind"] | "all">("all")
  const [selected, setSelected] = useState<string | null>(matter.claims[0]?.id ?? null)

  const filteredClaims =
    tab === "all" ? matter.claims : matter.claims.filter((c) => c.kind === tab)
  const selectedClaim = matter.claims.find((c) => c.id === selected) ?? filteredClaims[0]

  return (
    <div className="flex flex-col">
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">
              Claims & Defenses
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Build your theory of the case. Each claim breaks down to its prima facie elements.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Sparkles className="h-3.5 w-3.5" />
              Suggest claims
            </Button>
            <Button size="sm" className="gap-1.5">
              <Plus className="h-3.5 w-3.5" />
              New claim
            </Button>
          </div>
        </div>

        <Tabs value={tab} onValueChange={(v) => setTab(v as Claim["kind"] | "all")} className="mt-4">
          <TabsList className="bg-muted/40">
            <TabsTrigger value="all" className="text-xs">
              All ({matter.claims.length})
            </TabsTrigger>
            <TabsTrigger value="claim" className="text-xs">
              Claims ({matter.claims.filter((c) => c.kind === "claim").length})
            </TabsTrigger>
            <TabsTrigger value="counterclaim" className="text-xs">
              Counterclaims ({matter.claims.filter((c) => c.kind === "counterclaim").length})
            </TabsTrigger>
            <TabsTrigger value="defense" className="text-xs">
              Defenses ({matter.claims.filter((c) => c.kind === "defense").length})
            </TabsTrigger>
          </TabsList>
          <TabsContent value={tab} className="mt-0" />
        </Tabs>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[360px_minmax(0,1fr)]">
        {/* Claims list */}
        <div className="border-r border-border bg-card">
          <ScrollArea className="h-[calc(100vh-220px)]">
            <ul className="divide-y divide-border">
              {filteredClaims.map((claim) => {
                const isActive = selectedClaim?.id === claim.id
                const supported = claim.elements.filter((e) => e.status === "supported").length
                const total = claim.elements.length
                return (
                  <li key={claim.id} id={claim.id}>
                    <button
                      onClick={() => setSelected(claim.id)}
                      className={cn(
                        "flex w-full items-start gap-3 px-4 py-3 text-left transition-colors",
                        isActive ? "bg-muted/60" : "hover:bg-muted/30",
                      )}
                    >
                      <KindBadge kind={claim.kind} small />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium leading-tight text-foreground text-pretty">
                          {claim.title}
                        </p>
                        <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
                          {claim.cause}
                        </p>
                        <div className="mt-2 flex items-center gap-2">
                          <ProgressBar supported={supported} total={total} />
                          <span className="font-mono text-[10px] text-muted-foreground">
                            {supported}/{total}
                          </span>
                        </div>
                      </div>
                      {isActive && (
                        <ChevronRight className="mt-2 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                      )}
                    </button>
                  </li>
                )
              })}
            </ul>
          </ScrollArea>
        </div>

        {/* Claim detail */}
        {selectedClaim ? (
          <ClaimDetail claim={selectedClaim} matter={matter} />
        ) : (
          <div className="flex items-center justify-center p-12 text-center">
            <p className="text-sm text-muted-foreground">No claim selected</p>
          </div>
        )}
      </div>
    </div>
  )
}

function ProgressBar({ supported, total }: { supported: number; total: number }) {
  const pct = total ? (supported / total) * 100 : 0
  return (
    <div className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
      <div
        className={cn(
          "absolute inset-y-0 left-0 transition-all",
          pct === 100 ? "bg-emerald-600" : pct >= 50 ? "bg-amber-500" : "bg-rose-500",
        )}
        style={{ width: `${pct}%` }}
      />
    </div>
  )
}

function KindBadge({ kind, small }: { kind: Claim["kind"]; small?: boolean }) {
  const meta = KIND_META[kind]
  const Icon = meta.icon
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-md border",
        meta.color,
        small ? "h-7 w-7" : "h-9 w-9",
      )}
    >
      <Icon className={small ? "h-3.5 w-3.5" : "h-4 w-4"} />
    </span>
  )
}

function ClaimDetail({ claim, matter }: { claim: Claim; matter: Matter }) {
  const meta = KIND_META[claim.kind]
  const supportedCount = claim.elements.filter((e) => e.status === "supported").length
  const allSupported = supportedCount === claim.elements.length

  return (
    <ScrollArea className="h-[calc(100vh-220px)]">
      <div className="mx-auto max-w-3xl px-6 py-6">
        {/* Header */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Badge variant="outline" className={cn("text-[10px] uppercase", meta.color)}>
              {meta.label}
            </Badge>
            <span className="font-mono text-[10px] text-muted-foreground">{claim.id}</span>
          </div>
          <h2 className="text-2xl font-semibold tracking-tight text-foreground text-balance">
            {claim.title}
          </h2>
          <p className="text-sm leading-relaxed text-muted-foreground">{claim.theory}</p>

          <div className="flex flex-wrap items-center gap-3 text-xs">
            <span className="flex items-center gap-1.5">
              <BookOpen className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="font-medium text-foreground">{claim.cause}</span>
            </span>
            <span className="text-muted-foreground">·</span>
            <span className="text-muted-foreground">
              Against:{" "}
              <span className="font-medium text-foreground">{claim.against}</span>
            </span>
          </div>

          <Card className="border-border/60 p-3">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs font-medium text-foreground">Element coverage</p>
                <p className="mt-0.5 text-[11px] text-muted-foreground">
                  {supportedCount} of {claim.elements.length} elements supported
                </p>
              </div>
              {allSupported ? (
                <Badge className="gap-1 bg-emerald-600/15 text-emerald-700 hover:bg-emerald-600/15 dark:text-emerald-300">
                  <CheckCircle2 className="h-3 w-3" />
                  Trial-ready
                </Badge>
              ) : (
                <Badge variant="outline" className="gap-1 border-amber-500/40 text-amber-700 dark:text-amber-400">
                  <AlertTriangle className="h-3 w-3" />
                  Has gaps
                </Badge>
              )}
            </div>
            <div className="mt-2">
              <ProgressBar supported={supportedCount} total={claim.elements.length} />
            </div>
          </Card>
        </div>

        {/* Elements */}
        <div className="mt-8">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold text-foreground">Elements</h3>
            <Button variant="ghost" size="sm" className="h-7 gap-1 text-[11px]">
              <Plus className="h-3 w-3" />
              Add element
            </Button>
          </div>

          <ol className="mt-3 space-y-3">
            {claim.elements.map((element, idx) => (
              <ElementCard key={element.id} element={element} index={idx + 1} matter={matter} />
            ))}
          </ol>
        </div>

        {/* Damages */}
        {claim.damages && claim.damages.length > 0 && (
          <div className="mt-8">
            <h3 className="text-sm font-semibold text-foreground">Damages</h3>
            <ul className="mt-3 space-y-2">
              {claim.damages.map((d, i) => (
                <li
                  key={i}
                  className="flex items-center justify-between rounded-md border border-border bg-card px-3 py-2 text-xs"
                >
                  <div>
                    <p className="font-medium text-foreground">{d.category}</p>
                    {d.theory && (
                      <p className="text-[11px] text-muted-foreground">{d.theory}</p>
                    )}
                  </div>
                  <span className="font-mono font-semibold text-foreground">{d.amount}</span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Counter-arguments */}
        {claim.counterArguments && claim.counterArguments.length > 0 && (
          <div className="mt-8">
            <h3 className="text-sm font-semibold text-foreground">Anticipated counter-arguments</h3>
            <ul className="mt-3 space-y-2">
              {claim.counterArguments.map((c, i) => (
                <li
                  key={i}
                  className="rounded-md border border-amber-500/30 bg-amber-500/5 p-3 text-xs"
                >
                  <p className="font-medium text-foreground">{c.argument}</p>
                  <p className="mt-1 leading-relaxed text-muted-foreground">
                    <span className="font-medium text-foreground">Response: </span>
                    {c.response}
                  </p>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </ScrollArea>
  )
}

function ElementCard({
  element,
  index,
  matter,
}: {
  element: ClaimElement
  index: number
  matter: Matter
}) {
  const supportingFacts = matter.facts.filter((f) =>
    element.supportingFactIds.includes(f.id),
  )

  const StatusIcon =
    element.status === "supported"
      ? CheckCircle2
      : element.status === "weak"
        ? AlertTriangle
        : element.status === "rebutted"
          ? XCircle
          : Circle

  const statusColor =
    element.status === "supported"
      ? "text-emerald-600 dark:text-emerald-400"
      : element.status === "weak"
        ? "text-amber-600 dark:text-amber-400"
        : element.status === "rebutted"
          ? "text-rose-600 dark:text-rose-400"
          : "text-muted-foreground"

  return (
    <li className="rounded-md border border-border bg-card">
      <div className="flex items-start gap-3 px-4 py-3">
        <StatusIcon className={cn("mt-0.5 h-4 w-4 shrink-0", statusColor)} />
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline justify-between gap-2">
            <h4 className="text-sm font-semibold text-foreground">
              <span className="mr-2 font-mono text-[10px] text-muted-foreground">{index}.</span>
              {element.title}
            </h4>
            <Badge variant="outline" className={cn("text-[10px] capitalize", statusColor)}>
              {element.status}
            </Badge>
          </div>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            {element.description}
          </p>
          {element.legalAuthority && (
            <p className="mt-2 font-mono text-[10px] text-muted-foreground">
              Authority: <span className="text-foreground">{element.legalAuthority}</span>
            </p>
          )}
        </div>
      </div>

      {supportingFacts.length > 0 && (
        <div className="border-t border-border bg-muted/20 px-4 py-2.5">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            {supportingFacts.length} supporting fact{supportingFacts.length === 1 ? "" : "s"}
          </p>
          <ul className="mt-1.5 space-y-1">
            {supportingFacts.map((fact) => (
              <li key={fact.id}>
                <Link
                  href={`/matters/${matter.id}/facts#${fact.id}`}
                  className="flex items-start justify-between gap-2 rounded px-1.5 py-1 text-[11px] hover:bg-background"
                >
                  <span className="line-clamp-1 text-foreground">{fact.statement}</span>
                  <ConfidenceBadge value={fact.confidence} size="sm" />
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}
    </li>
  )
}
