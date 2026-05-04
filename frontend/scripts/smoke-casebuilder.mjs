const API_BASE = process.env.ORS_API_BASE_URL || "http://localhost:8080/api/v1"
const API_KEY = process.env.ORS_API_KEY
const UPLOAD_CORS_ORIGIN = process.env.ORS_SMOKE_UPLOAD_ORIGIN?.trim() || ""

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

  const uploadBatchId = `smoke:${Date.now()}`
  const document = await signedUpload(matterId, {
    bytes: "Tenant reported mold on April 1, 2026. Landlord accepted rent after notice. Repairs were not completed for two weeks.",
    document_type: "evidence",
    filename: "smoke-narrative.md",
    folder: "Smoke",
    mime_type: "text/markdown",
    relative_path: "Smoke/Narratives/smoke-narrative.md",
    upload_batch_id: uploadBatchId,
  })
  assert(document.document_id, "signed upload returned document")
  assert(document.storage_status === "stored", "document stored")
  assert(document.original_relative_path === "Smoke/Narratives/smoke-narrative.md", "document preserved private relative path")

  const viewOnlyDocument = await signedUpload(matterId, {
    bytes: "%PDF-1.4\nstored only",
    document_type: "evidence",
    filename: "stored-only.pdf",
    folder: "Smoke",
    mime_type: "application/pdf",
    relative_path: "Smoke/Narratives/stored-only.pdf",
    upload_batch_id: `${uploadBatchId}:view`,
  })
  assert(viewOnlyDocument.storage_status === "stored", "non-Markdown document stored")
  assert(viewOnlyDocument.processing_status === "view_only", "non-Markdown document stays view-only")

  const indexJob = await runIndexJob(matterId, {
    document_ids: [document.document_id],
    upload_batch_id: uploadBatchId,
  })
  assert(indexJob.processed === 1, "matter index job processed uploaded document")
  assert(indexJob.summary.indexed_documents >= 1, "matter index summary tracks indexed documents")

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
  assert(indexJob.produced_timeline_suggestions >= 1, "matter index job reports timeline suggestions")
  assert(extraction.timeline_suggestions?.length > 0, "reviewable timeline suggestions returned")
  assert(extraction.timeline_suggestions[0].status === "suggested", "timeline suggestion is review-first")
  assert(extraction.timeline_suggestions[0].agent_run_id, "indexed suggestion points to a timeline agent run")

  const skippedIndexJob = await runIndexJob(matterId, {
    document_ids: [viewOnlyDocument.document_id],
    upload_batch_id: `${uploadBatchId}:view`,
  })
  assert(skippedIndexJob.processed === 0, "non-Markdown document is not indexed")
  assert(skippedIndexJob.skipped === 1, "non-Markdown document is reported as skipped")

  const fact = extraction.proposed_facts[0]

  const approvedFact = await request(
    `/matters/${encodeURIComponent(matterId)}/facts/${encodeURIComponent(fact.fact_id || fact.id)}/approve`,
    { method: "POST" },
  )
  assert(approvedFact.status === "supported", "fact approved")

  const editedSuggestion = await request(
    `/matters/${encodeURIComponent(matterId)}/timeline/suggestions/${encodeURIComponent(extraction.timeline_suggestions[0].suggestion_id)}`,
    {
      method: "PATCH",
      body: JSON.stringify({ title: "Smoke edited April 1 notice" }),
    },
  )
  assert(editedSuggestion.title === "Smoke edited April 1 notice", "timeline suggestion title can be edited before approval")

  const timelineApproval = await request(
    `/matters/${encodeURIComponent(matterId)}/timeline/suggestions/${encodeURIComponent(editedSuggestion.suggestion_id)}/approve`,
    { method: "POST" },
  )
  assert(timelineApproval.suggestion?.status === "approved", "timeline suggestion approved")
  assert(timelineApproval.event?.date === "2026-04-01", "approved timeline event keeps extracted date")
  assert(timelineApproval.event?.title === "Smoke edited April 1 notice", "approved event uses reviewed suggestion title")

  const timelineSuggest = await request(`/matters/${encodeURIComponent(matterId)}/timeline/suggest`, {
    method: "POST",
    body: JSON.stringify({ document_ids: [document.document_id], limit: 5 }),
  })
  assert(timelineSuggest.agent_run?.provider_mode === "template", "provider-free timeline agent run recorded")
  assert(timelineSuggest.agent_run?.agent_type === "timeline_builder", "timeline agent run records harness agent type")
  assert(timelineSuggest.agent_run?.scope_type === "document", "timeline agent run records request scope")
  assert(timelineSuggest.agent_run?.input_hash, "timeline agent run records input hash")
  assert(timelineSuggest.agent_run?.pipeline_version, "timeline agent run records pipeline version")
  assert(timelineSuggest.agent_run?.extractor_version, "timeline agent run records extractor version")
  assert(timelineSuggest.agent_run?.deterministic_candidate_count >= 1, "timeline agent run records deterministic candidate count")
  assert(timelineSuggest.agent_run?.stored_suggestion_count >= 1, "timeline agent run records stored suggestion count")
  assert(timelineSuggest.suggestions?.every((suggestion) => suggestion.agent_run_id === timelineSuggest.agent_run.agent_run_id), "suggestions point to the returned agent run")

  const agentRuns = await request(`/matters/${encodeURIComponent(matterId)}/timeline/agent-runs`)
  assert(agentRuns.some((run) => run.agent_run_id === timelineSuggest.agent_run.agent_run_id), "timeline agent run list includes latest run")
  const agentRun = await request(
    `/matters/${encodeURIComponent(matterId)}/timeline/agent-runs/${encodeURIComponent(timelineSuggest.agent_run.agent_run_id)}`,
  )
  assert(agentRun.agent_run_id === timelineSuggest.agent_run.agent_run_id, "timeline agent run detail is reachable")

  const graph = await request(`/matters/${encodeURIComponent(matterId)}/graph`)
  assert(
    graph.nodes?.some((node) => node.kind === "event" && node.id === timelineApproval.event.event_id),
    "approved timeline event appears in matter graph",
  )
  assert(
    graph.nodes?.some((node) => node.kind === "timeline_agent_run" && node.id === timelineSuggest.agent_run.agent_run_id),
    "timeline agent run appears in matter graph",
  )

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

async function signedUpload(matterId, input) {
  const blob = new Blob([input.bytes], { type: input.mime_type })
  const intent = await request(`/matters/${encodeURIComponent(matterId)}/files/uploads`, {
    method: "POST",
    body: JSON.stringify({
      bytes: blob.size,
      confidentiality: "private",
      document_type: input.document_type,
      filename: input.filename,
      folder: input.folder,
      mime_type: input.mime_type,
      relative_path: input.relative_path,
      upload_batch_id: input.upload_batch_id,
    }),
  })

  if (UPLOAD_CORS_ORIGIN) {
    await assertSignedUploadCorsPreflight(intent, input)
  }

  const putHeaders = { ...(intent.headers ?? {}) }
  if (UPLOAD_CORS_ORIGIN) {
    putHeaders.Origin = UPLOAD_CORS_ORIGIN
  }

  const putResponse = await fetch(intent.url, {
    method: intent.method || "PUT",
    headers: putHeaders,
    body: blob,
  })
  if (!putResponse.ok) {
    throw new Error(`Signed upload PUT failed for ${input.filename}: ${putResponse.status} ${putResponse.statusText}`)
  }
  if (UPLOAD_CORS_ORIGIN) {
    assertSignedUploadCorsResponse(putResponse, input.filename)
  }

  return request(
    `/matters/${encodeURIComponent(matterId)}/files/uploads/${encodeURIComponent(intent.upload_id)}/complete`,
    {
      method: "POST",
      body: JSON.stringify({
        bytes: blob.size,
        document_id: intent.document_id,
        etag: putResponse.headers.get("etag"),
      }),
    },
  )
}

async function assertSignedUploadCorsPreflight(intent, input) {
  const requestHeaders = signedUploadRequestHeaderNames(intent, input)
  const headers = {
    Origin: UPLOAD_CORS_ORIGIN,
    "Access-Control-Request-Method": intent.method || "PUT",
  }
  if (requestHeaders) {
    headers["Access-Control-Request-Headers"] = requestHeaders
  }

  const response = await fetch(intent.url, {
    method: "OPTIONS",
    headers,
  })
  const allowedOrigin = response.headers.get("access-control-allow-origin")
  const allowedMethods = response.headers.get("access-control-allow-methods")
  const allowedHeaders = response.headers.get("access-control-allow-headers")

  assert(response.ok, `signed upload CORS preflight succeeded for ${input.filename}`)
  assert(
    allowedOrigin === "*" || allowedOrigin === UPLOAD_CORS_ORIGIN,
    `signed upload CORS allows ${UPLOAD_CORS_ORIGIN} for ${input.filename}`,
  )
  assert(
    headerListAllowsToken(allowedMethods, intent.method || "PUT"),
    `signed upload CORS allows ${intent.method || "PUT"} for ${input.filename}`,
  )
  for (const header of requestHeaders.split(",").filter(Boolean)) {
    assert(
      headerListAllowsToken(allowedHeaders, header),
      `signed upload CORS allows request header ${header} for ${input.filename}`,
    )
  }
}

function assertSignedUploadCorsResponse(response, filename) {
  const allowedOrigin = response.headers.get("access-control-allow-origin")
  const exposedHeaders = response.headers.get("access-control-expose-headers")
  assert(
    allowedOrigin === "*" || allowedOrigin === UPLOAD_CORS_ORIGIN,
    `signed upload response exposes ${UPLOAD_CORS_ORIGIN} for ${filename}`,
  )
  assert(headerListAllowsToken(exposedHeaders, "etag"), `signed upload response exposes ETag for ${filename}`)
}

function signedUploadRequestHeaderNames(intent, input) {
  const names = new Set(
    Object.keys(intent.headers ?? {})
      .map((name) => name.trim().toLowerCase())
      .filter(Boolean),
  )
  if (input.mime_type) {
    names.add("content-type")
  }
  return [...names].sort().join(",")
}

function headerListAllowsToken(value, token) {
  if (!value) return false
  const normalizedToken = token.trim().toLowerCase()
  return value
    .split(",")
    .map((part) => part.trim().toLowerCase())
    .some((part) => part === "*" || part === normalizedToken)
}

async function runIndexJob(matterId, input) {
  const job = await request(`/matters/${encodeURIComponent(matterId)}/index/jobs`, {
    method: "POST",
    body: JSON.stringify(input),
  })
  assert(job.index_job_id, "index job id returned")
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const latest = await request(
      `/matters/${encodeURIComponent(matterId)}/index/jobs/${encodeURIComponent(job.index_job_id)}`,
    )
    if (!["queued", "running"].includes(latest.status)) {
      return latest
    }
    await new Promise((resolve) => setTimeout(resolve, 750))
  }
  throw new Error(`Index job ${job.index_job_id} did not finish before smoke timeout`)
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
