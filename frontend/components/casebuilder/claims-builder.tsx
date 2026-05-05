"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
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
import { matterFactsHref } from "@/lib/casebuilder/routes"
import { getMatterReadiness } from "@/lib/casebuilder/readiness"
import { createClaim, mapClaimElements } from "@/lib/casebuilder/api"
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
    color: "text-case-claim border-case-claim/40 bg-case-claim/10",
  },
  counterclaim: {
    label: "Counterclaim",
    icon: Gavel,
    color: "text-case-counterclaim border-case-counterclaim/40 bg-case-counterclaim/10",
  },
  defense: {
    label: "Affirmative Defense",
    icon: Shield,
    color: "text-case-defense border-case-defense/40 bg-case-defense/10",
  },
}

export function ClaimsBuilder({ matter }: ClaimsBuilderProps) {
  const router = useRouter()
  const [tab, setTab] = useState<Claim["kind"] | "all">("all")
  const [selected, setSelected] = useState<string | null>(matter.claims[0]?.id ?? null)
  const [showCreate, setShowCreate] = useState(false)
  const [kind, setKind] = useState<Claim["kind"]>("claim")
  const [title, setTitle] = useState("")
  const [claimType, setClaimType] = useState("custom")
  const [legalTheory, setLegalTheory] = useState("")
  const [elementLines, setElementLines] = useState("")
  const [supportingFactId, setSupportingFactId] = useState("")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const readiness = useMemo(() => getMatterReadiness(matter), [matter])

  const filteredClaims =
    tab === "all" ? matter.claims : matter.claims.filter((c) => c.kind === tab)
  const selectedClaim = matter.claims.find((c) => c.id === selected) ?? filteredClaims[0]

  async function onCreateClaim() {
    if (!title.trim()) {
      setError("Add a title for the claim.")
      return
    }
    setSaving(true)
    setError(null)
    const elements = elementLines
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((text) => ({ text }))
    const result = await createClaim(matter.id, {
      kind,
      title: title.trim(),
      claim_type: claimType.trim() || "custom",
      legal_theory: legalTheory.trim(),
      fact_ids: supportingFactId ? [supportingFactId] : [],
      elements,
    })
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Claim could not be created.")
      return
    }
    setSelected(result.data.id)
    setShowCreate(false)
    setKind("claim")
    setTitle("")
    setClaimType("custom")
    setLegalTheory("")
    setElementLines("")
    setSupportingFactId("")
    router.refresh()
  }

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
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 bg-transparent"
              disabled
              title="AI claim suggestion is in limited beta; create or review claims manually for now."
            >
              <Sparkles className="h-3.5 w-3.5" />
              Suggest claims (beta)
            </Button>
            <Button size="sm" className="gap-1.5" onClick={() => setShowCreate((value) => !value)}>
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

        {matter.claims.length === 0 && <ClaimsSetupBanner matter={matter} readiness={readiness} />}

        {showCreate && (
          <div className="mt-4 grid gap-3 rounded-md border border-border bg-card p-3 md:grid-cols-[160px_minmax(0,1fr)_180px]">
            <select
              value={kind}
              onChange={(event) => setKind(event.target.value as Claim["kind"])}
              className="rounded border border-border bg-background px-3 py-2 font-mono text-xs"
            >
              <option value="claim">claim</option>
              <option value="counterclaim">counterclaim</option>
              <option value="defense">defense</option>
            </select>
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="Claim title"
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
            />
            <input
              value={claimType}
              onChange={(event) => setClaimType(event.target.value)}
              placeholder="claim type"
              className="rounded border border-border bg-background px-3 py-2 font-mono text-xs focus:border-primary focus:outline-none"
            />
            <textarea
              value={legalTheory}
              onChange={(event) => setLegalTheory(event.target.value)}
              placeholder="Legal theory"
              rows={3}
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none md:col-span-3"
            />
            <textarea
              value={elementLines}
              onChange={(event) => setElementLines(event.target.value)}
              placeholder="One required element per line"
              rows={4}
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none md:col-span-2"
            />
            <select
              value={supportingFactId}
              onChange={(event) => setSupportingFactId(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 text-xs"
            >
              <option value="">No starting fact</option>
              {matter.facts.map((fact) => (
                <option key={fact.id} value={fact.id}>
                  {fact.statement.slice(0, 80)}
                </option>
              ))}
            </select>
            <div className="flex items-center justify-between gap-3 md:col-span-3">
              <p className="text-xs text-destructive">{error}</p>
              <Button size="sm" onClick={onCreateClaim} disabled={saving}>
                {saving ? "Saving" : "Create claim"}
              </Button>
            </div>
          </div>
        )}
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
          <ClaimsEmptyState matter={matter} readiness={readiness} />
        )}
      </div>
    </div>
  )
}

function ClaimsSetupBanner({ matter, readiness }: { matter: Matter; readiness: ReturnType<typeof getMatterReadiness> }) {
  return (
    <div className="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
      <span className="font-medium text-foreground">Claims are not created yet.</span>{" "}
      Review {readiness.reviewFacts} extracted fact{readiness.reviewFacts === 1 ? "" : "s"} and {matter.timeline_suggestions.length} timeline suggestion{matter.timeline_suggestions.length === 1 ? "" : "s"} before building legal theories.
    </div>
  )
}

function ClaimsEmptyState({ matter, readiness }: { matter: Matter; readiness: ReturnType<typeof getMatterReadiness> }) {
  return (
    <div className="flex items-center justify-center p-8">
      <Card className="max-w-xl border-dashed bg-transparent p-6 text-center">
        <Scale className="mx-auto h-8 w-8 text-muted-foreground" />
        <h2 className="mt-3 text-base font-semibold text-foreground">No claims or defenses yet</h2>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          This matter has {matter.facts.length} extracted fact{matter.facts.length === 1 ? "" : "s"} and {readiness.pendingTimelineSuggestions} timeline suggestion{readiness.pendingTimelineSuggestions === 1 ? "" : "s"}. Create a claim when the facts are reviewed enough to support a legal theory.
        </p>
        <div className="mt-4 flex flex-wrap justify-center gap-2">
          <Button asChild variant="outline" size="sm" className="bg-transparent">
            <Link href={matterFactsHref(matter.id)}>Review facts</Link>
          </Button>
        </div>
      </Card>
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
          pct === 100 ? "bg-success" : pct >= 50 ? "bg-warning" : "bg-destructive",
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
  const router = useRouter()
  const meta = KIND_META[claim.kind]
  const supportedCount = claim.elements.filter((e) => e.status === "supported").length
  const allSupported = supportedCount === claim.elements.length
  const [mapping, setMapping] = useState(false)
  const [mapError, setMapError] = useState<string | null>(null)

  async function onMapElements() {
    setMapping(true)
    setMapError(null)
    const result = await mapClaimElements(matter.id, claim.id)
    setMapping(false)
    if (!result.data) {
      setMapError(result.error || "Element mapping failed.")
      return
    }
    router.refresh()
  }

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
                <Badge className="gap-1 bg-success/15 text-success hover:bg-success/15">
                  <CheckCircle2 className="h-3 w-3" />
                  Trial-ready
                </Badge>
              ) : (
                <Badge variant="outline" className="gap-1 border-warning/40 text-warning">
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
            <Button variant="ghost" size="sm" className="h-7 gap-1 text-[11px]" onClick={onMapElements} disabled={mapping}>
              <Sparkles className="h-3 w-3" />
              {mapping ? "Mapping" : "Map elements"}
            </Button>
          </div>
          {mapError && <p className="mt-2 text-xs text-destructive">{mapError}</p>}

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
                  className="rounded-md border border-warning/30 bg-warning/5 p-3 text-xs"
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
      ? "text-success"
      : element.status === "weak"
        ? "text-warning"
        : element.status === "rebutted"
          ? "text-destructive"
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
                  href={matterFactsHref(matter.id, fact.id)}
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
