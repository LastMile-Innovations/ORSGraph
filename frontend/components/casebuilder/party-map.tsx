"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { Building2, Mail, Phone, Plus, UserRound, Users } from "lucide-react"
import type { Matter } from "@/lib/casebuilder/types"
import { createParty } from "@/lib/casebuilder/api"

interface PartyMapProps {
  matter: Matter
}

export function PartyMap({ matter }: PartyMapProps) {
  const router = useRouter()
  const [name, setName] = useState("")
  const [role, setRole] = useState("witness")
  const [partyType, setPartyType] = useState("individual")
  const [representedBy, setRepresentedBy] = useState("")
  const [contactEmail, setContactEmail] = useState("")
  const [contactPhone, setContactPhone] = useState("")
  const [notes, setNotes] = useState("")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function onCreate() {
    if (!name.trim()) {
      setError("Add a party name.")
      return
    }
    setSaving(true)
    setError(null)
    const result = await createParty(matter.id, {
      name: name.trim(),
      role,
      party_type: partyType,
      represented_by: representedBy.trim() || undefined,
      contact_email: contactEmail.trim() || undefined,
      contact_phone: contactPhone.trim() || undefined,
      notes: notes.trim() || undefined,
    })
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Party could not be created.")
      return
    }
    setName("")
    setRole("witness")
    setPartyType("individual")
    setRepresentedBy("")
    setContactEmail("")
    setContactPhone("")
    setNotes("")
    router.refresh()
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <Users className="h-3.5 w-3.5 text-primary" />
          party / entity map
        </div>
        <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Parties</h1>
        <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
          People, organizations, courts, agencies, witnesses, and counsel linked to this matter.
        </p>
      </header>

      <section className="border-b border-border bg-background px-6 py-4">
        <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_160px_160px]">
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Party or entity name"
            className="rounded border border-border bg-card px-3 py-2 text-sm focus:border-primary focus:outline-none"
          />
          <select
            value={role}
            onChange={(event) => setRole(event.target.value)}
            className="rounded border border-border bg-card px-3 py-2 font-mono text-xs"
          >
            {["plaintiff", "defendant", "petitioner", "respondent", "third_party", "witness", "attorney", "agency", "court", "judge"].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
          <select
            value={partyType}
            onChange={(event) => setPartyType(event.target.value)}
            className="rounded border border-border bg-card px-3 py-2 font-mono text-xs"
          >
            {["individual", "entity", "government", "court"].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
          <input
            value={representedBy}
            onChange={(event) => setRepresentedBy(event.target.value)}
            placeholder="Represented by"
            className="rounded border border-border bg-card px-3 py-2 text-sm focus:border-primary focus:outline-none"
          />
          <input
            value={contactEmail}
            onChange={(event) => setContactEmail(event.target.value)}
            placeholder="Email"
            className="rounded border border-border bg-card px-3 py-2 text-sm focus:border-primary focus:outline-none"
          />
          <input
            value={contactPhone}
            onChange={(event) => setContactPhone(event.target.value)}
            placeholder="Phone"
            className="rounded border border-border bg-card px-3 py-2 text-sm focus:border-primary focus:outline-none"
          />
          <textarea
            value={notes}
            onChange={(event) => setNotes(event.target.value)}
            rows={2}
            placeholder="Notes"
            className="rounded border border-border bg-card px-3 py-2 text-sm focus:border-primary focus:outline-none md:col-span-2"
          />
          <button
            type="button"
            onClick={onCreate}
            disabled={saving}
            className="flex items-center justify-center gap-1.5 rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Plus className="h-3.5 w-3.5" />
            {saving ? "saving" : "add party"}
          </button>
        </div>
        {error && <p className="mt-2 text-xs text-destructive">{error}</p>}
      </section>

      <main className="grid grid-cols-1 gap-3 px-6 py-6 lg:grid-cols-2 xl:grid-cols-3">
        {matter.parties.length === 0 ? (
          <section className="rounded border border-dashed border-border p-6 text-sm text-muted-foreground">
            No parties have been added yet.
          </section>
        ) : (
          matter.parties.map((party) => {
            const Icon = party.partyType === "individual" ? UserRound : Building2
            return (
              <article key={party.id} className="rounded border border-border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded bg-primary/10 text-primary">
                    <Icon className="h-4 w-4" />
                  </div>
                  <div className="min-w-0">
                    <h2 className="truncate text-sm font-semibold text-foreground">{party.name}</h2>
                    <div className="mt-1 flex flex-wrap gap-1.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                      <span className="rounded bg-muted px-1.5 py-0.5">{party.role}</span>
                      <span className="rounded bg-muted px-1.5 py-0.5">{party.partyType}</span>
                    </div>
                  </div>
                </div>
                {party.representedBy && (
                  <p className="mt-3 text-xs text-muted-foreground">Represented by {party.representedBy}</p>
                )}
                <div className="mt-3 space-y-1 text-xs text-muted-foreground">
                  {party.contactEmail && (
                    <div className="flex items-center gap-1.5">
                      <Mail className="h-3 w-3" />
                      {party.contactEmail}
                    </div>
                  )}
                  {party.contactPhone && (
                    <div className="flex items-center gap-1.5">
                      <Phone className="h-3 w-3" />
                      {party.contactPhone}
                    </div>
                  )}
                </div>
                {party.notes && <p className="mt-3 text-xs leading-relaxed text-muted-foreground">{party.notes}</p>}
              </article>
            )
          })
        )}
      </main>
    </div>
  )
}
