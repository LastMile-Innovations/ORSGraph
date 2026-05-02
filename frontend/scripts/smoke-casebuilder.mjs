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

  const emptyComplaint = await request(`/matters/${encodeURIComponent(matterId)}/complaints`, {
    method: "POST",
    body: JSON.stringify({ title: "Empty graph complaint" }),
  })
  assert(emptyComplaint.paragraphs?.length > 0, "empty matter complaint still has a usable paragraph")
  assert(emptyComplaint.next_actions?.length > 0, "empty matter complaint has next actions")

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
  form.append("relative_path", "Smoke/Narratives/smoke-narrative.txt")
  form.append("upload_batch_id", `smoke:${Date.now()}`)

  const document = await request(`/matters/${encodeURIComponent(matterId)}/files/binary`, {
    method: "POST",
    body: form,
  })
  assert(document.document_id, "binary upload returned document")
  assert(document.storage_status === "stored", "document stored")
  assert(document.original_relative_path === "Smoke/Narratives/smoke-narrative.txt", "document preserved private relative path")

  const indexRun = await request(`/matters/${encodeURIComponent(matterId)}/index/run`, {
    method: "POST",
    body: JSON.stringify({ document_ids: [document.document_id] }),
  })
  assert(indexRun.processed === 1, "matter index run processed uploaded document")
  assert(indexRun.summary.indexed_documents >= 1, "matter index summary tracks indexed documents")

  const extraction = await request(
    `/matters/${encodeURIComponent(matterId)}/documents/${encodeURIComponent(document.document_id)}/extract`,
    { method: "POST" },
  )
  assert(extraction.status === "processed", "document extraction remains readable after indexing")
  assert(extraction.proposed_facts?.length > 0, "proposed facts returned")
  assert(extraction.source_spans?.length > 0, "source spans returned")
  assert(extraction.index_run?.status === "review_ready", "index run provenance returned")
  assert(extraction.artifact_manifest?.normalized_text_version_id, "artifact manifest references normalized text")
  assert(extraction.index_artifacts?.some((artifact) => artifact.artifact_kind === "text.normalized.json"), "normalized text artifact stored")
  assert(extraction.text_chunks?.length > 0, "index text chunks returned")
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

  const evidence = await request(`/matters/${encodeURIComponent(matterId)}/evidence`, {
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

  const complaint = await request(`/matters/${encodeURIComponent(matterId)}/complaints`, {
    method: "POST",
    body: JSON.stringify({ title: "Smoke complaint" }),
  })
  assert(complaint.complaint_id, "complaint id returned")
  assert(complaint.paragraphs?.length > 0, "complaint seeded paragraphs")
  assert(complaint.counts?.length > 0, "complaint seeded counts from claim")

  const paragraph = await request(
    `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/paragraphs`,
    {
      method: "POST",
      body: JSON.stringify({
        text: "Defendant failed to make repairs after written notice.",
        role: "factual_allegation",
        fact_ids: [approvedFact.fact_id || approvedFact.id],
        evidence_ids: [evidence.evidence_id || evidence.id],
      }),
    },
  )
  const paragraphId = paragraph.paragraphs.at(-1).paragraph_id
  await request(
    `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/links`,
    {
      method: "POST",
      body: JSON.stringify({
        target_type: "paragraph",
        target_id: paragraphId,
        citation: "ORS 90.320",
        canonical_id: "ORS 90.320",
      }),
    },
  )
  const qc = await request(
    `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/qc/run`,
    { method: "POST" },
  )
  assert(qc.mode === "deterministic", "complaint qc is deterministic")
  const findings = qc.result || []
  if (findings[0]) {
    await request(
      `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/qc/findings/${encodeURIComponent(findings[0].finding_id)}`,
      {
        method: "PATCH",
        body: JSON.stringify({ status: "ignored" }),
      },
    )
  }
  const preview = await request(
    `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/preview`,
  )
  assert(preview.html?.includes("court-paper"), "complaint preview generated")
  const artifact = await request(
    `/matters/${encodeURIComponent(matterId)}/complaints/${encodeURIComponent(complaint.complaint_id)}/export`,
    {
      method: "POST",
      body: JSON.stringify({ format: "html", include_exhibits: true, include_qc_report: true }),
    },
  )
  assert(artifact.artifact_id, "complaint export artifact returned")

  const workProduct = await request(`/matters/${encodeURIComponent(matterId)}/work-products`, {
    method: "POST",
    body: JSON.stringify({ title: "Smoke motion", product_type: "motion" }),
  })
  assert(workProduct.document_ast?.blocks?.length > 0, "work product returned canonical AST")
  const firstBlock = workProduct.document_ast.blocks[0]
  const workProductSnapshots = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/snapshots`,
  )
  const latestWorkProductSnapshot = [...workProductSnapshots].sort(
    (left, right) => (right.sequence_number ?? 0) - (left.sequence_number ?? 0),
  )[0]
  assert(latestWorkProductSnapshot?.document_hash, "work product snapshot has document hash")
  const patchedWorkProduct = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/patch`,
    {
      method: "POST",
      body: JSON.stringify({
        patch_id: `${workProduct.work_product_id}:patch:smoke`,
        work_product_id: workProduct.work_product_id,
        base_document_hash: latestWorkProductSnapshot.document_hash,
        created_by: "system",
        reason: "Smoke AST patch",
        created_at: `${Date.now()}`,
        operations: [
          {
            op: "update_block",
            block_id: firstBlock.block_id,
            after: { text: "Smoke AST patch updated this motion block." },
          },
        ],
      }),
    },
  )
  assert(
    patchedWorkProduct.document_ast.blocks[0].text.includes("Smoke AST patch"),
    "AST patch updated block text",
  )
  await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/links`,
    {
      method: "POST",
      body: JSON.stringify({
        block_id: firstBlock.block_id,
        target_type: "fact",
        target_id: approvedFact.fact_id || approvedFact.id,
        relation: "supports",
      }),
    },
  )
  const rangeFact = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/text-ranges`,
    {
      method: "POST",
      body: JSON.stringify({
        block_id: firstBlock.block_id,
        start_offset: 0,
        end_offset: 15,
        quote: "Smoke AST patch",
        target_type: "fact",
        target_id: approvedFact.fact_id || approvedFact.id,
        relation: "supports",
      }),
    },
  )
  assert(
    rangeFact.document_ast?.links?.some(
      (link) => link.target_type === "fact" && link.source_text_range?.quote === "Smoke AST patch",
    ),
    "selected text support link persisted",
  )
  const rangeCitation = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/text-ranges`,
    {
      method: "POST",
      body: JSON.stringify({
        block_id: firstBlock.block_id,
        start_offset: 0,
        end_offset: 15,
        quote: "Smoke AST patch",
        target_type: "authority",
        target_id: "ORS 90.320",
        relation: "cites",
        citation: "ORS 90.320",
        canonical_id: "ORS 90.320",
      }),
    },
  )
  assert(
    rangeCitation.document_ast?.citations?.some(
      (citation) => citation.raw_text === "ORS 90.320" && citation.source_text_range?.quote === "Smoke AST patch",
    ),
    "selected text citation persisted",
  )
  const rangeExhibit = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/text-ranges`,
    {
      method: "POST",
      body: JSON.stringify({
        block_id: firstBlock.block_id,
        start_offset: 0,
        end_offset: 15,
        quote: "Smoke AST patch",
        target_type: "document",
        target_id: document.document_id,
        relation: "authenticates",
        exhibit_label: "Smoke Exhibit",
        document_id: document.document_id,
      }),
    },
  )
  assert(
    rangeExhibit.document_ast?.exhibits?.some(
      (exhibit) => exhibit.label === "Smoke Exhibit" && exhibit.source_text_range?.quote === "Smoke AST patch",
    ),
    "selected text exhibit persisted",
  )
  const astCompare = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/compare?from=${encodeURIComponent(latestWorkProductSnapshot.snapshot_id)}&layers=all`,
  )
  assert(
    astCompare.layer_diffs?.some((diff) => diff.layer === "support"),
    "AST compare reports support-layer changes",
  )
  const astValidation = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/validate`,
    { method: "POST" },
  )
  assert(astValidation.valid, "AST validation passed")
  const markdownProjection = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/to-markdown`,
    { method: "POST" },
  )
  assert(markdownProjection.markdown?.includes("Smoke motion"), "AST converts to markdown")
  const markdownAst = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/from-markdown`,
    {
      method: "POST",
      body: JSON.stringify({ markdown: "## COUNT I - Smoke\n\n1. Smoke allegation." }),
    },
  )
  assert(markdownAst.document_ast?.blocks?.length === 2, "markdown converts to AST")
  const astHtml = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/to-html`,
    { method: "POST" },
  )
  assert(astHtml.html?.includes("work-product-preview"), "AST converts to HTML")
  const astText = await request(
    `/matters/${encodeURIComponent(matterId)}/work-products/${encodeURIComponent(workProduct.work_product_id)}/ast/to-plain-text`,
    { method: "POST" },
  )
  assert(astText.plain_text?.includes("Smoke motion"), "AST converts to plain text")

  const docxPackage = await request(`/matters/${encodeURIComponent(matterId)}/export/docx`, { method: "POST" })
  assert(docxPackage.result?.format === "docx", "matter DOCX export package returned")
  assert(
    docxPackage.result?.warnings?.some((warning) => warning.includes("renderer")),
    "DOCX export reports renderer status",
  )

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
