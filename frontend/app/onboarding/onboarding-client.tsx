"use client"

import { useRouter } from "next/navigation"
import { useSession } from "next-auth/react"
import { ArrowRight, Briefcase, FileText, GavelIcon, Loader2 } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"
import { Button } from "@/components/ui/button"
import { newMatterHref } from "@/lib/casebuilder/routes"
import { trackConversionEvent } from "@/lib/conversion-events"

const OPTIONS = [
  {
    id: "fight",
    title: "I need to respond",
    body: "Start with a complaint or notice you received and build the response workspace.",
    icon: GavelIcon,
  },
  {
    id: "build",
    title: "I need to file",
    body: "Tell the story, upload evidence, and start a complaint-focused matter.",
    icon: FileText,
  },
  {
    id: "blank",
    title: "Organize a matter",
    body: "Create an empty workspace and add documents, parties, facts, and deadlines.",
    icon: Briefcase,
  },
] as const

export function OnboardingClient() {
  const router = useRouter()
  const session = useSession()

  function choose(intent: (typeof OPTIONS)[number]["id"]) {
    trackConversionEvent("first_matter_started", { intent })
    router.push(newMatterHref(intent === "blank" ? undefined : intent))
  }

  if (session.status === "loading") {
    return (
      <AuthFrame eyebrow="Preparing workspace" title="Checking your access." body="This should only take a moment.">
        <div className="rounded-md border border-border bg-card p-5">
          <Loader2 className="h-5 w-5 animate-spin text-primary" />
        </div>
      </AuthFrame>
    )
  }

  return (
    <AuthFrame
      eyebrow="First matter"
      title="Start with the legal problem in front of you."
      body="CaseBuilder works best when the first step is concrete: the complaint you received, the filing you need to build, or the evidence you need to organize."
    >
      <section className="grid gap-3">
        {OPTIONS.map((option) => {
          const Icon = option.icon
          return (
            <button
              key={option.id}
              type="button"
              onClick={() => choose(option.id)}
              className="flex min-h-24 items-center justify-between gap-4 rounded-md border border-border bg-card p-4 text-left transition hover:border-primary/60 hover:bg-muted/50"
            >
              <span className="flex min-w-0 items-start gap-3">
                <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary/15 text-primary">
                  <Icon className="h-5 w-5" />
                </span>
                <span className="min-w-0">
                  <span className="block font-semibold">{option.title}</span>
                  <span className="mt-1 block text-sm leading-6 text-muted-foreground">{option.body}</span>
                </span>
              </span>
              <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground" />
            </button>
          )
        })}
        <Button type="button" variant="outline" onClick={() => router.push("/dashboard")} className="min-h-10 rounded-md">
          Go to dashboard
        </Button>
      </section>
    </AuthFrame>
  )
}
