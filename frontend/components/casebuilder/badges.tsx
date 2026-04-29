import { cn } from "@/lib/utils"
import {
  AlertTriangle,
  CheckCircle2,
  CircleHelp,
  CircleSlash,
  FileQuestion,
  ShieldAlert,
  Sparkles,
  XCircle,
} from "lucide-react"
import type {
  ClaimStatus,
  DefenseStatus,
  EvidenceStrength,
  FactStatus,
  MatterStatus,
  ParagraphFactCheck,
  ProcessingStatus,
  RiskLevel,
  TaskPriority,
  TaskStatus,
} from "@/lib/casebuilder/types"

const BASE = "inline-flex items-center gap-1 rounded font-mono text-[10px] uppercase tracking-wide px-1.5 py-0.5"

export function FactStatusBadge({ status, className }: { status: FactStatus; className?: string }) {
  const map: Record<FactStatus, { label: string; cls: string; icon: typeof CheckCircle2 }> = {
    supported: { label: "supported", cls: "bg-success/15 text-success", icon: CheckCircle2 },
    alleged: { label: "alleged", cls: "bg-primary/15 text-primary", icon: Sparkles },
    disputed: { label: "disputed", cls: "bg-warning/15 text-warning", icon: AlertTriangle },
    admitted: { label: "admitted", cls: "bg-success/15 text-success", icon: CheckCircle2 },
    denied: { label: "denied", cls: "bg-destructive/15 text-destructive", icon: XCircle },
    unknown: { label: "unknown", cls: "bg-muted text-muted-foreground", icon: CircleHelp },
    contradicted: { label: "contradicted", cls: "bg-destructive/15 text-destructive", icon: XCircle },
    needs_evidence: { label: "needs evidence", cls: "bg-warning/15 text-warning", icon: FileQuestion },
  }
  const m = map[status]
  const Icon = m.icon
  return (
    <span className={cn(BASE, m.cls, className)}>
      <Icon className="h-3 w-3" />
      {m.label}
    </span>
  )
}

export function EvidenceStrengthBadge({
  strength,
  className,
}: {
  strength: EvidenceStrength
  className?: string
}) {
  const map: Record<EvidenceStrength, string> = {
    strong: "bg-success/15 text-success",
    moderate: "bg-primary/15 text-primary",
    weak: "bg-warning/15 text-warning",
    speculative: "bg-muted text-muted-foreground",
  }
  return <span className={cn(BASE, map[strength], className)}>{strength}</span>
}

export function RiskBadge({ level, className }: { level: RiskLevel; className?: string }) {
  const map: Record<RiskLevel, string> = {
    high: "bg-destructive/15 text-destructive",
    medium: "bg-warning/15 text-warning",
    low: "bg-success/15 text-success",
  }
  return <span className={cn(BASE, map[level], className)}>{level}</span>
}

export function ClaimStatusBadge({ status, className }: { status: ClaimStatus; className?: string }) {
  const map: Record<ClaimStatus, string> = {
    candidate: "bg-primary/15 text-primary",
    asserted: "bg-accent/20 text-accent",
    dismissed: "bg-muted text-muted-foreground",
    resolved: "bg-success/15 text-success",
    withdrawn: "bg-muted text-muted-foreground",
  }
  return <span className={cn(BASE, map[status], className)}>{status}</span>
}

export function DefenseStatusBadge({ status, className }: { status: DefenseStatus; className?: string }) {
  const map: Record<DefenseStatus, string> = {
    candidate: "bg-primary/15 text-primary",
    asserted: "bg-accent/20 text-accent",
    waived: "bg-muted text-muted-foreground",
    rejected: "bg-destructive/15 text-destructive",
  }
  return <span className={cn(BASE, map[status], className)}>{status}</span>
}

export function ProcessingBadge({
  status,
  className,
}: {
  status: ProcessingStatus
  className?: string
}) {
  const map: Record<ProcessingStatus, { label: string; cls: string }> = {
    queued: { label: "queued", cls: "bg-muted text-muted-foreground" },
    processing: { label: "processing", cls: "bg-primary/15 text-primary" },
    processed: { label: "processed", cls: "bg-success/15 text-success" },
    failed: { label: "failed", cls: "bg-destructive/15 text-destructive" },
  }
  return <span className={cn(BASE, map[status].cls, className)}>{map[status].label}</span>
}

export function MatterStatusBadge({ status, className }: { status: MatterStatus; className?: string }) {
  const map: Record<MatterStatus, string> = {
    active: "bg-success/15 text-success",
    intake: "bg-primary/15 text-primary",
    stayed: "bg-warning/15 text-warning",
    closed: "bg-muted text-muted-foreground",
    appeal: "bg-accent/20 text-accent",
  }
  return <span className={cn(BASE, map[status], className)}>{status}</span>
}

export function TaskStatusBadge({ status, className }: { status: TaskStatus; className?: string }) {
  const map: Record<TaskStatus, { label: string; cls: string; icon: typeof CheckCircle2 }> = {
    todo: { label: "todo", cls: "bg-muted text-muted-foreground", icon: CircleSlash },
    in_progress: { label: "in progress", cls: "bg-primary/15 text-primary", icon: Sparkles },
    blocked: { label: "blocked", cls: "bg-warning/15 text-warning", icon: ShieldAlert },
    done: { label: "done", cls: "bg-success/15 text-success", icon: CheckCircle2 },
  }
  const m = map[status]
  const Icon = m.icon
  return (
    <span className={cn(BASE, m.cls, className)}>
      <Icon className="h-3 w-3" />
      {m.label}
    </span>
  )
}

export function PriorityBadge({ priority, className }: { priority: TaskPriority; className?: string }) {
  const map: Record<TaskPriority, string> = {
    high: "bg-destructive/15 text-destructive",
    med: "bg-warning/15 text-warning",
    low: "bg-muted text-muted-foreground",
  }
  return <span className={cn(BASE, map[priority], className)}>{priority}</span>
}

export function ParaCheckBadge({
  status,
  className,
}: {
  status: ParagraphFactCheck
  className?: string
}) {
  const map: Record<ParagraphFactCheck, { label: string; cls: string; icon: typeof CheckCircle2 }> = {
    supported: { label: "supported", cls: "bg-success/15 text-success", icon: CheckCircle2 },
    needs_evidence: { label: "needs evidence", cls: "bg-warning/15 text-warning", icon: FileQuestion },
    needs_authority: { label: "needs authority", cls: "bg-warning/15 text-warning", icon: ShieldAlert },
    contradicted: { label: "contradicted", cls: "bg-destructive/15 text-destructive", icon: XCircle },
    citation_issue: { label: "citation issue", cls: "bg-warning/15 text-warning", icon: AlertTriangle },
    deadline_warning: { label: "deadline warning", cls: "bg-warning/15 text-warning", icon: AlertTriangle },
    unchecked: { label: "unchecked", cls: "bg-muted text-muted-foreground", icon: CircleHelp },
  }
  const m = map[status]
  const Icon = m.icon
  return (
    <span className={cn(BASE, m.cls, className)}>
      <Icon className="h-3 w-3" />
      {m.label}
    </span>
  )
}

export function ConfidenceBar({ value, className }: { value: number; className?: string }) {
  const pct = Math.round(value * 100)
  const tone = value >= 0.85 ? "bg-success" : value >= 0.7 ? "bg-primary" : value >= 0.5 ? "bg-warning" : "bg-destructive"
  return (
    <div className={cn("flex items-center gap-1.5", className)}>
      <div className="h-1 w-12 overflow-hidden rounded bg-border">
        <div className={cn("h-full", tone)} style={{ width: `${pct}%` }} />
      </div>
      <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{pct}%</span>
    </div>
  )
}

export function ConfidenceBadge({
  value,
  size = "md",
  className,
}: {
  value: number
  size?: "sm" | "md"
  className?: string
}) {
  const pct = Math.round(value * 100)
  const tone =
    value >= 0.85
      ? "bg-success/15 text-success"
      : value >= 0.7
        ? "bg-primary/15 text-primary"
        : value >= 0.5
          ? "bg-warning/15 text-warning"
          : "bg-destructive/15 text-destructive"

  return (
    <span
      className={cn(
        BASE,
        tone,
        size === "sm" && "px-1 py-0 text-[9px]",
        className,
      )}
    >
      {pct}%
    </span>
  )
}
