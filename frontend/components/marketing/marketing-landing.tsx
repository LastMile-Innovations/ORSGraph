"use client"

import Image from "next/image"
import { signIn, useSession } from "next-auth/react"
import { useRouter } from "next/navigation"
import {
  ArrowRight,
  BookMarked,
  BriefcaseBusiness,
  CheckCircle2,
  FileCheck2,
  GitBranch,
  Landmark,
  LockKeyhole,
  SearchCheck,
  ShieldCheck,
  Sparkles,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { trackConversionEvent } from "@/lib/conversion-events"

const proofPoints = [
  { value: "Source-first", label: "Authority, exhibit, and filing records stay traceable." },
  { value: "Graph-native", label: "Statutes, claims, facts, and citations live in one operating model." },
  { value: "Filing-ready", label: "Matter work moves from intake to QC without leaving the workspace." },
]

const operatingLanes = [
  {
    icon: SearchCheck,
    title: "Research that remembers",
    body: "Ask questions, inspect citation paths, and keep controlling authority connected to the work product it supports.",
  },
  {
    icon: BriefcaseBusiness,
    title: "Matter command center",
    body: "Organize parties, facts, evidence, deadlines, tasks, and draft work without turning the case into scattered notes.",
  },
  {
    icon: FileCheck2,
    title: "Quality before filing",
    body: "Review claims, source trails, warnings, and export packages from the same operational surface.",
  },
]

const workflow = [
  { label: "Intake", detail: "documents, folders, facts" },
  { label: "Structure", detail: "claims, parties, timelines" },
  { label: "Authority", detail: "statutes, citations, source trails" },
  { label: "Draft", detail: "complaints, answers, motions" },
  { label: "QC", detail: "warnings, exports, audit trail" },
]

export function MarketingLanding() {
  const router = useRouter()
  const session = useSession()
  const isSignedIn = session.status === "authenticated"

  function enterApp() {
    if (isSignedIn) {
      router.push("/onboarding")
      return
    }

    trackConversionEvent("landing_cta_click", { action: "request_access" })
    router.push("/auth/request-access")
  }

  function signInFromLanding() {
    trackConversionEvent("sign_in_started", { source: "landing" })
    void signIn("zitadel", { callbackUrl: "/onboarding" })
  }

  return (
    <main className="min-h-screen overflow-x-hidden bg-[#10130f] text-[#f7f0df]">
      <section className="relative min-h-[86svh] overflow-hidden">
        <Image
          src="/marketing/legal-os-hero.png"
          alt="ORSGraph legal operations workspace with documents, citation graph, and case timeline"
          fill
          priority
          sizes="100vw"
          className="object-cover"
        />
        <div className="absolute inset-0 bg-[linear-gradient(90deg,rgba(9,12,13,0.94)_0%,rgba(9,12,13,0.78)_44%,rgba(9,12,13,0.28)_100%)]" />
        <div className="absolute inset-x-0 top-0 z-10">
          <header className="mx-auto flex h-16 max-w-7xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-[#f0b35a] text-[#11140f] shadow-lg shadow-black/20">
                <Landmark className="h-5 w-5" strokeWidth={2.4} />
              </span>
              <div className="min-w-0">
                <div className="font-mono text-sm font-semibold tracking-normal text-[#fff8e6]">ORSGraph</div>
                <div className="hidden font-mono text-[10px] uppercase tracking-normal text-[#c9d8cc] sm:block">
                  Legal operating environment
                </div>
              </div>
            </div>

            <div className="flex shrink-0 items-center gap-2">
              <a
                href="#platform"
                className="hidden rounded-md px-3 py-2 text-sm font-medium text-[#dce8dc] outline-none transition hover:bg-white/10 hover:text-white focus-visible:ring-2 focus-visible:ring-[#f0b35a]/70 sm:inline-flex"
              >
                Platform
              </a>
              <Button
                type="button"
                onClick={enterApp}
                className="h-9 rounded-md bg-[#f0b35a] px-4 text-[#11140f] hover:bg-[#f4c978]"
              >
                {isSignedIn ? "Enter Dashboard" : "Request Access"}
              </Button>
              {!isSignedIn && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={signInFromLanding}
                  className="hidden h-9 rounded-md border-[#d9ead6]/25 bg-[#10130f]/35 px-4 text-[#f7f0df] hover:bg-[#10130f]/60 sm:inline-flex"
                >
                  I Have An Invite
                </Button>
              )}
            </div>
          </header>
        </div>

        <div className="relative z-10 mx-auto flex min-h-[86svh] max-w-7xl items-end px-4 pb-12 pt-28 sm:px-6 sm:pb-16 lg:px-8">
          <div className="w-full max-w-[calc(100vw-2rem)] sm:max-w-4xl">
            <div className="mb-5 inline-flex max-w-full min-w-0 items-center gap-2 rounded-md border border-[#f0b35a]/30 bg-black/25 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-normal text-[#f6d392] backdrop-blur">
              <Sparkles className="h-3.5 w-3.5 shrink-0" />
              <span className="min-w-0 truncate">Source-backed case work, from authority to filing</span>
            </div>

            <h1 className="max-w-4xl text-balance text-4xl font-semibold tracking-normal text-[#fff8e6] sm:text-5xl lg:text-6xl">
              ORSGraph turns legal work into a source-first operating system.
            </h1>

            <p className="mt-5 max-w-2xl text-pretty break-words text-base leading-7 text-[#d8dfd4] sm:text-lg">
              Research Oregon law, build matters, connect evidence to claims, and ship cleaner filings from one protected workspace.
            </p>

            <div className="mt-8 flex flex-col gap-3 sm:flex-row">
              <Button
                type="button"
                onClick={enterApp}
                className="min-h-11 w-full rounded-md bg-[#f0b35a] px-5 text-[#11140f] hover:bg-[#f4c978] sm:w-auto"
              >
                {isSignedIn ? "Create First Matter" : "Request Beta Access"}
                <ArrowRight className="h-4 w-4" />
              </Button>
              <button
                type="button"
                onClick={isSignedIn ? () => router.push("/dashboard") : signInFromLanding}
                className="inline-flex min-h-11 w-full items-center justify-center gap-2 rounded-md border border-[#d9ead6]/25 bg-[#10130f]/50 px-5 text-sm font-medium text-[#f7f0df] outline-none backdrop-blur transition hover:border-[#79d5c8]/70 hover:bg-[#10130f]/75 focus-visible:ring-2 focus-visible:ring-[#79d5c8]/70 sm:w-auto"
              >
                {isSignedIn ? "Open Dashboard" : "I Have An Invite"}
                <GitBranch className="h-4 w-4" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <section id="platform" className="border-y border-[#314035] bg-[#f7f0df] text-[#151915]">
        <div className="mx-auto grid max-w-7xl gap-6 px-4 py-8 sm:px-6 lg:grid-cols-3 lg:px-8">
          {proofPoints.map((point) => (
            <div key={point.value} className="rounded-md border border-[#d8c9a6] bg-[#fffaf0] p-5 shadow-sm">
              <div className="flex items-center gap-2 text-sm font-semibold text-[#884635]">
                <CheckCircle2 className="h-4 w-4" />
                {point.value}
              </div>
              <p className="mt-3 text-sm leading-6 text-[#3f473e]">{point.label}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-[#171d19] px-4 py-16 text-[#f7f0df] sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="grid gap-10 lg:grid-cols-[0.86fr_1.14fr] lg:items-start">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#79d5c8]">Legal OS</div>
              <h2 className="mt-3 max-w-xl text-3xl font-semibold tracking-normal sm:text-4xl">
                One workspace for the work that usually fractures.
              </h2>
              <p className="mt-4 max-w-xl text-sm leading-7 text-[#cbd5c5]">
                ORSGraph keeps research, evidence, drafting, and review close enough that every claim can carry its source trail.
              </p>
            </div>

            <div className="grid gap-4 md:grid-cols-3">
              {operatingLanes.map((lane) => {
                const Icon = lane.icon
                return (
                  <article key={lane.title} className="rounded-md border border-[#33473c] bg-[#202821] p-5">
                    <Icon className="h-5 w-5 text-[#f0b35a]" />
                    <h3 className="mt-4 text-base font-semibold tracking-normal text-[#fff8e6]">{lane.title}</h3>
                    <p className="mt-3 text-sm leading-6 text-[#cbd5c5]">{lane.body}</p>
                  </article>
                )
              })}
            </div>
          </div>
        </div>
      </section>

      <section className="bg-[#f7f0df] px-4 py-16 text-[#151915] sm:px-6 lg:px-8">
        <div className="mx-auto max-w-7xl">
          <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#884635]">Workflow</div>
              <h2 className="mt-3 max-w-2xl text-3xl font-semibold tracking-normal sm:text-4xl">
                Built for cases that need control, not another pile of tabs.
              </h2>
            </div>
            <div className="flex items-center gap-2 rounded-md border border-[#cfbea0] bg-[#fffaf0] px-3 py-2 text-sm text-[#3f473e]">
              <LockKeyhole className="h-4 w-4 text-[#884635]" />
              Protected app access through Zitadel
            </div>
          </div>

          <div className="mt-10 grid gap-3 md:grid-cols-5">
            {workflow.map((item, index) => (
              <div key={item.label} className="rounded-md border border-[#d8c9a6] bg-[#fffaf0] p-4">
                <div className="flex items-center justify-between gap-3">
                  <span className="font-mono text-[11px] uppercase tracking-normal text-[#884635]">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <ShieldCheck className="h-4 w-4 text-[#579c89]" />
                </div>
                <h3 className="mt-5 text-base font-semibold tracking-normal">{item.label}</h3>
                <p className="mt-2 text-sm leading-6 text-[#566053]">{item.detail}</p>
              </div>
            ))}
          </div>

          <div className="mt-10 flex flex-col items-start justify-between gap-5 rounded-md border border-[#cfbea0] bg-[#171d19] p-5 text-[#f7f0df] sm:flex-row sm:items-center">
            <div>
              <div className="font-mono text-xs uppercase tracking-normal text-[#79d5c8]">Ready</div>
              <p className="mt-2 max-w-2xl text-base leading-7 text-[#e8eadc]">
                Request beta access, accept an invite, then create your first protected CaseBuilder matter.
              </p>
            </div>
            <Button
              type="button"
              onClick={enterApp}
              className="min-h-10 rounded-md bg-[#f0b35a] px-5 text-[#11140f] hover:bg-[#f4c978]"
            >
              {isSignedIn ? "Create First Matter" : "Request Access"}
              <ArrowRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </section>

      <footer className="border-t border-[#29352e] bg-[#10130f] px-4 py-7 text-sm text-[#aeb9ad] sm:px-6 lg:px-8">
        <div className="mx-auto flex max-w-7xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2">
            <BookMarked className="h-4 w-4 text-[#f0b35a]" />
            <span>ORSGraph</span>
          </div>
          <div>Source-backed legal work for Oregon matters.</div>
        </div>
      </footer>
    </main>
  )
}
