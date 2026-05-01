const API_BASE = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || process.env.ORS_API_BASE_URL || "http://localhost:8080/api/v1"
const API_KEY = process.env.NEXT_PUBLIC_ORS_API_KEY || process.env.ORS_API_KEY

const headers = API_KEY ? { "x-api-key": API_KEY } : {}
let matterId = null

async function request(path, options = {}) {
  const response = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      ...headers,
      ...(options.body && typeof options.body === "string" ? { "content-type": "application/json" } : {}),
      ...(options.headers ?? {}),
    },
  })
  const body = await response.json().catch(() => ({}))
  if (!response.ok) {
    const error = new Error(body.error || `${response.status} ${response.statusText}`)
    error.status = response.status
    error.body = body
    throw error
  }
  return body
}

async function main() {
  const matter = await request("/matters", {
    method: "POST",
    body: JSON.stringify({
      name: `CaseBuilder smoke ${Date.now()}`,
      matter_type: "civil",
      user_role: "plaintiff",
      jurisdiction: "Oregon",
      court: "Smoke Court",
    }),
  })
  matterId = matter.matter_id || matter.id
  assert(matterId, "matter id returned")

  const form = new FormData()
  form.append(
    "file",
    new Blob([
      "Tenant reported mold on April 1, 2026. Landlord accepted rent after notice. Repairs were not completed for two weeks.",
    ], { type: "text/plain" }),
    "smoke-narrative.txt",
  )
  form.append("document_type", "evidence")
  form.append("folder", "Smoke")
  form.append("confidentiality", "private")

  const document = await request(`/matters/${encodeURIComponent(matterId)}/files/binary`, {
    method: "POST",
    body: form,
  })
  assert(document.document_id, "binary upload returned document")
  assert(document.storage_status === "stored", "document stored")

  const extraction = await request(
    `/matters/${encodeURIComponent(matterId)}/documents/${encodeURIComponent(document.document_id)}/extract`,
    { method: "POST" },
  )
  assert(extraction.status === "processed", "document extracted")
  assert(extraction.proposed_facts?.length > 0, "proposed facts returned")
  const fact = extraction.proposed_facts[0]

  const approvedFact = await request(
    `/matters/${encodeURIComponent(matterId)}/facts/${encodeURIComponent(fact.fact_id || fact.id)}/approve`,
    { method: "POST" },
  )
  assert(approvedFact.status === "supported", "fact approved")

  await request(`/matters/${encodeURIComponent(matterId)}/timeline`, {
    method: "POST",
    body: JSON.stringify({
      date: "2026-04-01",
      title: "Tenant reported mold",
      kind: "notice",
      source_document_id: document.document_id,
      linked_fact_ids: [approvedFact.fact_id || approvedFact.id],
    }),
  })

  const claim = await request(`/matters/${encodeURIComponent(matterId)}/claims`, {
    method: "POST",
    body: JSON.stringify({
      title: "Habitability",
      claim_type: "habitability",
      legal_theory: "Landlord failed to repair after notice.",
      fact_ids: [approvedFact.fact_id || approvedFact.id],
      elements: [
        {
          text: "Notice of condition",
          fact_ids: [approvedFact.fact_id || approvedFact.id],
        },
      ],
    }),
  })
  await request(
    `/matters/${encodeURIComponent(matterId)}/claims/${encodeURIComponent(claim.claim_id || claim.id)}/map-elements`,
    { method: "POST" },
  )

  await request(`/matters/${encodeURIComponent(matterId)}/evidence`, {
    method: "POST",
    body: JSON.stringify({
      document_id: document.document_id,
      quote: "Tenant reported mold on April 1, 2026.",
      source_span: "smoke quote",
      supports_fact_ids: [approvedFact.fact_id || approvedFact.id],
    }),
  })

  await request(`/matters/${encodeURIComponent(matterId)}/authority/attach`, {
    method: "POST",
    body: JSON.stringify({
      target_type: "claim",
      target_id: claim.claim_id || claim.id,
      citation: "ORS 90.320",
      canonical_id: "ORS 90.320",
      reason: "Smoke authority attachment",
    }),
  })

  const draft = await request(`/matters/${encodeURIComponent(matterId)}/drafts`, {
    method: "POST",
    body: JSON.stringify({ title: "Smoke complaint", draft_type: "complaint" }),
  })
  await request(
    `/matters/${encodeURIComponent(matterId)}/drafts/${encodeURIComponent(draft.draft_id || draft.id)}/generate`,
    { method: "POST" },
  )
  await request(
    `/matters/${encodeURIComponent(matterId)}/drafts/${encodeURIComponent(draft.draft_id || draft.id)}/fact-check`,
    { method: "POST" },
  )
  await request(
    `/matters/${encodeURIComponent(matterId)}/drafts/${encodeURIComponent(draft.draft_id || draft.id)}/citation-check`,
    { method: "POST" },
  )

  try {
    await request(`/matters/${encodeURIComponent(matterId)}/export/docx`, { method: "POST" })
    throw new Error("export/docx unexpectedly succeeded")
  } catch (error) {
    if (error.status !== 400 || !String(error.body?.error ?? "").includes("Export is deferred")) {
      throw error
    }
  }

  console.log("CaseBuilder V0 smoke passed")
}

function assert(value, message) {
  if (!value) throw new Error(`Smoke assertion failed: ${message}`)
}

main()
  .catch((error) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(async () => {
    if (!matterId) return
    await fetch(`${API_BASE}/matters/${encodeURIComponent(matterId)}`, {
      method: "DELETE",
      headers,
    }).catch(() => {})
  })
