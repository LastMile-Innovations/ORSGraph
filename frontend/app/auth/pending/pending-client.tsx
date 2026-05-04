"use client"

import Link from "next/link"
import { signIn, useSession } from "next-auth/react"
import { useEffect } from "react"
import { useRouter } from "next/navigation"
import { ArrowRight, Clock3, ShieldAlert } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"
import { Button } from "@/components/ui/button"

export function PendingClient({ safeCallbackUrl }: { safeCallbackUrl: string }) {
  const router = useRouter()
  const session = useSession()
  const status = session.data?.accessStatus || "unknown"
  const isAdmin = session.data?.isAdmin === true

  useEffect(() => {
    if (session.status === "authenticated") {
      void session.update()
    }
  }, [session])

  useEffect(() => {
    if (session.status === "authenticated" && (session.data?.accessStatus === "active" || isAdmin)) {
      router.replace(safeCallbackUrl)
    }
  }, [isAdmin, router, safeCallbackUrl, session.data?.accessStatus, session.status])

  return (
    <AuthFrame
      eyebrow="Access review"
      title={status === "blocked" ? "This account cannot access ORSGraph." : "Your beta access is not active yet."}
      body="Your identity is signed in, but ORSGraph app access is invite-only for this launch."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        <div className="flex items-start gap-3">
          {status === "blocked" ? (
            <ShieldAlert className="mt-1 h-6 w-6 shrink-0 text-destructive" />
          ) : (
            <Clock3 className="mt-1 h-6 w-6 shrink-0 text-warning" />
          )}
          <div>
            <h2 className="text-lg font-semibold">{status === "blocked" ? "Access blocked" : "Waiting for invite approval"}</h2>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">
              If you have an invite link, open it while signed in with the invited email. Otherwise request beta access and we will review it.
            </p>
          </div>
        </div>
        <div className="mt-5 grid gap-2 sm:grid-cols-2">
          <Link
            href="/auth/request-access"
            className="inline-flex min-h-10 items-center justify-center rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
          >
            Request access
          </Link>
          <Button type="button" variant="outline" onClick={() => signIn("zitadel", { callbackUrl: safeCallbackUrl })} className="min-h-10 rounded-md">
            Sign in again
            <ArrowRight className="h-4 w-4" />
          </Button>
        </div>
      </section>
    </AuthFrame>
  )
}
