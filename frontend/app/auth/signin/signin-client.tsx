"use client"

import Link from "next/link"
import { signIn } from "next-auth/react"
import { ArrowRight, KeyRound, Mail } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"
import { Button } from "@/components/ui/button"
import { trackConversionEvent } from "@/lib/conversion-events"

export function SignInClient({ safeCallbackUrl }: { safeCallbackUrl: string }) {
  function startSignIn() {
    trackConversionEvent("sign_in_started", { source: "signin_page" })
    void signIn("zitadel", { callbackUrl: safeCallbackUrl })
  }

  return (
    <AuthFrame
      eyebrow="Protected access"
      title="Sign in to keep your legal work private."
      body="ORSGraph is opening gradually so self represented users can get a controlled, source-first workspace without losing track of evidence, claims, or deadlines."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        <div className="flex items-start gap-3">
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary/15 text-primary">
            <KeyRound className="h-5 w-5" />
          </span>
          <div>
            <h2 className="text-lg font-semibold tracking-normal">Use your beta invite</h2>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">
              If your invite has already been accepted, sign in and continue to your first matter.
            </p>
          </div>
        </div>

        <Button type="button" onClick={startSignIn} className="mt-5 min-h-11 w-full rounded-md">
          Sign in
          <ArrowRight className="h-4 w-4" />
        </Button>

        <div className="mt-5 grid gap-2 sm:grid-cols-2">
          <Link
            href="/auth/request-access"
            className="inline-flex min-h-10 items-center justify-center gap-2 rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
          >
            <Mail className="h-4 w-4" />
            Request access
          </Link>
          <Link
            href="/"
            className="inline-flex min-h-10 items-center justify-center rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
          >
            Back to ORSGraph
          </Link>
        </div>
      </section>
    </AuthFrame>
  )
}
