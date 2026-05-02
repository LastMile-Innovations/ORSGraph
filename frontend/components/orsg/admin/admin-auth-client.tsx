"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { Clipboard, Loader2, MailPlus, RefreshCcw, ShieldCheck, XCircle } from "lucide-react"
import {
  createInvite,
  listAccessRequests,
  listInvites,
  revokeInvite,
  type BetaInvite,
  type InviteRequest,
} from "@/lib/auth-access"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"

export function AdminAuthClient() {
  const [requests, setRequests] = useState<InviteRequest[]>([])
  const [invites, setInvites] = useState<BetaInvite[]>([])
  const [email, setEmail] = useState("")
  const [situation, setSituation] = useState("")
  const [jurisdiction, setJurisdiction] = useState("Oregon")
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [createdLink, setCreatedLink] = useState<string | null>(null)

  const pendingRequests = useMemo(() => requests.filter((request) => request.status === "pending"), [requests])

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [nextRequests, nextInvites] = await Promise.all([listAccessRequests(), listInvites()])
      setRequests(nextRequests)
      setInvites(nextInvites)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Admin auth API unavailable")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  async function submitInvite(targetEmail = email) {
    setBusy(true)
    setError(null)
    try {
      const response = await createInvite({
        email: targetEmail.trim() || undefined,
        situation_type: situation.trim() || undefined,
        jurisdiction: jurisdiction.trim() || undefined,
        expires_in_days: 14,
      })
      const url = `${window.location.origin}${response.invite_url_path}`
      setCreatedLink(url)
      await navigator.clipboard?.writeText(url).catch(() => undefined)
      setEmail("")
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not create invite")
    } finally {
      setBusy(false)
    }
  }

  async function revoke(invite: BetaInvite) {
    setBusy(true)
    setError(null)
    try {
      await revokeInvite(invite.invite_id)
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not revoke invite")
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin bg-background">
      <header className="border-b border-border bg-card px-6 py-6">
        <div className="mx-auto flex max-w-7xl flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <div className="font-mono text-xs uppercase tracking-normal text-muted-foreground">Admin / beta access</div>
            <h1 className="mt-2 text-2xl font-semibold tracking-normal">Invite-only access control</h1>
            <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
              Review beta requests, create copyable invite links, and keep ORSGraph gated while the product sharpens.
            </p>
          </div>
          <Button type="button" variant="outline" onClick={load} disabled={loading} className="rounded-md">
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCcw className="h-4 w-4" />}
            Refresh
          </Button>
        </div>
      </header>

      <main className="mx-auto grid w-full max-w-7xl gap-6 px-6 py-6 lg:grid-cols-[0.74fr_1.26fr]">
        <section className="rounded-md border border-border bg-card p-4">
          <div className="flex items-center gap-2">
            <MailPlus className="h-5 w-5 text-primary" />
            <h2 className="font-semibold">Create invite</h2>
          </div>
          <div className="mt-4 space-y-3">
            <Field label="Email">
              <Input value={email} onChange={(event) => setEmail(event.target.value)} placeholder="user@example.com" />
            </Field>
            <Field label="Situation">
              <Input value={situation} onChange={(event) => setSituation(event.target.value)} placeholder="Build complaint, respond, organize evidence" />
            </Field>
            <Field label="Jurisdiction">
              <Input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value)} />
            </Field>
            <Button type="button" onClick={() => submitInvite()} disabled={busy} className="w-full rounded-md">
              {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <ShieldCheck className="h-4 w-4" />}
              Create and copy invite
            </Button>
            {createdLink && (
              <div className="rounded-md border border-success/30 bg-success/10 p-3 text-xs text-success">
                Invite copied: {createdLink}
              </div>
            )}
            {error && <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">{error}</div>}
          </div>
        </section>

        <section className="space-y-6">
          <Panel title="Pending requests" count={pendingRequests.length}>
            {pendingRequests.length === 0 ? (
              <Empty label="No pending access requests." />
            ) : (
              <div className="divide-y divide-border">
                {pendingRequests.map((request) => (
                  <article key={request.request_id} className="flex flex-col gap-3 py-4 sm:flex-row sm:items-start sm:justify-between">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-mono text-sm text-foreground">{request.email}</span>
                        <StatusBadge status={request.status} />
                      </div>
                      <p className="mt-1 text-sm text-muted-foreground">{request.situation_type || "No situation supplied"}</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {request.deadline_urgency || "No urgency"} / {request.jurisdiction || "No jurisdiction"}
                      </p>
                      {request.note && <p className="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{request.note}</p>}
                    </div>
                    <Button type="button" size="sm" onClick={() => submitInvite(request.email)} disabled={busy} className="rounded-md">
                      <Clipboard className="h-4 w-4" />
                      Invite
                    </Button>
                  </article>
                ))}
              </div>
            )}
          </Panel>

          <Panel title="Recent invites" count={invites.length}>
            {invites.length === 0 ? (
              <Empty label="No invites yet." />
            ) : (
              <div className="divide-y divide-border">
                {invites.map((invite) => (
                  <article key={invite.invite_id} className="flex flex-col gap-3 py-4 sm:flex-row sm:items-start sm:justify-between">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-mono text-sm text-foreground">{invite.email || "Open invite"}</span>
                        <StatusBadge status={invite.status} />
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        Expires {formatTimestamp(invite.expires_at)}
                        {invite.accepted_at ? ` / accepted ${formatTimestamp(invite.accepted_at)}` : ""}
                      </p>
                    </div>
                    {invite.status === "active" && (
                      <Button type="button" size="sm" variant="outline" onClick={() => revoke(invite)} disabled={busy} className="rounded-md">
                        <XCircle className="h-4 w-4" />
                        Revoke
                      </Button>
                    )}
                  </article>
                ))}
              </div>
            )}
          </Panel>
        </section>
      </main>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <Label>{label}</Label>
      {children}
    </div>
  )
}

function Panel({ title, count, children }: { title: string; count: number; children: React.ReactNode }) {
  return (
    <section className="rounded-md border border-border bg-card p-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="font-semibold">{title}</h2>
        <Badge variant="secondary">{count}</Badge>
      </div>
      <div className="mt-2">{children}</div>
    </section>
  )
}

function StatusBadge({ status }: { status: string }) {
  const tone = status === "active" || status === "accepted" || status === "invited" ? "bg-success/15 text-success" : status === "revoked" || status === "blocked" ? "bg-destructive/15 text-destructive" : "bg-warning/15 text-warning"
  return <span className={`rounded-md px-2 py-0.5 font-mono text-[10px] uppercase tracking-normal ${tone}`}>{status}</span>
}

function Empty({ label }: { label: string }) {
  return <div className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">{label}</div>
}

function formatTimestamp(value?: string | null) {
  if (!value) return "unknown"
  const seconds = Number(value)
  if (!Number.isFinite(seconds)) return value
  return new Date(seconds * 1000).toLocaleDateString()
}
