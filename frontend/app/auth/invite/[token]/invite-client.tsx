"use client"

import { useEffect, useState } from "react"
import { signIn, useSession } from "next-auth/react"
import { useRouter } from "next/navigation"
import { ArrowRight, CheckCircle2, Loader2, ShieldAlert, TicketCheck } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"
import { Button } from "@/components/ui/button"
import { acceptInvite, lookupInvite, type InviteLookupResponse } from "@/lib/auth-access"
import { trackConversionEvent } from "@/lib/conversion-events"

export function InviteClient({ token }: { token: string }) {
  const router = useRouter()
  const session = useSession()
  const [invite, setInvite] = useState<InviteLookupResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let disposed = false
    lookupInvite(token)
      .then((next) => {
        if (!disposed) setInvite(next)
      })
      .catch((err) => {
        if (!disposed) setError(err instanceof Error ? err.message : "Invite lookup failed")
      })
      .finally(() => {
        if (!disposed) setLoading(false)
      })
    return () => {
      disposed = true
    }
  }, [token])

  async function accept() {
    if (session.status !== "authenticated") {
      void signIn("zitadel", { callbackUrl: `/auth/invite/${encodeURIComponent(token)}` })
      return
    }
    setBusy(true)
    setError(null)
    try {
      await acceptInvite(token)
      trackConversionEvent("invite_accepted")
      await session.update()
      router.push("/onboarding")
    } catch (err) {
      setError(err instanceof Error ? err.message : "Invite could not be accepted")
    } finally {
      setBusy(false)
    }
  }

  const inviteStatus = invite?.status || "loading"
  const usable = invite?.found && inviteStatus === "active"

  return (
    <AuthFrame
      eyebrow="Beta invite"
      title={usable ? "Accept your ORSGraph beta invite." : "This invite needs attention."}
      body="Use the invited account to activate your protected workspace, then create your first CaseBuilder matter."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        <div className="flex items-start gap-3">
          {usable ? (
            <TicketCheck className="mt-1 h-6 w-6 shrink-0 text-success" />
          ) : (
            <ShieldAlert className="mt-1 h-6 w-6 shrink-0 text-warning" />
          )}
          <div>
            <h2 className="text-lg font-semibold">
              {loading ? "Checking invite" : usable ? "Invite ready" : statusMessage(inviteStatus)}
            </h2>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">
              {usable
                ? invite.email
                  ? `This invite is for ${invite.email}. Sign in with that email to continue.`
                  : "This invite can be accepted by your signed-in account."
                : recoveryMessage(inviteStatus)}
            </p>
          </div>
        </div>

        {error && <div className="mt-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}

        <Button type="button" disabled={loading || busy || !usable} onClick={accept} className="mt-5 min-h-11 w-full rounded-md">
          {busy || loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <CheckCircle2 className="h-4 w-4" />}
          {session.status === "authenticated" ? "Accept invite" : "Sign in to accept"}
          <ArrowRight className="h-4 w-4" />
        </Button>
      </section>
    </AuthFrame>
  )
}

function statusMessage(status: string) {
  if (status === "accepted") return "Invite already accepted"
  if (status === "expired") return "Invite expired"
  if (status === "revoked") return "Invite revoked"
  return "Invite not found"
}

function recoveryMessage(status: string) {
  if (status === "accepted") return "Sign in with the account that accepted this invite."
  if (status === "expired") return "Request a fresh beta invite."
  if (status === "revoked") return "Request access again if you still need the beta."
  return "Check the invite link or request beta access."
}
