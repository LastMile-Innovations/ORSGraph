"use client"

import Image from "next/image"
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
    <main className="min-h-screen overflow-x-hidden bg-[#0b0f16] text-[#f8fbff]">
      <section className="relative min-h-[84svh] overflow-hidden border-b border-white/10">
        <Image
          src="/marketing/legal-os-hero.png"
          alt="ORSGraph legal operations workspace with documents, citation graph, and case timeline"
          fill
          priority
          sizes="100vw"
          className="object-cover"
        />
        <div className="absolute inset-0 bg-[linear-gradient(90deg,rgba(4,8,16,0.95)_0%,rgba(7,12,22,0.82)_48%,rgba(7,12,22,0.34)_100%)]" />
        <div className="absolute inset-x-0 top-0 z-30 border-b border-white/10 bg-[#07101c]/65 backdrop-blur">
          <header className="mx-auto flex h-16 max-w-7xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-[#7dd3fc] text-[#06111e] shadow-lg shadow-black/20">
                <Landmark className="h-5 w-5" strokeWidth={2.4} />
              </span>
              <div className="min-w-0">
                <div className="font-mono text-sm font-semibold tracking-normal text-white">ORSGraph</div>
                <div className="hidden font-mono text-[10px] uppercase tracking-normal text-[#a8b8cc] sm:block">
                  Source-first legal OS
                </div>
              </div>
            </div>

            <nav className="hidden items-center gap-1 text-sm font-medium text-[#c8d7ea] md:flex">
              <a href="#platform" className="rounded-md px-3 py-2 outline-none hover:bg-white/10 hover:text-white focus-visible:ring-2 focus-visible:ring-[#7dd3fc]/70">
                Platform
              </a>
              <a href="#workflow" className="rounded-md px-3 py-2 outline-none hover:bg-white/10 hover:text-white focus-visible:ring-2 focus-visible:ring-[#7dd3fc]/70">
                Workflow
              </a>
            </nav>

            <div className="flex shrink-0 items-center gap-2">
              {!isSignedIn && (
                <Button
                  asChild
                  variant="outline"
                  className="hidden h-9 rounded-md border-white/20 bg-white/5 px-4 text-white hover:bg-white/10 sm:inline-flex"
                >
                  <a href={inviteHref} onClick={() => trackConversionEvent("sign_in_started", { source: "landing" })}>
                    I Have An Invite
                  </a>
                </Button>
              )}
              <Button
                asChild
                className="h-9 rounded-md bg-[#7dd3fc] px-4 text-[#06111e] hover:bg-[#bae6fd]"
              >
                <a
                  href={primaryHref}
                  onClick={() => {
                    if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                  }}
                >
                  {isSignedIn ? "Enter App" : "Request Access"}
                </a>
              </Button>
            </div>
          </header>
        </div>

        <div className="relative z-10 mx-auto flex min-h-[84svh] max-w-7xl items-end px-4 pb-10 pt-28 sm:px-6 sm:pb-14 lg:px-8">
          <div className="grid w-full gap-8 lg:grid-cols-[minmax(0,0.94fr)_26rem] lg:items-end">
            <div className="max-w-4xl">
              <div className="mb-5 inline-flex max-w-full min-w-0 items-center gap-2 rounded-md border border-[#7dd3fc]/30 bg-[#07101c]/55 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-normal text-[#bae6fd] backdrop-blur">
                <Sparkles className="h-3.5 w-3.5 shrink-0" />
                <span className="min-w-0 truncate">Authority, evidence, and drafting in one source trail</span>
              </div>

              <h1 className="max-w-4xl text-balance text-4xl font-semibold tracking-normal text-white sm:text-5xl lg:text-6xl">
                ORSGraph is the legal workspace where every answer keeps its receipts.
              </h1>

              <p className="mt-5 max-w-2xl text-pretty break-words text-base leading-7 text-[#d7e2f1] sm:text-lg">
                Research Oregon law, structure matters, connect evidence to claims, and move drafts through QC without losing the source trail.
              </p>

              <div className="mt-8 flex flex-col gap-3 sm:flex-row">
                <Button
                  asChild
                  className="min-h-11 w-full rounded-md bg-[#7dd3fc] px-5 text-[#06111e] hover:bg-[#bae6fd] sm:w-auto"
                >
                  <a
                    href={primaryHref}
                    onClick={() => {
                      if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                    }}
                  >
                    {isSignedIn ? "Create First Matter" : "Request Beta Access"}
                    <ArrowRight className="h-4 w-4" />
                  </a>
                </Button>
                <a
                  href={isSignedIn ? "/dashboard" : inviteHref}
                  onClick={() => {
                    if (!isSignedIn) trackConversionEvent("sign_in_started", { source: "landing" })
                  }}
                  className="inline-flex min-h-11 w-full items-center justify-center gap-2 rounded-md border border-white/20 bg-[#07101c]/55 px-5 text-sm font-medium text-white outline-none backdrop-blur transition hover:border-[#7dd3fc]/70 hover:bg-[#07101c]/80 focus-visible:ring-2 focus-visible:ring-[#7dd3fc]/70 sm:w-auto"
                >
                  {isSignedIn ? "Open Dashboard" : "I Have An Invite"}
                  <GitBranch className="h-4 w-4" />
                </a>
              </div>
            </div>

            <div className="hidden rounded-md border border-white/15 bg-[#07101c]/70 p-4 shadow-2xl shadow-black/30 backdrop-blur lg:block">
              <div className="flex items-center justify-between gap-3 border-b border-white/10 pb-3">
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-normal text-[#7dd3fc]">Live matter flow</div>
                  <div className="mt-1 text-sm font-semibold text-white">Source pack complete</div>
                </div>
                <span className="rounded-md bg-[#22c55e]/15 px-2 py-1 font-mono text-[10px] uppercase tracking-normal text-[#86efac]">
                  QC ready
                </span>
              </div>
              <div className="mt-4 space-y-3">
                {workflow.map((item, index) => {
                  const Icon = item.icon
                  return (
                    <div key={item.label} className="flex items-center gap-3 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2.5">
                      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-[#7dd3fc]/15 text-[#7dd3fc]">
                        <Icon className="h-4 w-4" />
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center justify-between gap-3">
                          <span className="text-sm font-medium text-white">{item.label}</span>
                          <span className="font-mono text-[10px] text-[#8da2bc]">{String(index + 1).padStart(2, "0")}</span>
                        </div>
                        <p className="truncate text-xs text-[#a8b8cc]">{item.detail}</p>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="platform" className="border-b border-[#dbe4ef] bg-[#f8fafc] text-[#101827]">
        <div className="mx-auto grid max-w-7xl gap-px bg-[#dbe4ef] px-4 py-8 sm:px-6 lg:grid-cols-3 lg:px-8">
          {proofPoints.map((point) => (
            <div key={point.value} className="bg-white p-5">
              <div className="flex items-center gap-2 text-sm font-semibold text-[#075985]">
                <CheckCircle2 className="h-4 w-4" />
                {point.value}
              </div>
              <p className="mt-3 text-sm leading-6 text-[#42526b]">{point.label}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-[#101827] px-4 py-16 text-white sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="grid gap-10 lg:grid-cols-[0.82fr_1.18fr] lg:items-start">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#7dd3fc]">Legal OS</div>
              <h2 className="mt-3 max-w-xl text-3xl font-semibold tracking-normal sm:text-4xl">
                A quieter operating surface for complicated legal work.
              </h2>
              <p className="mt-4 max-w-xl text-sm leading-7 text-[#c8d7ea]">
                The app is built for repeat use: dense enough for real case work, clear enough that source support remains visible.
              </p>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              {operatingLanes.map((lane) => {
                const Icon = lane.icon
                return (
                  <article key={lane.title} className="rounded-md border border-white/10 bg-white/[0.04] p-5">
                    <Icon className="h-5 w-5 text-[#7dd3fc]" />
                    <h3 className="mt-4 text-base font-semibold tracking-normal text-white">{lane.title}</h3>
                    <p className="mt-3 text-sm leading-6 text-[#c8d7ea]">{lane.body}</p>
                  </article>
                )
              })}
            </div>
          </div>
        </div>
      </section>

      <section id="workflow" className="bg-[#f8fafc] px-4 py-16 text-[#101827] sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#075985]">Workflow</div>
              <h2 className="mt-3 max-w-2xl text-3xl font-semibold tracking-normal sm:text-4xl">
                From loose files to source-backed work product.
              </h2>
            </div>
            <div className="flex items-center gap-2 rounded-md border border-[#cbd5e1] bg-white px-3 py-2 text-sm text-[#42526b]">
              <LockKeyhole className="h-4 w-4 text-[#075985]" />
              Protected app access through Zitadel
            </div>
          </div>

          <div className="mt-10 grid gap-3 md:grid-cols-5">
            {workflow.map((item, index) => {
              const Icon = item.icon
              return (
                <div key={item.label} className="rounded-md border border-[#dbe4ef] bg-white p-4">
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-mono text-[11px] uppercase tracking-normal text-[#075985]">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <Icon className="h-4 w-4 text-[#0891b2]" />
                  </div>
                  <h3 className="mt-5 text-base font-semibold tracking-normal">{item.label}</h3>
                  <p className="mt-2 text-sm leading-6 text-[#516176]">{item.detail}</p>
                </div>
              )
            })}
          </div>

          <div className="mt-10 flex flex-col items-start justify-between gap-5 rounded-md border border-[#cbd5e1] bg-[#101827] p-5 text-white sm:flex-row sm:items-center">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#7dd3fc]">Ready</div>
              <p className="mt-2 max-w-2xl text-base leading-7 text-[#d7e2f1]">
                Request beta access, accept an invite, then create your first protected CaseBuilder matter.
              </p>
            </div>
            <Button
              asChild
              className="min-h-10 rounded-md bg-[#7dd3fc] px-5 text-[#06111e] hover:bg-[#bae6fd]"
            >
              <a
                href={primaryHref}
                onClick={() => {
                  if (!isSignedIn) trackConversionEvent("landing_cta_click", { action: "request_access" })
                }}
              >
                {isSignedIn ? "Create First Matter" : "Request Access"}
                <ArrowRight className="h-4 w-4" />
              </a>
            </Button>
          </div>
        </div>
      </section>

      <footer className="border-t border-white/10 bg-[#0b0f16] px-4 py-7 text-sm text-[#a8b8cc] sm:px-6 lg:px-8">
        <div className="mx-auto flex max-w-7xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2">
            <BookMarked className="h-4 w-4 text-[#7dd3fc]" />
            <span>ORSGraph</span>
          </div>
          <div>Source-backed legal work for Oregon matters.</div>
        </div>
      </footer>
    </main>
  )
}
