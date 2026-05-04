import type { LegalStatus, ChunkType, ProvisionSignal } from "@/lib/types"
import { cn } from "@/lib/utils"
import {
  CheckCircle2,
  Quote,
  Hash,
  Scale,
  Clock,
  ShieldAlert,
  Type,
  GitBranch,
} from "lucide-react"

type BadgeProps = {
  className?: string
}

export function StatusBadge({ status, className }: BadgeProps & { status: LegalStatus }) {
  const map: Record<LegalStatus, string> = {
    active: "bg-success/15 text-success",
    repealed: "bg-destructive/15 text-destructive",
    renumbered: "bg-warning/15 text-warning",
    amended: "bg-accent/20 text-accent-foreground",
  }
  return (
    <span
      className={cn(
        "inline-flex items-center rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        map[status],
        className,
      )}
    >
      {status}
    </span>
  )
}

export function CitationBadge({
  citation,
  className,
  href,
}: {
  citation: string
  className?: string
  href?: string
}) {
  const Tag = href ? "a" : "span"
  return (
    <Tag
      href={href}
      className={cn(
        "inline-flex items-center gap-1 rounded border border-border bg-muted/40 px-1.5 py-0.5 font-mono text-xs text-foreground hover:border-primary hover:text-primary",
        className,
      )}
    >
      <Hash className="h-3 w-3 text-muted-foreground" />
      {citation}
    </Tag>
  )
}

const SIGNAL_META: Record<ProvisionSignal, { label: string; icon: typeof Type; cls: string }> = {
  definition: { label: "def", icon: Type, cls: "bg-chart-1/15 text-chart-1" },
  exception: { label: "exc", icon: ShieldAlert, cls: "bg-warning/15 text-warning" },
  deadline: { label: "deadline", icon: Clock, cls: "bg-chart-3/20 text-chart-3" },
  penalty: { label: "penalty", icon: Scale, cls: "bg-destructive/15 text-destructive" },
  citation: { label: "cite", icon: Quote, cls: "bg-accent/20 text-accent" },
}

export function SignalBadge({ signal, className }: BadgeProps & { signal: ProvisionSignal }) {
  const m = SIGNAL_META[signal]
  const Icon = m.icon
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        m.cls,
        className,
      )}
    >
      <Icon className="h-3 w-3" />
      {m.label}
    </span>
  )
}

const CHUNK_META: Record<ChunkType, { label: string; cls: string }> = {
  full_statute: { label: "full statute", cls: "bg-primary/15 text-primary" },
  contextual_provision: { label: "ctx provision", cls: "bg-chart-1/15 text-chart-1" },
  definition_block: { label: "definition", cls: "bg-chart-1/15 text-chart-1" },
  exception_block: { label: "exception", cls: "bg-warning/15 text-warning" },
  deadline_block: { label: "deadline", cls: "bg-chart-3/20 text-chart-3" },
  penalty_block: { label: "penalty", cls: "bg-destructive/15 text-destructive" },
  citation_context: { label: "citation ctx", cls: "bg-accent/20 text-accent" },
}

export function ChunkTypeBadge({ type, className }: BadgeProps & { type: ChunkType }) {
  const m = CHUNK_META[type]
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        m.cls,
        className,
      )}
    >
      <GitBranch className="h-3 w-3" />
      {m.label}
    </span>
  )
}

const SEMANTIC_META: Record<string, { label: string; icon: typeof Type; cls: string }> = {
  obligation: { label: "obligation", icon: Scale, cls: "bg-chart-1/15 text-chart-1" },
  definition: { label: "definition", icon: Type, cls: "bg-chart-1/15 text-chart-1" },
  exception: { label: "exception", icon: ShieldAlert, cls: "bg-warning/15 text-warning" },
  deadline: { label: "deadline", icon: Clock, cls: "bg-chart-3/20 text-chart-3" },
  penalty: { label: "penalty", icon: ShieldAlert, cls: "bg-destructive/15 text-destructive" },
  remedy: { label: "remedy", icon: CheckCircle2, cls: "bg-success/15 text-success" },
  notice: { label: "notice", icon: Quote, cls: "bg-accent/20 text-accent" },
}

export function SemanticBadge({ type, className }: BadgeProps & { type: string }) {
  const m = SEMANTIC_META[type.toLowerCase()] || { label: type, icon: Type, cls: "bg-muted text-muted-foreground" }
  const Icon = m.icon
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        m.cls,
        className,
      )}
    >
      <Icon className="h-3 w-3" />
      {m.label}
    </span>
  )
}

export function SourceBadge({ className }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded bg-primary/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary",
        className,
      )}
    >
      <CheckCircle2 className="h-3 w-3" />
      source-backed
    </span>
  )
}

export function TrustBadge({
  level,
  className,
}: BadgeProps & {
  level: "official" | "parsed" | "extracted" | "generated" | "user_draft"
}) {
  const map = {
    official: { label: "official source", cls: "bg-primary/15 text-primary" },
    parsed: { label: "parsed", cls: "bg-chart-1/15 text-chart-1" },
    extracted: { label: "extracted", cls: "bg-accent/20 text-accent" },
    generated: { label: "generated", cls: "bg-warning/15 text-warning" },
    user_draft: { label: "user draft", cls: "bg-muted text-muted-foreground" },
  }
  const m = map[level]
  return (
    <span
      className={cn(
        "inline-flex items-center rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        m.cls,
        className,
      )}
    >
      {m.label}
    </span>
  )
}
