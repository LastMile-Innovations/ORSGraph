"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useMemo, useState } from "react"
import type { LucideIcon } from "lucide-react"
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  FileText,
  GavelIcon,
  Layers3,
  NotebookText,
  PenLine,
  Plus,
  Scale,
  ScrollText,
  Search,
  ShieldCheck,
} from "lucide-react"
import type { Matter, WorkProduct } from "@/lib/casebuilder/types"
import type { CreateWorkProductInput } from "@/lib/casebuilder/api"
import { createWorkProduct } from "@/lib/casebuilder/api"
import { matterComplaintHref, matterWorkProductHref, newWorkProductHref } from "@/lib/casebuilder/routes"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"

interface WorkProductDashboardProps {
  matter: Matter
  workProducts: WorkProduct[]
  initialCreate?: boolean
  initialProductType?: string
}

interface WorkProductTemplate {
  id: string
  title: string
  productType: string
  description: string
  icon: LucideIcon
}

const WORK_PRODUCT_TEMPLATES: WorkProductTemplate[] = [
  {
    id: "complaint-oregon-civil",
    title: "Complaint",
    productType: "complaint",
    description: "Caption, numbered allegations, counts, relief, QC, preview, and export state.",
    icon: GavelIcon,
  },
  {
    id: "answer-response-grid",
    title: "Answer",
    productType: "answer",
    description: "Responses, affirmative defenses, counterclaims, and prayer for relief.",
    icon: Scale,
  },
  {
    id: "motion-standard",
    title: "Motion",
    productType: "motion",
    description: "Relief requested, facts, argument, declarations, and proposed order hooks.",
    icon: ScrollText,
  },
  {
    id: "declaration-support",
    title: "Declaration",
    productType: "declaration",
    description: "Declarant identity, personal-knowledge facts, exhibits, and signature block.",
    icon: PenLine,
  },
  {
    id: "legal-memo",
    title: "Legal memo",
    productType: "legal_memo",
    description: "Question presented, facts, analysis, authority, and recommended next step.",
    icon: NotebookText,
  },
  {
    id: "demand-letter",
    title: "Demand letter",
    productType: "demand_letter",
    description: "Narrative facts, legal basis, requested cure, deadline, and attachment list.",
    icon: FileText,
  },
  {
    id: "notice",
    title: "Notice",
    productType: "notice",
    description: "Recipient, legal basis, required content, proof notes, and service details.",
    icon: AlertTriangle,
  },
  {
    id: "exhibit-list",
    title: "Exhibit list",
    productType: "exhibit_list",
    description: "Stable exhibit labels, source files, foundation notes, and packet order.",
    icon: Layers3,
  },
  {
    id: "proposed-order",
    title: "Proposed order",
    productType: "proposed_order",
    description: "Findings, relief ordered, signature area, and service/circulation notes.",
    icon: ShieldCheck,
  },
]

export function WorkProductDashboard({
  matter,
  workProducts,
  initialCreate = false,
  initialProductType,
}: WorkProductDashboardProps) {
  const router = useRouter()
  const [query, setQuery] = useState("")
  const [showCreate, setShowCreate] = useState(initialCreate)
  const [title, setTitle] = useState("")
  const [productType, setProductType] = useState(initialProductType || "motion")
  const [templateId, setTemplateId] = useState(templateForType(initialProductType || "motion")?.id ?? "motion-standard")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return workProducts
    return workProducts.filter((product) =>
      [product.title, product.product_type, product.status, product.review_status]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(needle)),
    )
  }, [query, workProducts])

  const sortedProducts = useMemo(
    () =>
      [...filtered].sort((a, b) =>
        (b.updated_at || b.created_at || "").localeCompare(a.updated_at || a.created_at || ""),
      ),
    [filtered],
  )

  const existingByType = useMemo(() => {
    const map = new Map<string, WorkProduct>()
    for (const product of workProducts) {
      if (!map.has(product.product_type)) map.set(product.product_type, product)
    }
    return map
  }, [workProducts])

  async function createAndOpen(input: CreateWorkProductInput) {
    setSaving(true)
    setError(null)
    const created = await createWorkProduct(matter.id, input)
    setSaving(false)
    if (!created.data) {
      setError(created.error || "Work product could not be created.")
      return
    }
    router.push(matterWorkProductHref(matter.id, created.data.id, "editor"))
    router.refresh()
  }

  const handleManualCreate = () => {
    const selectedTemplate = templateForId(templateId)
    createAndOpen({
      title: title.trim() || `${selectedTemplate?.title ?? labelForType(productType)} - ${matter.shortName || matter.name}`,
      product_type: productType,
      template: templateId,
    })
  }

  const handleTemplateCreate = (template: WorkProductTemplate) => {
    createAndOpen({
      title: `${template.title} - ${matter.shortName || matter.name}`,
      product_type: template.productType,
      template: template.id,
    })
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="border-b border-border bg-background px-6 py-5">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <FileText className="h-3.5 w-3.5 text-primary" />
              shared builder
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Work Product</h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              Shared AST-backed documents, drafts, filings, checks, exports, and history for this matter.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button asChild variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Link href={matterComplaintHref(matter.id, "editor")}>
                <GavelIcon className="h-3.5 w-3.5" />
                Complaint
              </Link>
            </Button>
            <Button size="sm" className="gap-1.5" onClick={() => setShowCreate((value) => !value)}>
              <Plus className="h-3.5 w-3.5" />
              New
            </Button>
          </div>
        </div>

        <div className="mt-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="relative max-w-sm flex-1">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search work product..."
              className="h-8 pl-8 text-xs"
            />
          </div>
          <div className="flex flex-wrap gap-2 text-[11px] text-muted-foreground">
            <SummaryPill label="Documents" value={workProducts.length} />
            <SummaryPill label="Open QC" value={workProducts.reduce((sum, product) => sum + product.findings.filter((finding) => finding.status === "open").length, 0)} />
            <SummaryPill label="Exports" value={workProducts.reduce((sum, product) => sum + product.artifacts.length, 0)} />
          </div>
        </div>

        {showCreate && (
          <div className="mt-4 grid gap-2 rounded-md border border-border bg-card p-3 md:grid-cols-[minmax(0,1fr)_170px_220px_auto]">
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="Title"
              className="rounded-md border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
            />
            <select
              value={productType}
              onChange={(event) => {
                setProductType(event.target.value)
                setTemplateId(templateForType(event.target.value)?.id ?? event.target.value)
              }}
              className="rounded-md border border-border bg-background px-3 py-2 font-mono text-xs"
            >
              {WORK_PRODUCT_TEMPLATES.map((template) => (
                <option key={template.productType} value={template.productType}>
                  {template.productType}
                </option>
              ))}
            </select>
            <select
              value={templateId}
              onChange={(event) => setTemplateId(event.target.value)}
              className="rounded-md border border-border bg-background px-3 py-2 text-xs"
            >
              {WORK_PRODUCT_TEMPLATES.map((template) => (
                <option key={template.id} value={template.id}>
                  {template.title}
                </option>
              ))}
            </select>
            <Button size="sm" disabled={saving} onClick={handleManualCreate}>
              {saving ? "Creating" : "Create"}
            </Button>
            {error && <p className="text-xs text-destructive md:col-span-4">{error}</p>}
          </div>
        )}
      </header>

      <ScrollArea className="flex-1">
        <main className="grid gap-6 px-6 py-6 xl:grid-cols-[minmax(0,1fr)_360px]">
          <section className="min-w-0">
            <div className="mb-3 flex items-center justify-between gap-3">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Matter work product
              </h2>
              <Button asChild variant="ghost" size="sm" className="h-7 gap-1.5 text-xs">
                <Link href={newWorkProductHref(matter.id)}>
                  <Plus className="h-3.5 w-3.5" />
                  Add
                </Link>
              </Button>
            </div>

            {sortedProducts.length === 0 ? (
              <div className="rounded-md border border-dashed border-border bg-card p-6">
                <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <FileText className="h-4 w-4 text-muted-foreground" />
                  No work product yet
                </div>
                <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
                  Start with a complaint, answer, motion, declaration, memo, or filing support document.
                </p>
              </div>
            ) : (
              <ul className="grid grid-cols-1 gap-2 lg:grid-cols-2">
                {sortedProducts.map((product) => (
                  <li key={product.id}>
                    <WorkProductCard matter={matter} product={product} />
                  </li>
                ))}
              </ul>
            )}
          </section>

          <aside className="min-w-0">
            <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Templates
            </h2>
            <ul className="grid grid-cols-1 gap-2">
              {WORK_PRODUCT_TEMPLATES.map((template) => {
                const existing = existingByType.get(template.productType)
                const Icon = template.icon
                return (
                  <li key={template.id}>
                    <Card className="p-3">
                      <div className="flex items-start gap-3">
                        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
                          <Icon className="h-4 w-4" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-start justify-between gap-2">
                            <h3 className="text-sm font-semibold text-foreground">{template.title}</h3>
                            {existing && (
                              <Badge variant="outline" className="text-[10px]">
                                existing
                              </Badge>
                            )}
                          </div>
                          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{template.description}</p>
                          <div className="mt-3 flex flex-wrap gap-2">
                            {existing && (
                              <Button asChild variant="outline" size="sm" className="h-7 bg-transparent text-xs">
                                <Link href={matterWorkProductHref(matter.id, existing.id)}>Open</Link>
                              </Button>
                            )}
                            <Button
                              variant={existing ? "ghost" : "secondary"}
                              size="sm"
                              className="h-7 text-xs"
                              disabled={saving}
                              onClick={() => handleTemplateCreate(template)}
                            >
                              Create
                            </Button>
                          </div>
                        </div>
                      </div>
                    </Card>
                  </li>
                )
              })}
            </ul>
          </aside>
        </main>
      </ScrollArea>
    </div>
  )
}

function WorkProductCard({ matter, product }: { matter: Matter; product: WorkProduct }) {
  const openFindings = product.findings.filter((finding) => finding.status === "open")
  const blocks = product.blocks.length || product.document_ast.blocks.length
  const latestArtifact = product.artifacts[0]

  return (
    <Link href={matterWorkProductHref(matter.id, product.id)} className="group block h-full">
      <Card className="h-full p-4 transition-colors group-hover:border-foreground/30 group-hover:bg-muted/30">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline" className="text-[10px] capitalize">
                {labelForType(product.product_type)}
              </Badge>
              <StatusBadge status={product.status} />
              {product.review_status && (
                <span className="rounded bg-warning/10 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-warning">
                  {product.review_status.replace(/_/g, " ")}
                </span>
              )}
            </div>
            <h3 className="mt-3 line-clamp-2 text-sm font-semibold leading-tight text-foreground">
              {product.title}
            </h3>
          </div>
          <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
        </div>

        <div className="mt-4 grid grid-cols-3 gap-2 text-[11px]">
          <Metric label="Blocks" value={blocks} />
          <Metric label="QC" value={openFindings.length} warn={openFindings.length > 0} />
          <Metric label="Exports" value={product.artifacts.length} />
        </div>

        <div className="mt-4 flex flex-wrap items-center justify-between gap-2 text-[10px] text-muted-foreground">
          <span className="font-mono tabular-nums">
            {product.updated_at || product.created_at || "not saved"}
          </span>
          {latestArtifact ? (
            <span className="font-mono uppercase tracking-wide">{latestArtifact.format} export</span>
          ) : (
            <span className="font-mono uppercase tracking-wide">no export</span>
          )}
        </div>
      </Card>
    </Link>
  )
}

function StatusBadge({ status }: { status: string }) {
  const done = ["final", "filed", "served"].includes(status)
  const Icon = done ? CheckCircle2 : status === "review" ? Clock : FileText
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        done && "bg-success/15 text-success",
        status === "review" && "bg-warning/15 text-warning",
        !done && status !== "review" && "bg-muted text-muted-foreground",
      )}
    >
      <Icon className="h-2.5 w-2.5" />
      {status}
    </span>
  )
}

function Metric({ label, value, warn = false }: { label: string; value: number; warn?: boolean }) {
  return (
    <div className="rounded-md border border-border bg-background px-2 py-1.5">
      <div className={cn("font-mono text-sm tabular-nums", warn ? "text-warning" : "text-foreground")}>
        {value}
      </div>
      <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div>
    </div>
  )
}

function SummaryPill({ label, value }: { label: string; value: number }) {
  return (
    <span className="rounded-md border border-border bg-card px-2 py-1">
      <span className="font-mono tabular-nums text-foreground">{value}</span> {label}
    </span>
  )
}

function templateForType(productType: string) {
  return WORK_PRODUCT_TEMPLATES.find((template) => template.productType === productType)
}

function templateForId(templateId: string) {
  return WORK_PRODUCT_TEMPLATES.find((template) => template.id === templateId)
}

function labelForType(value: string) {
  return value.replace(/_/g, " ")
}
