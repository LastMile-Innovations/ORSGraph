"use client"

import Image from "next/image"
import Link from "next/link"
import { useSession } from "next-auth/react"
import {
  ArrowRight,
  BookMarked,
  BriefcaseBusiness,
  CheckCircle2,
  FileCheck2,
  FileSearch,
  GitBranch,
  Landmark,
  Layers3,
  LockKeyhole,
  Network,
  SearchCheck,
  ShieldCheck,
  Sparkles,
  UploadCloud,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { trackConversionEvent } from "@/lib/conversion-events"

const proofPoints = [
  { value: "52K+", label: "graph-ready statutory and authority records" },
  { value: "Source-first", label: "facts, claims, citations, and exhibits stay traceable" },
  { value: "Matter-native", label: "intake, drafting, QC, and export share one workspace" },
]

const operatingLanes = [
  {
    icon: SearchCheck,
    title: "Research",
    body: "Search, ask, and inspect controlling authority with citation paths still attached.",
  },
  {
    icon: UploadCloud,
    title: "Intake",
    body: "Bring in files and folders while preserving the case structure users already have.",
  },
  {
    icon: BriefcaseBusiness,
    title: "Build",
    body: "Turn documents into parties, facts, deadlines, claims, and work product.",
  },
  {
    icon: FileCheck2,
    title: "Review",
    body: "QC the source trail before a draft becomes a filing packet.",
  },
]

const workflow = [
  { label: "Upload", detail: "documents and folders", icon: UploadCloud },
  { label: "Extract", detail: "facts and deadlines", icon: Layers3 },
  { label: "Connect", detail: "authority and evidence", icon: Network },
  { label: "Draft", detail: "complaints, answers, motions", icon: FileSearch },
  { label: "File", detail: "QC and export", icon: ShieldCheck },
]

export function MarketingLanding() {
  const session = useSession()
  const isSignedIn = session.status === "authenticated"
  const primaryHref = isSignedIn ? "/onboarding" : "/auth/request-access"
  const inviteHref = "/auth/signin?callbackUrl=%2Fonboarding"

  return (
    <main className="min-h-screen overflow-x-hidden bg-background text-foreground">
      <section className="relative min-h-[84svh] overflow-hidden border-b border-hero-border text-hero-foreground">
        <Image
          src="/marketing/legal-os-hero.png"
          alt="ORSGraph legal operations workspace with documents, citation graph, and case timeline"
          fill
          preload
          quality={90}
          sizes="100vw"
          className="object-cover"
        />
        <div className="absolute inset-0 bg-[linear-gradient(90deg,var(--hero-overlay-strong)_0%,var(--hero-overlay-medium)_48%,var(--hero-overlay-soft)_100%)]" />
        <div className="absolute inset-x-0 top-0 z-30 border-b border-hero-border bg-hero-panel backdrop-blur">
          <header className="mx-auto flex h-16 max-w-7xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-hero-accent text-hero-accent-foreground shadow-lg shadow-black/20">
                <Landmark className="h-5 w-5" strokeWidth={2.4} />
              </span>
              <div className="min-w-0">
                <div className="font-mono text-sm font-semibold tracking-normal text-hero-foreground">ORSGraph</div>
                <div className="hidden font-mono text-[10px] uppercase tracking-normal text-hero-muted sm:block">
                  Source-first legal OS
                </div>
              </div>
            </div>

            <nav className="hidden items-center gap-1 text-sm font-medium text-hero-muted md:flex">
              <Link href="#platform" scroll={false} className="rounded-md px-3 py-2 outline-none hover:bg-hero-foreground/10 hover:text-hero-foreground focus-visible:ring-2 focus-visible:ring-hero-accent/70">
                Platform
              </Link>
              <Link href="#workflow" scroll={false} className="rounded-md px-3 py-2 outline-none hover:bg-hero-foreground/10 hover:text-hero-foreground focus-visible:ring-2 focus-visible:ring-hero-accent/70">
                Workflow
              </Link>
            </nav>

            <div className="flex shrink-0 items-center gap-2">
              {!isSignedIn && (
                <Button
                  asChild
                  variant="outline"
                  className="hidden h-9 rounded-md border-hero-border bg-hero-foreground/5 px-4 text-hero-foreground hover:bg-hero-foreground/10 sm:inline-flex"
                >
                  <Link href={inviteHref} onClick={() => trackConversionEvent("sign_in_started", { source: "landing" })}>
                    I Have An Invite
                  </Link>
                </Button>
              )}
              <Button
                asChild
                className="h-9 rounded-md bg-hero-accent px-4 text-hero-accent-foreground hover:bg-hero-accent/85"
              >
                <Link
                  href={primaryHref}
                  onClick={() => {
                    if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                  }}
                >
                  {isSignedIn ? "Enter App" : "Request Access"}
                </Link>
              </Button>
            </div>
          </header>
        </div>

        <div className="relative z-10 mx-auto flex min-h-[84svh] max-w-7xl items-end px-4 pb-10 pt-28 sm:px-6 sm:pb-14 lg:px-8">
          <div className="grid w-full gap-8 lg:grid-cols-[minmax(0,0.94fr)_26rem] lg:items-end">
            <div className="max-w-4xl">
              <div className="mb-5 inline-flex max-w-full min-w-0 items-center gap-2 rounded-md border border-hero-accent/30 bg-hero-panel px-3 py-1.5 text-[11px] font-semibold uppercase tracking-normal text-hero-accent backdrop-blur">
                <Sparkles className="h-3.5 w-3.5 shrink-0" />
                <span className="min-w-0 truncate">Authority, evidence, and drafting in one source trail</span>
              </div>

              <h1 className="max-w-4xl text-balance text-4xl font-semibold tracking-normal text-hero-foreground sm:text-5xl lg:text-6xl">
                ORSGraph is the legal workspace where every answer keeps its receipts.
              </h1>

              <p className="mt-5 max-w-2xl text-pretty break-words text-base leading-7 text-hero-muted sm:text-lg">
                Research Oregon law, structure matters, connect evidence to claims, and move drafts through QC without losing the source trail.
              </p>

              <div className="mt-8 flex flex-col gap-3 sm:flex-row">
                <Button
                  asChild
                  className="min-h-11 w-full rounded-md bg-hero-accent px-5 text-hero-accent-foreground hover:bg-hero-accent/85 sm:w-auto"
                >
                  <Link
                    href={primaryHref}
                    onClick={() => {
                      if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                    }}
                  >
                    {isSignedIn ? "Create First Matter" : "Request Beta Access"}
                    <ArrowRight className="h-4 w-4" />
                  </Link>
                </Button>
                <Link
                  href={isSignedIn ? "/dashboard" : inviteHref}
                  onClick={() => {
                    if (!isSignedIn) trackConversionEvent("sign_in_started", { source: "landing" })
                  }}
                  className="inline-flex min-h-11 w-full items-center justify-center gap-2 rounded-md border border-hero-border bg-hero-panel px-5 text-sm font-medium text-hero-foreground outline-none backdrop-blur transition hover:border-hero-accent/70 hover:bg-hero-foreground/10 focus-visible:ring-2 focus-visible:ring-hero-accent/70 sm:w-auto"
                >
                  {isSignedIn ? "Open Dashboard" : "I Have An Invite"}
                  <GitBranch className="h-4 w-4" />
                </Link>
              </div>
            </div>

            <div className="hidden rounded-md border border-hero-border bg-hero-panel p-4 shadow-2xl shadow-black/30 backdrop-blur lg:block">
              <div className="flex items-center justify-between gap-3 border-b border-hero-border pb-3">
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-normal text-hero-accent">Live matter flow</div>
                  <div className="mt-1 text-sm font-semibold text-hero-foreground">Source pack complete</div>
                </div>
                <span className="rounded-md bg-success/15 px-2 py-1 font-mono text-[10px] uppercase tracking-normal text-success">
                  QC ready
                </span>
              </div>
              <div className="mt-4 space-y-3">
                {workflow.map((item, index) => {
                  const Icon = item.icon
                  return (
                    <div key={item.label} className="flex items-center gap-3 rounded-md border border-hero-border bg-hero-foreground/[0.04] px-3 py-2.5">
                      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-hero-accent/15 text-hero-accent">
                        <Icon className="h-4 w-4" />
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center justify-between gap-3">
                          <span className="text-sm font-medium text-hero-foreground">{item.label}</span>
                          <span className="font-mono text-[10px] text-hero-muted">{String(index + 1).padStart(2, "0")}</span>
                        </div>
                        <p className="truncate text-xs text-hero-muted">{item.detail}</p>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="platform" className="border-b border-border bg-background text-foreground">
        <div className="mx-auto grid max-w-7xl gap-px bg-border px-4 py-8 sm:px-6 lg:grid-cols-3 lg:px-8">
          {proofPoints.map((point) => (
            <div key={point.value} className="bg-card p-5">
              <div className="flex items-center gap-2 text-sm font-semibold text-primary">
                <CheckCircle2 className="h-4 w-4" />
                {point.value}
              </div>
              <p className="mt-3 text-sm leading-6 text-muted-foreground">{point.label}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-hero px-4 py-16 text-hero-foreground sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="grid gap-10 lg:grid-cols-[0.82fr_1.18fr] lg:items-start">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-hero-accent">Legal OS</div>
              <h2 className="mt-3 max-w-xl text-3xl font-semibold tracking-normal sm:text-4xl">
                A quieter operating surface for complicated legal work.
              </h2>
              <p className="mt-4 max-w-xl text-sm leading-7 text-hero-muted">
                The app is built for repeat use: dense enough for real case work, clear enough that source support remains visible.
              </p>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              {operatingLanes.map((lane) => {
                const Icon = lane.icon
                return (
                  <article key={lane.title} className="rounded-md border border-hero-border bg-hero-foreground/[0.04] p-5">
                    <Icon className="h-5 w-5 text-hero-accent" />
                    <h3 className="mt-4 text-base font-semibold tracking-normal text-hero-foreground">{lane.title}</h3>
                    <p className="mt-3 text-sm leading-6 text-hero-muted">{lane.body}</p>
                  </article>
                )
              })}
            </div>
          </div>
        </div>
      </section>

      <section id="workflow" className="bg-background px-4 py-16 text-foreground sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-primary">Workflow</div>
              <h2 className="mt-3 max-w-2xl text-3xl font-semibold tracking-normal sm:text-4xl">
                From loose files to source-backed work product.
              </h2>
            </div>
            <div className="flex items-center gap-2 rounded-md border border-border bg-card px-3 py-2 text-sm text-muted-foreground">
              <LockKeyhole className="h-4 w-4 text-primary" />
              Protected app access through Zitadel
            </div>
          </div>

          <div className="mt-10 grid gap-3 md:grid-cols-5">
            {workflow.map((item, index) => {
              const Icon = item.icon
              return (
                <div key={item.label} className="rounded-md border border-border bg-card p-4">
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-mono text-[11px] uppercase tracking-normal text-primary">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <Icon className="h-4 w-4 text-accent" />
                  </div>
                  <h3 className="mt-5 text-base font-semibold tracking-normal">{item.label}</h3>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">{item.detail}</p>
                </div>
              )
            })}
          </div>

          <div className="mt-10 flex flex-col items-start justify-between gap-5 rounded-md border border-hero-border bg-hero p-5 text-hero-foreground sm:flex-row sm:items-center">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-hero-accent">Ready</div>
              <p className="mt-2 max-w-2xl text-base leading-7 text-hero-muted">
                Request beta access, accept an invite, then create your first protected CaseBuilder matter.
              </p>
            </div>
            <Button
              asChild
              className="min-h-10 rounded-md bg-hero-accent px-5 text-hero-accent-foreground hover:bg-hero-accent/85"
            >
              <Link
                href={primaryHref}
                onClick={() => {
                  if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                }}
              >
                {isSignedIn ? "Create First Matter" : "Request Access"}
                <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
          </div>
        </div>
      </section>

      <footer className="border-t border-hero-border bg-hero px-4 py-7 text-sm text-hero-muted sm:px-6 lg:px-8">
        <div className="mx-auto flex max-w-7xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2">
            <BookMarked className="h-4 w-4 text-hero-accent" />
            <span>ORSGraph</span>
          </div>
          <div>Source-backed legal work for Oregon matters.</div>
        </div>
      </footer>
    </main>
  )
}
