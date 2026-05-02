"use client"

import Link from "next/link"
import { FormEvent, useState } from "react"
import { ArrowRight, CheckCircle2, Loader2, Mail, ShieldCheck } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"
import { Button } from "@/components/ui/button"
import { requestAccess } from "@/lib/auth-access"
import { trackConversionEvent } from "@/lib/conversion-events"

const SITUATIONS = [
  "I need to respond to a complaint",
  "I need to build a complaint",
  "I need to organize evidence",
  "I need legal research for an Oregon matter",
]

const URGENCY = ["Deadline this week", "Deadline this month", "No immediate deadline", "Not sure"]

export function RequestAccessClient() {
  const [email, setEmail] = useState("")
  const [situation, setSituation] = useState(SITUATIONS[0])
  const [urgency, setUrgency] = useState(URGENCY[1])
  const [jurisdiction, setJurisdiction] = useState("Oregon")
  const [note, setNote] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [submitted, setSubmitted] = useState(false)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    try {
      await requestAccess({
        email,
        situation_type: situation,
        deadline_urgency: urgency,
        jurisdiction,
        note,
      })
      trackConversionEvent("access_request_submitted", { situation, urgency })
      setSubmitted(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not submit access request")
    } finally {
      setBusy(false)
    }
  }

  return (
    <AuthFrame
      eyebrow="Beta access"
      title="Tell us what you need to get under control."
      body="We are prioritizing people who need a practical workspace for deadlines, evidence, claims, and source-backed legal work."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        {submitted ? (
          <div className="space-y-5">
            <div className="flex items-start gap-3">
              <CheckCircle2 className="mt-1 h-6 w-6 shrink-0 text-success" />
              <div>
                <h2 className="text-lg font-semibold">Request received</h2>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">
                  We will review your beta access request. If you receive an invite, use the same email address when signing in.
                </p>
              </div>
            </div>
            <Link
              href="/auth/signin"
              className="inline-flex min-h-10 w-full items-center justify-center rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
            >
              I already have access
            </Link>
          </div>
        ) : (
          <form onSubmit={submit} className="space-y-4">
            <Field label="Email">
              <input
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                type="email"
                required
                placeholder="you@example.com"
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:border-primary"
              />
            </Field>
            <Field label="What are you trying to do?">
              <select
                value={situation}
                onChange={(event) => setSituation(event.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:border-primary"
              >
                {SITUATIONS.map((item) => (
                  <option key={item} value={item}>
                    {item}
                  </option>
                ))}
              </select>
            </Field>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="Deadline urgency">
                <select
                  value={urgency}
                  onChange={(event) => setUrgency(event.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:border-primary"
                >
                  {URGENCY.map((item) => (
                    <option key={item} value={item}>
                      {item}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="County / jurisdiction">
                <input
                  value={jurisdiction}
                  onChange={(event) => setJurisdiction(event.target.value)}
                  placeholder="Oregon, Linn County, etc."
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:border-primary"
                />
              </Field>
            </div>
            <Field label="Short note">
              <textarea
                value={note}
                onChange={(event) => setNote(event.target.value)}
                rows={5}
                placeholder="Keep it brief. Do not include private facts you are not ready to share."
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm leading-6 outline-none focus:border-primary"
              />
            </Field>

            {error && <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}

            <Button type="submit" disabled={busy} className="min-h-11 w-full rounded-md">
              {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Mail className="h-4 w-4" />}
              Request beta access
              <ArrowRight className="h-4 w-4" />
            </Button>
            <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
              <ShieldCheck className="h-3.5 w-3.5 text-success" />
              Access requests do not create public case records.
            </div>
          </form>
        )}
      </section>
    </AuthFrame>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-medium">{label}</span>
      {children}
    </label>
  )
}
