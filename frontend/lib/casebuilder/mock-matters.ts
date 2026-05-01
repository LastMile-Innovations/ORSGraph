// Demo matter data for CaseBuilder.
// Matter 1: Smith v. ABC Property Management (Multnomah County) — landlord/tenant.
// User role: defendant landlord… actually plaintiff tenant who was wrongfully evicted.
// Matter 2: lighter, "Doe v. Acme Co" employment matter.
// Matter 3: a small claims matter at intake.
//
// This data is shaped to be drop-in replaceable by real API responses.

import type {
  Matter,
  MatterSummary,
  CaseDocument,
  MatterParty,
  CaseFact,
  CaseEvent,
  CaseEvidence,
  CaseClaim,
  CaseDefense,
  CaseDeadline,
  CaseTask,
  CaseDraft,
  DocumentExtraction,
  MatterAskMessage,
  MatterChatCitation,
  MatterChatMessage,
} from "./types"

// ===== Matters list =====

export const matters: MatterSummary[] = [
  {
    matter_id: "matter:smith-abc",
    name: "Smith v. ABC Property Management",
    matter_type: "landlord_tenant",
    status: "active",
    user_role: "plaintiff",
    jurisdiction: "Oregon",
    court: "Multnomah County Circuit Court",
    case_number: "25CV12384",
    created_at: "2026-03-02T09:14:00Z",
    updated_at: "2026-04-26T17:42:00Z",
    document_count: 14,
    fact_count: 22,
    evidence_count: 19,
    claim_count: 3,
    draft_count: 2,
    open_task_count: 6,
    next_deadline: { description: "Answer due", due_date: "2026-05-12", days_remaining: 14 },
  },
  {
    matter_id: "matter:doe-acme",
    name: "Doe v. Acme Co",
    matter_type: "employment",
    status: "active",
    user_role: "plaintiff",
    jurisdiction: "Oregon",
    court: "U.S. District Court, D. Oregon",
    case_number: "3:26-cv-00114-HZ",
    created_at: "2026-02-10T11:05:00Z",
    updated_at: "2026-04-22T08:11:00Z",
    document_count: 9,
    fact_count: 14,
    evidence_count: 11,
    claim_count: 2,
    draft_count: 1,
    open_task_count: 3,
    next_deadline: { description: "Initial disclosures", due_date: "2026-05-20", days_remaining: 22 },
  },
  {
    matter_id: "matter:nguyen-pdx",
    name: "Nguyen v. PDX Towing",
    matter_type: "small_claims",
    status: "intake",
    user_role: "plaintiff",
    jurisdiction: "Oregon",
    court: "Multnomah County Small Claims",
    case_number: null,
    created_at: "2026-04-25T14:32:00Z",
    updated_at: "2026-04-27T10:01:00Z",
    document_count: 5,
    fact_count: 8,
    evidence_count: 6,
    claim_count: 1,
    draft_count: 0,
    open_task_count: 4,
    next_deadline: null,
  },
  {
    matter_id: "matter:rivera-osp",
    name: "Rivera v. Oregon State Police (Records)",
    matter_type: "admin",
    status: "active",
    user_role: "petitioner",
    jurisdiction: "Oregon",
    court: "Oregon DOJ Public Records",
    case_number: "PRR-2026-0481",
    created_at: "2026-03-18T16:00:00Z",
    updated_at: "2026-04-20T13:25:00Z",
    document_count: 7,
    fact_count: 10,
    evidence_count: 7,
    claim_count: 1,
    draft_count: 1,
    open_task_count: 2,
    next_deadline: { description: "DOJ petition response", due_date: "2026-05-30", days_remaining: 32 },
  },
]

// ===== The flagship demo: matter:smith-abc =====

export const matterSmithAbc: MatterSummary = matters[0]

export const partiesSmithAbc = [
  {
    party_id: "party:smith",
    matter_id: "matter:smith-abc",
    name: "Sarah Smith",
    role: "plaintiff",
    party_type: "individual",
    represented_by: null,
    contact_email: "sarah.smith@example.com",
    notes: "Tenant of unit 4B, 1418 NE Alberta St, Portland.",
  },
  {
    party_id: "party:abc",
    matter_id: "matter:smith-abc",
    name: "ABC Property Management LLC",
    role: "defendant",
    party_type: "entity",
    represented_by: "Halverson & Pratt, LLP",
    notes: "Property manager for landlord Northpoint Holdings.",
  },
  {
    party_id: "party:northpoint",
    matter_id: "matter:smith-abc",
    name: "Northpoint Holdings LLC",
    role: "defendant",
    party_type: "entity",
    represented_by: "Halverson & Pratt, LLP",
    notes: "Owner of record per RLID.",
  },
  {
    party_id: "party:hon-lee",
    matter_id: "matter:smith-abc",
    name: "Hon. Marcus Lee",
    role: "judge",
    party_type: "court",
    represented_by: null,
  },
]

// ===== Documents =====

export const documentsSmithAbc = [
  {
    document_id: "doc:complaint",
    matter_id: "matter:smith-abc",
    filename: "FED_Complaint_Northpoint_v_Smith.pdf",
    document_type: "complaint",
    mime_type: "application/pdf",
    pages: 7,
    bytes: 218_440,
    file_hash: "sha256:9b1a…44df",
    uploaded_at: "2026-04-22T10:14:00Z",
    source: "user_upload",
    confidentiality: "filed",
    processing_status: "processed",
    is_exhibit: false,
    summary:
      "FED complaint by Northpoint Holdings against Sarah Smith for nonpayment, attaching a 72-hour notice dated March 14, 2026. Alleges $2,840 in unpaid rent and seeks restitution of premises plus costs.",
    date_observed: "2026-04-21",
    parties_mentioned: ["Sarah Smith", "Northpoint Holdings LLC", "ABC Property Management LLC"],
    entities_mentioned: ["Multnomah County Circuit Court", "1418 NE Alberta St"],
    facts_extracted: 9,
    citations_found: 4,
    contradictions_flagged: 2,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Pleadings",
  },
  {
    document_id: "doc:lease",
    matter_id: "matter:smith-abc",
    filename: "Smith_Lease_2024-08-01.pdf",
    document_type: "lease",
    mime_type: "application/pdf",
    pages: 12,
    bytes: 884_220,
    file_hash: "sha256:1c20…8aab",
    uploaded_at: "2026-04-22T10:14:18Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. A",
    summary:
      "12-month residential lease beginning 2024-08-01 at $1,420/mo. Section 14 requires written notice of termination by certified mail. No no-cause termination clause.",
    date_observed: "2024-08-01",
    parties_mentioned: ["Sarah Smith", "ABC Property Management LLC"],
    entities_mentioned: ["1418 NE Alberta St, Unit 4B"],
    facts_extracted: 7,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:wrongful-eviction", "claim:habitability"],
    folder: "Contracts",
  },
  {
    document_id: "doc:72hr-notice",
    matter_id: "matter:smith-abc",
    filename: "72hr_Notice_2026-03-14.pdf",
    document_type: "notice",
    mime_type: "application/pdf",
    pages: 1,
    bytes: 41_120,
    file_hash: "sha256:7711…2ed4",
    uploaded_at: "2026-04-22T10:14:31Z",
    source: "user_upload",
    confidentiality: "filed",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. B",
    summary:
      "72-hour notice of nonpayment posted on door. Date of service stated as March 14, 2026. No proof of certified mailing.",
    date_observed: "2026-03-14",
    parties_mentioned: ["ABC Property Management LLC", "Sarah Smith"],
    entities_mentioned: [],
    facts_extracted: 4,
    citations_found: 1,
    contradictions_flagged: 1,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Notices",
  },
  {
    document_id: "doc:rent-ledger",
    matter_id: "matter:smith-abc",
    filename: "Rent_Ledger_Smith_2024-2026.xlsx",
    document_type: "spreadsheet",
    mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    pages: 3,
    bytes: 28_810,
    file_hash: "sha256:441a…ff09",
    uploaded_at: "2026-04-22T10:15:02Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. C",
    summary:
      "Tenant rent ledger showing monthly $1,420 transfers from 2024-08 through 2026-02. February 2026 marked as paid by Zelle on 2026-02-28.",
    date_observed: "2026-02-28",
    parties_mentioned: [],
    entities_mentioned: ["Zelle"],
    facts_extracted: 6,
    citations_found: 0,
    contradictions_flagged: 1,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Evidence",
  },
  {
    document_id: "doc:zelle-confirm",
    matter_id: "matter:smith-abc",
    filename: "Zelle_Confirmation_2026-02-28.png",
    document_type: "screenshot",
    mime_type: "image/png",
    pages: 1,
    bytes: 145_200,
    file_hash: "sha256:0ffe…11ad",
    uploaded_at: "2026-04-22T10:15:14Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. D",
    summary: "Zelle screenshot confirming $1,420 payment to ABC Property Management on Feb 28, 2026.",
    date_observed: "2026-02-28",
    parties_mentioned: ["ABC Property Management LLC"],
    entities_mentioned: ["Zelle"],
    facts_extracted: 2,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Evidence",
  },
  {
    document_id: "doc:email-mar3",
    matter_id: "matter:smith-abc",
    filename: "Email_ABC_to_Smith_2026-03-03.eml",
    document_type: "email",
    mime_type: "message/rfc822",
    pages: 2,
    bytes: 12_400,
    file_hash: "sha256:9182…aa00",
    uploaded_at: "2026-04-22T10:15:30Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. E",
    summary:
      "Email from D. Halverson (ABC) to S. Smith on March 3, 2026 acknowledging Feb rent received and stating: \"all current as of today.\"",
    date_observed: "2026-03-03",
    parties_mentioned: ["ABC Property Management LLC", "Sarah Smith"],
    entities_mentioned: [],
    facts_extracted: 3,
    citations_found: 0,
    contradictions_flagged: 1,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Correspondence",
  },
  {
    document_id: "doc:mold-photos",
    matter_id: "matter:smith-abc",
    filename: "Bathroom_Mold_Photos.zip",
    document_type: "photo",
    mime_type: "application/zip",
    pages: 1,
    bytes: 4_280_110,
    file_hash: "sha256:55cd…7f2e",
    uploaded_at: "2026-04-22T10:16:01Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. F",
    summary: "11 photos of black mold in bathroom drywall and ceiling, EXIF dated 2025-11-04 to 2026-02-09.",
    date_observed: "2026-02-09",
    parties_mentioned: [],
    entities_mentioned: ["1418 NE Alberta St, Unit 4B"],
    facts_extracted: 4,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:habitability"],
    folder: "Evidence",
  },
  {
    document_id: "doc:repair-request",
    matter_id: "matter:smith-abc",
    filename: "Repair_Request_Letter_2025-11-12.pdf",
    document_type: "letter",
    mime_type: "application/pdf",
    pages: 2,
    bytes: 84_000,
    file_hash: "sha256:1188…cc31",
    uploaded_at: "2026-04-22T10:16:21Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. G",
    summary:
      "Certified letter from S. Smith to ABC requesting repair of mold and broken heater under ORS 90.320. Delivered Nov 14, 2025.",
    date_observed: "2025-11-12",
    parties_mentioned: ["Sarah Smith", "ABC Property Management LLC"],
    entities_mentioned: ["USPS"],
    facts_extracted: 3,
    citations_found: 1,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:habitability", "claim:retaliation"],
    folder: "Correspondence",
  },
  {
    document_id: "doc:police-report",
    matter_id: "matter:smith-abc",
    filename: "PPB_Lockout_Report_2026-04-18.pdf",
    document_type: "police",
    mime_type: "application/pdf",
    pages: 4,
    bytes: 412_000,
    file_hash: "sha256:f001…1992",
    uploaded_at: "2026-04-22T10:16:42Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: true,
    exhibit_label: "Ex. H",
    summary:
      "Portland Police report of lockout incident at unit 4B on 2026-04-18. Officers spoke with ABC representative who admitted changing locks before any court order.",
    date_observed: "2026-04-18",
    parties_mentioned: ["Sarah Smith", "ABC Property Management LLC"],
    entities_mentioned: ["Portland Police Bureau"],
    facts_extracted: 5,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:wrongful-eviction"],
    folder: "Evidence",
  },
  {
    document_id: "doc:summons",
    matter_id: "matter:smith-abc",
    filename: "Summons_25CV12384.pdf",
    document_type: "order",
    mime_type: "application/pdf",
    pages: 2,
    bytes: 64_220,
    file_hash: "sha256:3322…aa01",
    uploaded_at: "2026-04-22T10:17:00Z",
    source: "user_upload",
    confidentiality: "filed",
    processing_status: "processed",
    is_exhibit: false,
    summary: "Summons issued by Multnomah County Circuit Court. First appearance set for 2026-05-12 at 9:00 AM.",
    date_observed: "2026-04-21",
    parties_mentioned: ["Sarah Smith", "Northpoint Holdings LLC"],
    entities_mentioned: ["Multnomah County Circuit Court"],
    facts_extracted: 2,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: [],
    folder: "Court Orders",
  },
  {
    document_id: "doc:texts",
    matter_id: "matter:smith-abc",
    filename: "iMessage_export_smith_abc.txt",
    document_type: "other",
    mime_type: "text/plain",
    pages: 6,
    bytes: 22_400,
    file_hash: "sha256:7a72…44b1",
    uploaded_at: "2026-04-22T10:17:20Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: false,
    summary:
      "SMS thread between S. Smith and D. Halverson from Oct 2025 to April 2026, including mold complaints and an April 12 message: \"You need to be out by Friday or we change the locks.\"",
    date_observed: "2026-04-12",
    parties_mentioned: ["Sarah Smith", "ABC Property Management LLC"],
    entities_mentioned: [],
    facts_extracted: 4,
    citations_found: 0,
    contradictions_flagged: 1,
    linked_claim_ids: ["claim:wrongful-eviction", "claim:retaliation"],
    folder: "Correspondence",
  },
  {
    document_id: "doc:medical",
    matter_id: "matter:smith-abc",
    filename: "Smith_Medical_Records_OHSU.pdf",
    document_type: "medical",
    mime_type: "application/pdf",
    pages: 18,
    bytes: 1_220_000,
    file_hash: "sha256:8c11…ee04",
    uploaded_at: "2026-04-22T10:17:48Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: false,
    summary:
      "OHSU records 2025-12 to 2026-02 documenting respiratory symptoms attributed to environmental mold exposure.",
    date_observed: "2026-02-04",
    parties_mentioned: ["Sarah Smith"],
    entities_mentioned: ["OHSU"],
    facts_extracted: 3,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:habitability"],
    folder: "Evidence",
  },
  {
    document_id: "doc:rlid",
    matter_id: "matter:smith-abc",
    filename: "RLID_OwnerOfRecord_NorthpointHoldings.pdf",
    document_type: "public_record",
    mime_type: "application/pdf",
    pages: 1,
    bytes: 38_000,
    file_hash: "sha256:0001…aaef",
    uploaded_at: "2026-04-23T08:11:00Z",
    source: "public_records",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: false,
    summary: "RLID property record showing Northpoint Holdings LLC as owner of record for 1418 NE Alberta St.",
    date_observed: "2026-04-23",
    parties_mentioned: ["Northpoint Holdings LLC"],
    entities_mentioned: ["RLID"],
    facts_extracted: 1,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: [],
    folder: "Public Records",
  },
  {
    document_id: "doc:draft-answer",
    matter_id: "matter:smith-abc",
    filename: "Draft_Answer_v3.docx",
    document_type: "answer",
    mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    pages: 6,
    bytes: 32_400,
    file_hash: "sha256:fff0…1100",
    uploaded_at: "2026-04-26T17:42:00Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "processed",
    is_exhibit: false,
    summary: "Working draft answer with affirmative defenses and counterclaim for habitability and retaliation.",
    date_observed: null,
    parties_mentioned: ["Sarah Smith", "Northpoint Holdings LLC"],
    entities_mentioned: [],
    facts_extracted: 0,
    citations_found: 6,
    contradictions_flagged: 0,
    linked_claim_ids: ["claim:wrongful-eviction", "claim:habitability", "claim:retaliation"],
    folder: "Drafts",
  },
]

// ===== Facts =====

export const factsSmithAbc = [
  {
    fact_id: "fact:lease-start",
    matter_id: "matter:smith-abc",
    text: "Sarah Smith executed a 12-month residential lease for 1418 NE Alberta St, Unit 4B beginning August 1, 2024.",
    fact_type: "agreement",
    date: "2024-08-01",
    status: "supported",
    confidence: 0.99,
    source_evidence_ids: ["evidence:lease-sec1"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:wrongful-eviction", "claim:habitability"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    party_id: "party:smith",
    needs_verification: false,
  },
  {
    fact_id: "fact:rent-paid-feb",
    matter_id: "matter:smith-abc",
    text: "On February 28, 2026 Sarah Smith paid $1,420 in rent via Zelle to ABC Property Management.",
    fact_type: "payment",
    date: "2026-02-28",
    status: "supported",
    confidence: 0.97,
    source_evidence_ids: ["evidence:zelle", "evidence:ledger-feb"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:wrongful-eviction"],
    supports_defense_ids: ["defense:no-default"],
    used_in_draft_ids: ["draft:answer-v3"],
    party_id: "party:smith",
    needs_verification: false,
  },
  {
    fact_id: "fact:abc-acknowledged",
    matter_id: "matter:smith-abc",
    text: "On March 3, 2026 ABC Property Management emailed Sarah Smith stating her account was \"all current as of today.\"",
    fact_type: "communication",
    date: "2026-03-03",
    status: "supported",
    confidence: 0.95,
    source_evidence_ids: ["evidence:email-mar3"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:wrongful-eviction"],
    supports_defense_ids: ["defense:no-default"],
    used_in_draft_ids: ["draft:answer-v3"],
    party_id: "party:abc",
    needs_verification: false,
  },
  {
    fact_id: "fact:72hr-posted",
    matter_id: "matter:smith-abc",
    text: "ABC posted a 72-hour notice of nonpayment on Smith's door on March 14, 2026.",
    fact_type: "notice",
    date: "2026-03-14",
    status: "disputed",
    confidence: 0.78,
    source_evidence_ids: ["evidence:notice-stamp"],
    contradicted_by_evidence_ids: ["evidence:email-mar3"],
    supports_claim_ids: [],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    party_id: "party:abc",
    needs_verification: true,
    notes: "Contradicts the March 3 email confirming current account status.",
  },
  {
    fact_id: "fact:no-cert-mail",
    matter_id: "matter:smith-abc",
    text: "ABC did not deliver the 72-hour notice by certified mail as required by Section 14 of the lease and ORS 90.155.",
    fact_type: "violation",
    date: "2026-03-14",
    status: "alleged",
    confidence: 0.7,
    source_evidence_ids: ["evidence:notice-stamp", "evidence:lease-sec14"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:wrongful-eviction"],
    supports_defense_ids: ["defense:improper-notice"],
    used_in_draft_ids: ["draft:answer-v3"],
    needs_verification: true,
  },
  {
    fact_id: "fact:mold-existed",
    matter_id: "matter:smith-abc",
    text: "Black mold was present in the unit's bathroom from at least November 2025 through February 2026.",
    fact_type: "condition",
    date: "2025-11-04",
    status: "supported",
    confidence: 0.93,
    source_evidence_ids: ["evidence:photo-mold-1", "evidence:medical-feb"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:habitability"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    needs_verification: false,
  },
  {
    fact_id: "fact:repair-requested",
    matter_id: "matter:smith-abc",
    text: "Smith sent a certified habitability repair request to ABC on November 12, 2025; delivered November 14, 2025.",
    fact_type: "communication",
    date: "2025-11-12",
    status: "supported",
    confidence: 0.96,
    source_evidence_ids: ["evidence:repair-letter"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:habitability", "claim:retaliation"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    party_id: "party:smith",
    needs_verification: false,
  },
  {
    fact_id: "fact:no-repair",
    matter_id: "matter:smith-abc",
    text: "ABC did not perform repairs within the 7-day cure window required by ORS 90.320.",
    fact_type: "violation",
    date: "2025-11-21",
    status: "alleged",
    confidence: 0.85,
    source_evidence_ids: ["evidence:repair-letter", "evidence:photo-mold-feb"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:habitability"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    needs_verification: false,
  },
  {
    fact_id: "fact:lockout",
    matter_id: "matter:smith-abc",
    text: "On April 18, 2026 ABC changed the locks on Unit 4B without a court order; Portland Police responded.",
    fact_type: "event",
    date: "2026-04-18",
    status: "supported",
    confidence: 0.98,
    source_evidence_ids: ["evidence:police-report"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:wrongful-eviction"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    needs_verification: false,
  },
  {
    fact_id: "fact:retaliation-timing",
    matter_id: "matter:smith-abc",
    text: "ABC's nonpayment notice was issued less than 6 months after Smith's habitability complaint, raising a presumption of retaliation under ORS 90.385.",
    fact_type: "violation",
    date: "2026-03-14",
    status: "alleged",
    confidence: 0.74,
    source_evidence_ids: ["evidence:repair-letter", "evidence:notice-stamp"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:retaliation"],
    supports_defense_ids: [],
    used_in_draft_ids: ["draft:answer-v3"],
    needs_verification: true,
  },
  {
    fact_id: "fact:medical-impact",
    matter_id: "matter:smith-abc",
    text: "Smith was treated at OHSU on December 8, 2025 and February 4, 2026 for mold-related respiratory symptoms.",
    fact_type: "injury",
    date: "2025-12-08",
    status: "supported",
    confidence: 0.9,
    source_evidence_ids: ["evidence:medical-feb"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: ["claim:habitability"],
    supports_defense_ids: [],
    used_in_draft_ids: [],
    needs_verification: false,
  },
  {
    fact_id: "fact:abc-not-owner",
    matter_id: "matter:smith-abc",
    text: "ABC Property Management is not the owner of record; Northpoint Holdings LLC is.",
    fact_type: "statement",
    date: null,
    status: "supported",
    confidence: 0.99,
    source_evidence_ids: ["evidence:rlid"],
    contradicted_by_evidence_ids: [],
    supports_claim_ids: [],
    supports_defense_ids: ["defense:improper-party"],
    used_in_draft_ids: [],
    needs_verification: false,
  },
]

// ===== Events / Timeline =====

export const eventsSmithAbc = [
  {
    event_id: "event:lease-start",
    matter_id: "matter:smith-abc",
    date: "2024-08-01",
    date_confidence: 0.99,
    description: "Lease commencement at 1418 NE Alberta St, Unit 4B.",
    category: "filing",
    source_document_id: "doc:lease",
    party_ids: ["party:smith", "party:abc"],
    linked_fact_ids: ["fact:lease-start"],
    linked_claim_ids: [],
    disputed: false,
  },
  {
    event_id: "event:repair-request",
    matter_id: "matter:smith-abc",
    date: "2025-11-12",
    date_confidence: 0.96,
    description: "Smith sends certified habitability repair request to ABC.",
    category: "communication",
    source_document_id: "doc:repair-request",
    party_ids: ["party:smith", "party:abc"],
    linked_fact_ids: ["fact:repair-requested"],
    linked_claim_ids: ["claim:habitability", "claim:retaliation"],
    disputed: false,
  },
  {
    event_id: "event:medical-1",
    matter_id: "matter:smith-abc",
    date: "2025-12-08",
    date_confidence: 0.95,
    description: "OHSU visit for respiratory symptoms.",
    category: "incident",
    source_document_id: "doc:medical",
    party_ids: ["party:smith"],
    linked_fact_ids: ["fact:medical-impact"],
    linked_claim_ids: ["claim:habitability"],
    disputed: false,
  },
  {
    event_id: "event:zelle",
    matter_id: "matter:smith-abc",
    date: "2026-02-28",
    date_confidence: 0.99,
    description: "Smith pays Feb rent ($1,420) via Zelle.",
    category: "payment",
    source_document_id: "doc:zelle-confirm",
    party_ids: ["party:smith", "party:abc"],
    linked_fact_ids: ["fact:rent-paid-feb"],
    linked_claim_ids: ["claim:wrongful-eviction"],
    disputed: false,
  },
  {
    event_id: "event:email-current",
    matter_id: "matter:smith-abc",
    date: "2026-03-03",
    date_confidence: 0.97,
    description: "ABC emails Smith confirming \"all current as of today.\"",
    category: "communication",
    source_document_id: "doc:email-mar3",
    party_ids: ["party:abc", "party:smith"],
    linked_fact_ids: ["fact:abc-acknowledged"],
    linked_claim_ids: ["claim:wrongful-eviction"],
    disputed: false,
  },
  {
    event_id: "event:notice",
    matter_id: "matter:smith-abc",
    date: "2026-03-14",
    date_confidence: 0.78,
    description: "72-hour notice of nonpayment posted on door (no certified mail).",
    category: "notice",
    source_document_id: "doc:72hr-notice",
    party_ids: ["party:abc"],
    linked_fact_ids: ["fact:72hr-posted", "fact:no-cert-mail", "fact:retaliation-timing"],
    linked_claim_ids: ["claim:wrongful-eviction", "claim:retaliation"],
    disputed: true,
  },
  {
    event_id: "event:lockout",
    matter_id: "matter:smith-abc",
    date: "2026-04-18",
    date_confidence: 0.99,
    description: "ABC changes locks at Unit 4B without court order; PPB responds.",
    category: "incident",
    source_document_id: "doc:police-report",
    party_ids: ["party:abc", "party:smith"],
    linked_fact_ids: ["fact:lockout"],
    linked_claim_ids: ["claim:wrongful-eviction"],
    disputed: false,
  },
  {
    event_id: "event:fed-filed",
    matter_id: "matter:smith-abc",
    date: "2026-04-21",
    date_confidence: 0.99,
    description: "Northpoint files FED complaint in Multnomah County Circuit Court.",
    category: "filing",
    source_document_id: "doc:complaint",
    party_ids: ["party:northpoint"],
    linked_fact_ids: [],
    linked_claim_ids: [],
    disputed: false,
  },
  {
    event_id: "event:appearance",
    matter_id: "matter:smith-abc",
    date: "2026-05-12",
    date_confidence: 1.0,
    description: "Scheduled first appearance — answer due.",
    category: "court_event",
    source_document_id: "doc:summons",
    party_ids: ["party:smith", "party:northpoint", "party:hon-lee"],
    linked_fact_ids: [],
    linked_claim_ids: [],
    disputed: false,
  },
]

// ===== Evidence =====

export const evidenceSmithAbc: CaseEvidence[] = [
  {
    evidence_id: "evidence:lease-sec1",
    matter_id: "matter:smith-abc",
    document_id: "doc:lease",
    source_span: "page 1, ¶ 1.2",
    quote: "This lease begins August 1, 2024 and continues for twelve (12) months.",
    evidence_type: "document_quote",
    strength: "strong",
    confidence: 0.99,
    exhibit_label: "Ex. A",
    supports_fact_ids: ["fact:lease-start"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:lease-sec14",
    matter_id: "matter:smith-abc",
    document_id: "doc:lease",
    source_span: "page 7, § 14",
    quote:
      "Any notice of termination must be delivered by U.S. certified mail with return receipt requested.",
    evidence_type: "document_quote",
    strength: "strong",
    confidence: 0.97,
    exhibit_label: "Ex. A",
    supports_fact_ids: ["fact:no-cert-mail"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:zelle",
    matter_id: "matter:smith-abc",
    document_id: "doc:zelle-confirm",
    source_span: "image, full screen",
    quote: "Sent $1,420.00 to ABC PROP MGMT on Feb 28, 2026 — Confirmation #ZL482911",
    evidence_type: "image",
    strength: "strong",
    confidence: 0.98,
    exhibit_label: "Ex. D",
    supports_fact_ids: ["fact:rent-paid-feb"],
    contradicts_fact_ids: ["fact:72hr-posted"],
  },
  {
    evidence_id: "evidence:ledger-feb",
    matter_id: "matter:smith-abc",
    document_id: "doc:rent-ledger",
    source_span: "row 19",
    quote: "2026-02-28 | $1,420.00 | Zelle | PAID",
    evidence_type: "record",
    strength: "moderate",
    confidence: 0.92,
    exhibit_label: "Ex. C",
    supports_fact_ids: ["fact:rent-paid-feb"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:email-mar3",
    matter_id: "matter:smith-abc",
    document_id: "doc:email-mar3",
    source_span: "page 1, body ¶ 2",
    quote: "Thanks Sarah — confirming receipt. Your account is all current as of today.",
    evidence_type: "email",
    strength: "strong",
    confidence: 0.96,
    exhibit_label: "Ex. E",
    supports_fact_ids: ["fact:abc-acknowledged"],
    contradicts_fact_ids: ["fact:72hr-posted"],
  },
  {
    evidence_id: "evidence:notice-stamp",
    matter_id: "matter:smith-abc",
    document_id: "doc:72hr-notice",
    source_span: "page 1, header",
    quote: "72-HOUR NOTICE OF NONPAYMENT — Date of service: 03/14/2026 — Method: posted on door",
    evidence_type: "document_quote",
    strength: "moderate",
    confidence: 0.84,
    exhibit_label: "Ex. B",
    supports_fact_ids: ["fact:72hr-posted", "fact:no-cert-mail"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:repair-letter",
    matter_id: "matter:smith-abc",
    document_id: "doc:repair-request",
    source_span: "page 1, ¶ 1",
    quote:
      "Pursuant to ORS 90.320, I formally request repair of the bathroom mold and broken heater within the statutory cure period.",
    evidence_type: "document_quote",
    strength: "strong",
    confidence: 0.97,
    exhibit_label: "Ex. G",
    supports_fact_ids: ["fact:repair-requested", "fact:no-repair", "fact:retaliation-timing"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:photo-mold-1",
    matter_id: "matter:smith-abc",
    document_id: "doc:mold-photos",
    source_span: "IMG_4219.jpg",
    quote: "Photo of black mold along bathroom ceiling tile, EXIF date 2025-11-04.",
    evidence_type: "image",
    strength: "strong",
    confidence: 0.93,
    exhibit_label: "Ex. F-1",
    supports_fact_ids: ["fact:mold-existed"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:photo-mold-feb",
    matter_id: "matter:smith-abc",
    document_id: "doc:mold-photos",
    source_span: "IMG_4711.jpg",
    quote: "Photo of expanded mold area on drywall, EXIF date 2026-02-09.",
    evidence_type: "image",
    strength: "strong",
    confidence: 0.92,
    exhibit_label: "Ex. F-7",
    supports_fact_ids: ["fact:mold-existed", "fact:no-repair"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:medical-feb",
    matter_id: "matter:smith-abc",
    document_id: "doc:medical",
    source_span: "Encounter 2026-02-04, Assessment",
    quote:
      "Reactive airway symptoms consistent with environmental mold exposure; recommend remediation of residence.",
    evidence_type: "record",
    strength: "moderate",
    confidence: 0.88,
    supports_fact_ids: ["fact:medical-impact", "fact:mold-existed"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:police-report",
    matter_id: "matter:smith-abc",
    document_id: "doc:police-report",
    source_span: "page 2, narrative",
    quote:
      "RP stated tenant was locked out; ABC representative on scene admitted he changed the locks earlier that day. No FED writ presented.",
    evidence_type: "record",
    strength: "strong",
    confidence: 0.97,
    exhibit_label: "Ex. H",
    supports_fact_ids: ["fact:lockout"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:rlid",
    matter_id: "matter:smith-abc",
    document_id: "doc:rlid",
    source_span: "Owner of Record",
    quote: "Owner of Record: NORTHPOINT HOLDINGS LLC",
    evidence_type: "record",
    strength: "strong",
    confidence: 0.99,
    supports_fact_ids: ["fact:abc-not-owner"],
    contradicts_fact_ids: [],
  },
  {
    evidence_id: "evidence:texts-apr12",
    matter_id: "matter:smith-abc",
    document_id: "doc:texts",
    source_span: "2026-04-12 17:42",
    quote: "You need to be out by Friday or we change the locks.",
    evidence_type: "testimony",
    strength: "moderate",
    confidence: 0.86,
    supports_fact_ids: ["fact:lockout"],
    contradicts_fact_ids: [],
  },
]

// ===== Claims =====

export const claimsSmithAbc = [
  {
    claim_id: "claim:wrongful-eviction",
    matter_id: "matter:smith-abc",
    count_label: "Counterclaim I",
    name: "Wrongful Eviction / Self-Help Lockout",
    claim_type: "statutory",
    legal_theory: "ORS 90.375 prohibits self-help eviction; tenant entitled to actual damages plus statutory penalty.",
    status: "candidate",
    risk_level: "low",
    elements: [
      {
        element_id: "el:wrongful-1",
        text: "Tenant has a possessory interest in the dwelling unit.",
        authority: "ORS 90.100",
        satisfied: true,
        fact_ids: ["fact:lease-start"],
        evidence_ids: ["evidence:lease-sec1"],
        missing_facts: [],
      },
      {
        element_id: "el:wrongful-2",
        text: "Landlord recovered possession by means other than a judgment for restitution.",
        authority: "ORS 90.375(1)",
        satisfied: true,
        fact_ids: ["fact:lockout"],
        evidence_ids: ["evidence:police-report"],
        missing_facts: [],
      },
      {
        element_id: "el:wrongful-3",
        text: "Tenant suffered actual damages or had access denied.",
        authority: "ORS 90.375(2)",
        satisfied: true,
        fact_ids: ["fact:lockout"],
        evidence_ids: ["evidence:police-report", "evidence:texts-apr12"],
        missing_facts: ["Itemized hotel/storage receipts to quantify damages"],
      },
    ],
    fact_ids: ["fact:lockout", "fact:rent-paid-feb", "fact:abc-acknowledged"],
    evidence_ids: ["evidence:police-report", "evidence:zelle", "evidence:email-mar3"],
    defense_ids: [],
    authorities: [
      { citation: "ORS 90.375", canonical_id: "or:ors:90.375", reason: "Self-help eviction prohibition + remedies." },
      { citation: "ORS 105.115", canonical_id: "or:ors:105.115", reason: "Authorized methods for FED restitution." },
    ],
    remedies: ["Actual damages", "Statutory penalty (2x monthly rent)", "Reasonable attorney fees"],
  },
  {
    claim_id: "claim:habitability",
    matter_id: "matter:smith-abc",
    count_label: "Counterclaim II",
    name: "Breach of Habitability",
    claim_type: "statutory",
    legal_theory: "ORS 90.320 imposes a duty to maintain habitability; failure permits damages and rent reduction.",
    status: "candidate",
    risk_level: "medium",
    elements: [
      {
        element_id: "el:hab-1",
        text: "Landlord failed to maintain habitability.",
        authority: "ORS 90.320(1)",
        satisfied: true,
        fact_ids: ["fact:mold-existed", "fact:no-repair"],
        evidence_ids: ["evidence:photo-mold-1", "evidence:photo-mold-feb"],
        missing_facts: [],
      },
      {
        element_id: "el:hab-2",
        text: "Tenant gave written notice and a reasonable opportunity to cure.",
        authority: "ORS 90.320(2)",
        satisfied: true,
        fact_ids: ["fact:repair-requested"],
        evidence_ids: ["evidence:repair-letter"],
        missing_facts: [],
      },
      {
        element_id: "el:hab-3",
        text: "Tenant suffered diminished value or actual damages.",
        authority: "ORS 90.360",
        satisfied: false,
        fact_ids: ["fact:medical-impact"],
        evidence_ids: ["evidence:medical-feb"],
        missing_facts: ["Diminution-in-value calculation", "Habitability expert declaration"],
      },
    ],
    fact_ids: ["fact:mold-existed", "fact:repair-requested", "fact:no-repair", "fact:medical-impact"],
    evidence_ids: ["evidence:photo-mold-1", "evidence:photo-mold-feb", "evidence:repair-letter", "evidence:medical-feb"],
    defense_ids: [],
    authorities: [
      { citation: "ORS 90.320", canonical_id: "or:ors:90.320", reason: "Landlord habitability duty." },
      { citation: "ORS 90.360", canonical_id: "or:ors:90.360", reason: "Tenant remedies for noncompliance." },
    ],
    remedies: ["Rent abatement", "Damages", "Injunction"],
  },
  {
    claim_id: "claim:retaliation",
    matter_id: "matter:smith-abc",
    count_label: "Counterclaim III",
    name: "Retaliatory Eviction",
    claim_type: "statutory",
    legal_theory:
      "ORS 90.385 creates a presumption of retaliation when landlord acts within 6 months of tenant's good-faith complaint.",
    status: "candidate",
    risk_level: "medium",
    elements: [
      {
        element_id: "el:ret-1",
        text: "Tenant engaged in protected activity.",
        authority: "ORS 90.385(2)",
        satisfied: true,
        fact_ids: ["fact:repair-requested"],
        evidence_ids: ["evidence:repair-letter"],
        missing_facts: [],
      },
      {
        element_id: "el:ret-2",
        text: "Landlord took adverse action within 6 months.",
        authority: "ORS 90.385(1)",
        satisfied: true,
        fact_ids: ["fact:retaliation-timing", "fact:lockout"],
        evidence_ids: ["evidence:notice-stamp", "evidence:police-report"],
        missing_facts: [],
      },
      {
        element_id: "el:ret-3",
        text: "Adverse action was retaliatory in nature.",
        authority: "ORS 90.385(1)",
        satisfied: false,
        fact_ids: [],
        evidence_ids: [],
        missing_facts: ["Direct evidence of retaliatory motive (statements, emails)"],
      },
    ],
    fact_ids: ["fact:repair-requested", "fact:retaliation-timing", "fact:lockout"],
    evidence_ids: ["evidence:repair-letter", "evidence:notice-stamp"],
    defense_ids: [],
    authorities: [{ citation: "ORS 90.385", canonical_id: "or:ors:90.385", reason: "Retaliation framework." }],
    remedies: ["Defense to FED", "Up to 2 months rent statutory damages", "Attorney fees"],
  },
]

// ===== Defenses =====

export const defensesSmithAbc: CaseDefense[] = [
  {
    defense_id: "defense:improper-notice",
    matter_id: "matter:smith-abc",
    name: "Improper Notice (Failure of Service)",
    basis: "Notice was not delivered by certified mail per lease § 14 and ORS 90.155 service rules.",
    status: "candidate",
    applies_to_claim_ids: ["claim:wrongful-eviction"],
    required_facts: ["Notice posted only on door", "Lease requires certified mail"],
    fact_ids: ["fact:no-cert-mail"],
    evidence_ids: ["evidence:notice-stamp", "evidence:lease-sec14"],
    authorities: [{ citation: "ORS 90.155", canonical_id: "or:ors:90.155", reason: "Manner of service of written notices." }],
    viability: "high",
  },
  {
    defense_id: "defense:no-default",
    matter_id: "matter:smith-abc",
    name: "Account Was Current — No Default",
    basis: "Landlord acknowledged in writing that the account was current six days before issuing the 72-hour notice.",
    status: "candidate",
    applies_to_claim_ids: ["claim:wrongful-eviction"],
    required_facts: ["February rent paid before due date", "ABC's March 3 acknowledgment"],
    fact_ids: ["fact:rent-paid-feb", "fact:abc-acknowledged"],
    evidence_ids: ["evidence:zelle", "evidence:email-mar3"],
    authorities: [{ citation: "ORS 90.394", canonical_id: "or:ors:90.394", reason: "Termination for nonpayment requires actual nonpayment." }],
    viability: "high",
  },
  {
    defense_id: "defense:improper-party",
    matter_id: "matter:smith-abc",
    name: "Improper Plaintiff (Real Party in Interest)",
    basis: "ABC Property Management is not the owner of record and lacks standing to sue for restitution.",
    status: "candidate",
    applies_to_claim_ids: ["claim:wrongful-eviction"],
    required_facts: ["Northpoint is owner of record per RLID"],
    fact_ids: ["fact:abc-not-owner"],
    evidence_ids: ["evidence:rlid"],
    authorities: [{ citation: "ORCP 26", canonical_id: "or:orcp:26", reason: "Real party in interest." }],
    viability: "medium",
  },
  {
    defense_id: "defense:retaliation",
    matter_id: "matter:smith-abc",
    name: "Retaliation (Affirmative Defense)",
    basis: "ORS 90.385 presumption of retaliation given habitability complaint within prior 6 months.",
    status: "candidate",
    applies_to_claim_ids: ["claim:wrongful-eviction"],
    required_facts: ["Habitability complaint", "Adverse action within 6 months"],
    fact_ids: ["fact:repair-requested", "fact:retaliation-timing"],
    evidence_ids: ["evidence:repair-letter", "evidence:notice-stamp"],
    authorities: [{ citation: "ORS 90.385", canonical_id: "or:ors:90.385", reason: "Retaliation as defense to FED." }],
    viability: "medium",
  },
]

// ===== Deadlines =====

export const deadlinesSmithAbc = [
  {
    deadline_id: "dl:answer",
    matter_id: "matter:smith-abc",
    description: "First appearance / answer due in FED case 25CV12384",
    due_date: "2026-05-12",
    days_remaining: 14,
    severity: "critical",
    source_citation: "ORS 105.137",
    source_canonical_id: "or:ors:105.137",
    triggered_by_event_id: "event:fed-filed",
    status: "open",
  },
  {
    deadline_id: "dl:counterclaim",
    matter_id: "matter:smith-abc",
    description: "Deadline to file counterclaim with answer",
    due_date: "2026-05-12",
    days_remaining: 14,
    severity: "critical",
    source_citation: "ORCP 22 A",
    triggered_by_event_id: "event:fed-filed",
    status: "open",
  },
  {
    deadline_id: "dl:discovery",
    matter_id: "matter:smith-abc",
    description: "Initial discovery requests window opens",
    due_date: "2026-05-26",
    days_remaining: 28,
    severity: "warning",
    source_citation: "ORCP 36",
    status: "open",
  },
  {
    deadline_id: "dl:tort-claim",
    matter_id: "matter:smith-abc",
    description: "Oregon Tort Claims Act notice (if claims against public entity)",
    due_date: "2026-09-18",
    days_remaining: 143,
    severity: "info",
    source_citation: "ORS 30.275",
    source_canonical_id: "or:ors:30.275",
    status: "open",
    notes: "Likely not applicable; verify before closing matter.",
  },
]

// ===== Tasks =====

export const tasksSmithAbc: CaseTask[] = [
  {
    task_id: "task:1",
    matter_id: "matter:smith-abc",
    title: "Finalize draft answer with affirmative defenses",
    status: "in_progress",
    priority: "high",
    due_date: "2026-05-09",
    assigned_to: "Sarah Smith",
    related_claim_ids: ["claim:wrongful-eviction"],
    related_document_ids: ["doc:draft-answer"],
    related_deadline_id: "dl:answer",
    source: "user",
  },
  {
    task_id: "task:2",
    matter_id: "matter:smith-abc",
    title: "Collect hotel and storage receipts post-lockout",
    status: "todo",
    priority: "high",
    due_date: "2026-05-08",
    assigned_to: "Sarah Smith",
    related_claim_ids: ["claim:wrongful-eviction"],
    related_document_ids: [],
    source: "ai_suggestion",
    description: "Needed to quantify actual damages under ORS 90.375.",
  },
  {
    task_id: "task:3",
    matter_id: "matter:smith-abc",
    title: "Obtain habitability expert declaration (mold remediation cost)",
    status: "todo",
    priority: "med",
    due_date: "2026-05-30",
    assigned_to: null,
    related_claim_ids: ["claim:habitability"],
    related_document_ids: ["doc:mold-photos", "doc:medical"],
    source: "ai_suggestion",
  },
  {
    task_id: "task:4",
    matter_id: "matter:smith-abc",
    title: "File counterclaim w/ answer",
    status: "todo",
    priority: "high",
    due_date: "2026-05-12",
    assigned_to: "Sarah Smith",
    related_claim_ids: ["claim:wrongful-eviction", "claim:habitability", "claim:retaliation"],
    related_document_ids: [],
    related_deadline_id: "dl:counterclaim",
    source: "deadline",
  },
  {
    task_id: "task:5",
    matter_id: "matter:smith-abc",
    title: "Subpoena ABC owner-tenant records for unit 4B",
    status: "blocked",
    priority: "med",
    due_date: null,
    assigned_to: null,
    related_claim_ids: [],
    related_document_ids: [],
    source: "user",
    description: "Blocked pending appearance and ORCP 36 window.",
  },
  {
    task_id: "task:6",
    matter_id: "matter:smith-abc",
    title: "Verify owner of record (RLID)",
    status: "done",
    priority: "low",
    due_date: "2026-04-23",
    assigned_to: "Sarah Smith",
    related_claim_ids: [],
    related_document_ids: ["doc:rlid"],
    source: "user",
  },
]

// ===== Drafts =====

export const draftAnswerSmithAbc = {
  draft_id: "draft:answer-v3",
  matter_id: "matter:smith-abc",
  title: "Draft Answer + Counterclaims (v3)",
  draft_type: "answer",
  status: "draft",
  created_at: "2026-04-24T13:11:00Z",
  updated_at: "2026-04-26T17:42:00Z",
  word_count: 982,
  factcheck_summary: {
    supported: 7,
    needs_evidence: 2,
    needs_authority: 1,
    contradicted: 0,
    citation_issue: 1,
    deadline_warning: 1,
    unchecked: 2,
  },
  paragraphs: [
    {
      paragraph_id: "para:caption",
      index: 0,
      role: "caption",
      heading_level: 1,
      text: "IN THE CIRCUIT COURT OF THE STATE OF OREGON FOR THE COUNTY OF MULTNOMAH",
      fact_ids: [],
      evidence_ids: [],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:title",
      index: 1,
      role: "heading",
      heading_level: 2,
      text: "DEFENDANT'S ANSWER, AFFIRMATIVE DEFENSES, AND COUNTERCLAIMS",
      fact_ids: [],
      evidence_ids: [],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:1",
      index: 2,
      role: "facts",
      text:
        "Defendant Sarah Smith is and at all relevant times has been a tenant of Unit 4B, 1418 NE Alberta Street, Portland, Oregon, pursuant to a written 12-month residential lease commencing August 1, 2024.",
      fact_ids: ["fact:lease-start"],
      evidence_ids: ["evidence:lease-sec1"],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:2",
      index: 3,
      role: "facts",
      text:
        "On February 28, 2026, Defendant timely paid the February 2026 rent in the amount of $1,420.00 to ABC Property Management LLC by Zelle transfer (Confirmation #ZL482911).",
      fact_ids: ["fact:rent-paid-feb"],
      evidence_ids: ["evidence:zelle", "evidence:ledger-feb"],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:3",
      index: 4,
      role: "facts",
      text:
        "On March 3, 2026, ABC Property Management LLC, through D. Halverson, sent Defendant a written communication confirming that her account was \"all current as of today.\"",
      fact_ids: ["fact:abc-acknowledged"],
      evidence_ids: ["evidence:email-mar3"],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:4",
      index: 5,
      role: "facts",
      text:
        "Notwithstanding the foregoing, on or about March 14, 2026, Plaintiff caused a 72-hour notice of nonpayment to be posted on Defendant's door without service by certified mail.",
      fact_ids: ["fact:72hr-posted", "fact:no-cert-mail"],
      evidence_ids: ["evidence:notice-stamp"],
      authorities: [
        { citation: "ORS 90.155", canonical_id: "or:ors:90.155", pinpoint: "(1)(c)" },
      ],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:5",
      index: 6,
      role: "facts",
      text:
        "On April 18, 2026, agents of Plaintiff entered Unit 4B and changed the locks without a judgment of restitution, in violation of ORS 90.375.",
      fact_ids: ["fact:lockout"],
      evidence_ids: ["evidence:police-report"],
      authorities: [{ citation: "ORS 90.375", canonical_id: "or:ors:90.375" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:6",
      index: 7,
      role: "law",
      heading_level: 3,
      text: "FIRST AFFIRMATIVE DEFENSE — Improper Service of Notice",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 90.155", canonical_id: "or:ors:90.155" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:7",
      index: 8,
      role: "analysis",
      text:
        "The 72-hour notice on which Plaintiff's claim depends was not served in the manner required by ORS 90.155 or by Section 14 of the parties' lease, and is therefore void ab initio.",
      fact_ids: ["fact:no-cert-mail"],
      evidence_ids: ["evidence:notice-stamp", "evidence:lease-sec14"],
      authorities: [{ citation: "ORS 90.155", canonical_id: "or:ors:90.155" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:8",
      index: 9,
      role: "law",
      heading_level: 3,
      text: "SECOND AFFIRMATIVE DEFENSE — No Default",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 90.394", canonical_id: "or:ors:90.394" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:9",
      index: 10,
      role: "analysis",
      text:
        "Plaintiff's representative confirmed in writing on March 3, 2026 that Defendant's rent account was current. No nonpayment occurred and no event of default existed at the time of the notice.",
      fact_ids: ["fact:rent-paid-feb", "fact:abc-acknowledged"],
      evidence_ids: ["evidence:zelle", "evidence:email-mar3"],
      authorities: [{ citation: "ORS 90.394", canonical_id: "or:ors:90.394" }],
      factcheck_status: "needs_authority",
      factcheck_note: "Add citation to controlling Oregon Court of Appeals decision on \"current account\" defense.",
    },
    {
      paragraph_id: "para:10",
      index: 11,
      role: "law",
      heading_level: 3,
      text: "THIRD AFFIRMATIVE DEFENSE — Retaliation",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 90.385", canonical_id: "or:ors:90.385" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:11",
      index: 12,
      role: "analysis",
      text:
        "Defendant served Plaintiff with a written habitability complaint pursuant to ORS 90.320 on November 12, 2025. Plaintiff's adverse actions occurred within six months thereafter, triggering the statutory presumption of retaliation under ORS 90.385.",
      fact_ids: ["fact:repair-requested", "fact:retaliation-timing"],
      evidence_ids: ["evidence:repair-letter"],
      authorities: [
        { citation: "ORS 90.320", canonical_id: "or:ors:90.320" },
        { citation: "ORS 90.385", canonical_id: "or:ors:90.385" },
      ],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:12",
      index: 13,
      role: "law",
      heading_level: 3,
      text: "COUNTERCLAIM I — Wrongful Eviction (ORS 90.375)",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 90.375", canonical_id: "or:ors:90.375" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:13",
      index: 14,
      role: "analysis",
      text:
        "Plaintiff recovered possession of the dwelling unit on April 18, 2026 by self-help means, namely changing the locks without a judgment of restitution. Defendant is entitled to actual damages plus an amount equal to two months' periodic rent.",
      fact_ids: ["fact:lockout"],
      evidence_ids: ["evidence:police-report", "evidence:texts-apr12"],
      authorities: [{ citation: "ORS 90.375", canonical_id: "or:ors:90.375" }],
      factcheck_status: "needs_evidence",
      factcheck_note: "Itemized hotel and storage receipts not yet linked to draft.",
    },
    {
      paragraph_id: "para:14",
      index: 15,
      role: "law",
      heading_level: 3,
      text: "COUNTERCLAIM II — Breach of Habitability (ORS 90.320)",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 90.320", canonical_id: "or:ors:90.320" }],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:15",
      index: 16,
      role: "analysis",
      text:
        "Plaintiff failed to maintain habitable premises following written notice and a reasonable opportunity to cure. Defendant suffered diminished value of the leasehold and out-of-pocket medical costs related to mold exposure.",
      fact_ids: ["fact:mold-existed", "fact:no-repair", "fact:medical-impact"],
      evidence_ids: ["evidence:photo-mold-1", "evidence:photo-mold-feb", "evidence:repair-letter", "evidence:medical-feb"],
      authorities: [
        { citation: "ORS 90.320", canonical_id: "or:ors:90.320" },
        { citation: "ORS 90.360", canonical_id: "or:ors:90.360" },
      ],
      factcheck_status: "needs_evidence",
      factcheck_note: "Diminution-in-value calculation missing.",
    },
    {
      paragraph_id: "para:16",
      index: 17,
      role: "relief",
      heading_level: 3,
      text: "PRAYER FOR RELIEF",
      fact_ids: [],
      evidence_ids: [],
      authorities: [],
      factcheck_status: "supported",
    },
    {
      paragraph_id: "para:17",
      index: 18,
      role: "relief",
      text:
        "Wherefore, Defendant respectfully requests that this Court (a) dismiss Plaintiff's complaint with prejudice; (b) award damages and statutory penalties on the counterclaims; (c) award reasonable attorney fees and costs; and (d) grant such further relief as the Court deems just.",
      fact_ids: [],
      evidence_ids: [],
      authorities: [{ citation: "ORS 105.137", canonical_id: "or:ors:105.137" }],
      factcheck_status: "deadline_warning",
      factcheck_note: "Verify deadline for amended counterclaim — see ORCP 22 A.",
    },
    {
      paragraph_id: "para:18",
      index: 19,
      role: "signature",
      text: "DATED: ____________ — Sarah Smith, Defendant pro se",
      fact_ids: [],
      evidence_ids: [],
      authorities: [],
      factcheck_status: "unchecked",
    },
  ],
}

// ===== Document extractions (per-document AI inspector content) =====

export const documentExtractionsSmithAbc: Record<string, DocumentExtraction> = {
  "doc:complaint": {
    document_id: "doc:complaint",
    summary:
      "FED complaint by Northpoint Holdings via ABC alleging $2,840 in unpaid rent. Critical: served via posting only; lease requires certified mail.",
    key_dates: [
      { date: "2026-04-21", description: "Complaint filed", page: 1 },
      { date: "2026-03-14", description: "Date of alleged 72-hour notice", page: 2 },
      { date: "2026-05-12", description: "First appearance", page: 1 },
    ],
    parties: [
      { name: "Northpoint Holdings LLC", role: "Plaintiff" },
      { name: "Sarah Smith", role: "Defendant" },
      { name: "ABC Property Management LLC", role: "Plaintiff's agent" },
    ],
    entities: [
      { name: "Multnomah County Circuit Court", type: "court" },
      { name: "Halverson & Pratt, LLP", type: "law firm" },
      { name: "1418 NE Alberta St, Unit 4B", type: "address" },
    ],
    possible_facts: [
      { text: "ABC posted a 72-hour notice on March 14, 2026.", confidence: 0.84, suggested_status: "disputed" },
      { text: "Northpoint claims $2,840 in unpaid rent.", confidence: 0.92, suggested_status: "disputed" },
    ],
    possible_claims: [
      { name: "Forcible entry and detainer", rationale: "Statutory FED claim under ORS Ch. 105.", viability: "low" },
    ],
    possible_defenses: [
      { name: "Improper notice", rationale: "No certified mail service.", viability: "high" },
      { name: "No default", rationale: "March 3 email confirms account current.", viability: "high" },
      { name: "Real party in interest", rationale: "ABC is not owner of record per RLID.", viability: "medium" },
    ],
    citations: [
      { raw: "ORS 90.394", resolved_canonical_id: "or:ors:90.394", status: "active" },
      { raw: "ORS 105.115", resolved_canonical_id: "or:ors:105.115", status: "active" },
      { raw: "ORS 90.155", resolved_canonical_id: "or:ors:90.155", status: "active" },
      { raw: "ORCP 7", status: "unresolved" },
    ],
    contradictions: [
      {
        text: "Complaint asserts default as of March 14, but ABC's March 3 email confirmed account current.",
        against_quote: "Your account is all current as of today.",
      },
      {
        text: "Complaint names ABC as plaintiff although owner of record is Northpoint.",
      },
    ],
    exhibit_notes: [
      "Mark complaint as Doc 1 / non-exhibit.",
      "Cross-reference with March 3 email (Ex. E) to highlight contradiction.",
    ],
  },
  "doc:lease": {
    document_id: "doc:lease",
    summary:
      "12-month residential lease beginning Aug 1, 2024 at $1,420/mo. Section 14 requires certified mail for any termination notice.",
    key_dates: [
      { date: "2024-08-01", description: "Lease commencement", page: 1 },
      { date: "2025-07-31", description: "Lease end (renewal silent)", page: 1 },
    ],
    parties: [
      { name: "Sarah Smith", role: "Tenant" },
      { name: "ABC Property Management LLC", role: "Manager" },
    ],
    entities: [{ name: "1418 NE Alberta St, Unit 4B", type: "address" }],
    possible_facts: [
      { text: "Monthly rent is $1,420.", confidence: 0.99, suggested_status: "supported" },
      { text: "Termination notice must be sent by certified mail.", confidence: 0.97, suggested_status: "supported" },
    ],
    possible_claims: [],
    possible_defenses: [
      { name: "Improper notice", rationale: "Lease § 14 requires certified mail.", viability: "high" },
    ],
    citations: [],
    contradictions: [],
    exhibit_notes: ["Mark as Ex. A; pinpoint § 14 for service argument."],
  },
}

// ===== Ask Matter (mock chat seed) =====

export const askMatterSmithAbc = [
  {
    message_id: "m1",
    role: "user",
    text: "What are the strongest defenses to the FED complaint?",
    timestamp: "2026-04-25T11:14:00Z",
  },
  {
    message_id: "m2",
    role: "assistant",
    text:
      "Three defenses look strong, in order of viability:\n\n1) Improper notice. The 72-hour notice was posted only; lease § 14 and ORS 90.155 require certified mail. (See Ex. B header and Ex. A § 14.)\n\n2) No default. ABC's March 3 email confirmed your account was \"all current as of today,\" only six days before the notice was served. The Zelle confirmation and rent ledger both show February rent was paid in full.\n\n3) Retaliation. ORS 90.385 creates a presumption when adverse action follows a good-faith habitability complaint within 6 months. Your November 12 repair letter is squarely inside that window.\n\nI can draft these as affirmative defenses now and link each paragraph to the supporting evidence. There's also a real-party-in-interest issue (ABC vs. Northpoint) worth raising as a motion to dismiss.",
    timestamp: "2026-04-25T11:14:11Z",
    context_used: {
      document_ids: ["doc:complaint", "doc:lease", "doc:72hr-notice", "doc:email-mar3", "doc:zelle-confirm", "doc:repair-request", "doc:rlid"],
      fact_ids: ["fact:no-cert-mail", "fact:rent-paid-feb", "fact:abc-acknowledged", "fact:retaliation-timing", "fact:abc-not-owner"],
      evidence_ids: ["evidence:notice-stamp", "evidence:lease-sec14", "evidence:zelle", "evidence:email-mar3", "evidence:repair-letter", "evidence:rlid"],
      authorities: [
        { citation: "ORS 90.155", canonical_id: "or:ors:90.155" },
        { citation: "ORS 90.385", canonical_id: "or:ors:90.385" },
        { citation: "ORS 90.394", canonical_id: "or:ors:90.394" },
        { citation: "ORS 90.375", canonical_id: "or:ors:90.375" },
      ],
    },
    caveats: [
      "Legal information, not legal advice.",
      "Real-party-in-interest defense depends on local procedural treatment of FED actions.",
    ],
  },
]

// ===== Helpers =====

const SMITH_MATTER_ID = "matter:smith-abc"

type LegacyDocument = Omit<
  CaseDocument,
  "id" | "title" | "kind" | "pageCount" | "fileSize" | "dateUploaded" | "status" | "entities" | "chunks" | "clauses" | "linkedFacts" | "issues"
> & {
  document_id: string
  filename: string
  document_type: CaseDocument["kind"]
  uploaded_at: string
  processing_status: CaseDocument["status"]
}
type LegacyFact = Omit<CaseFact, "id" | "statement" | "disputed" | "tags" | "sourceDocumentIds" | "citations"> & {
  fact_id: string
  text: string
  fact_type: string
  status: NonNullable<CaseFact["status"]>
}
type LegacyEvent = Omit<CaseEvent, "id" | "title" | "kind" | "category"> & {
  event_id: string
  description: string
  category: string
}
type LegacyClaimElement = {
  element_id: string
  text: string
  authority: string
  satisfied: boolean
  fact_ids: string[]
  evidence_ids: string[]
  missing_facts: string[]
}
type LegacyClaim = Omit<CaseClaim, "id" | "kind" | "title" | "cause" | "theory" | "against" | "risk" | "elements" | "supportingFactIds" | "counterArguments"> & {
  claim_id: string
  count_label: string
  name: string
  claim_type: string
  legal_theory: string
  risk_level: NonNullable<CaseClaim["risk"]>
  elements: LegacyClaimElement[]
  fact_ids: string[]
}
type LegacyDeadline = Omit<
  CaseDeadline,
  "id" | "title" | "category" | "kind" | "dueDate" | "daysRemaining" | "source" | "tasks"
> & {
  deadline_id: string
  description: string
  due_date: string
  days_remaining: number
}
type LegacyDraft = Omit<CaseDraft, "id" | "description" | "kind" | "lastEdited" | "wordCount" | "sections" | "citeCheckIssues" | "versions"> & {
  draft_id: string
  draft_type: CaseDraft["kind"]
  updated_at: string
  word_count: number
  paragraphs: NonNullable<CaseDraft["paragraphs"]>
}

function decodeSegment(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function normalizeId(id: string): string {
  const decoded = decodeSegment(id).trim()
  if (!decoded.includes(":")) {
    const matterId = `matter:${decoded}`
    if (matters.some((matter) => matter.matter_id === matterId)) return matterId
  }
  return decoded
}

function shortMatterName(name: string): string {
  return name.split(/\s+v\.\s+/i)[0] ?? name
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const kb = bytes / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB`
  return `${(kb / 1024).toFixed(1)} MB`
}

function titleFromFilename(filename: string): string {
  return filename
    .replace(/\.[a-z0-9]+$/i, "")
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
}

function unique(values: string[]): string[] {
  return [...new Set(values.filter(Boolean))]
}

function evidenceFor(id: string): CaseEvidence | undefined {
  return evidenceSmithAbc.find((e) => e.evidence_id === id)
}

function factStatusToElementStatus(status: boolean): "supported" | "missing" {
  return status ? "supported" : "missing"
}

function normalizeParty(party: (typeof partiesSmithAbc)[number]): MatterParty {
  return {
    id: party.party_id,
    party_id: party.party_id,
    matter_id: party.matter_id,
    name: party.name,
    role: party.role as MatterParty["role"],
    partyType: party.party_type as MatterParty["partyType"],
    party_type: party.party_type as MatterParty["party_type"],
    representedBy: party.represented_by,
    represented_by: party.represented_by,
    contactEmail: party.contact_email,
    contact_email: party.contact_email,
    notes: party.notes,
  }
}

function normalizeFact(fact: LegacyFact): CaseFact {
  const sourceEvidenceIds = fact.source_evidence_ids ?? []
  const citations = sourceEvidenceIds.flatMap((evidenceId) => {
    const evidence = evidenceFor(evidenceId)
    if (!evidence) return []
    return {
      documentId: evidence.document_id,
      quote: evidence.quote,
      snippet: evidence.quote,
    }
  })
  const sourceDocumentIds = unique(citations.map((citation) => citation.documentId))

  return {
    ...fact,
    id: fact.fact_id,
    statement: fact.text,
    disputed: fact.status === "disputed" || (fact.contradicted_by_evidence_ids?.length ?? 0) > 0,
    tags: unique([fact.fact_type, ...(fact.supports_claim_ids ?? []).map((claimId) => claimId.replace(/^claim:/, ""))]),
    sourceDocumentIds,
    citations,
  }
}

function normalizeDocument(document: LegacyDocument, facts: CaseFact[]): CaseDocument {
  const id = document.document_id
  const summary = document.summary || titleFromFilename(document.filename)
  const chunks = [
    {
      id: `chunk:${id}:summary`,
      heading: document.folder,
      page: 1,
      text: summary,
      tokens: Math.max(1, Math.ceil(summary.length / 4)),
    },
  ]
  const entities = [
    ...(document.parties_mentioned ?? []).map((value, index) => ({
      id: `ent:${id}:party:${index}`,
      type: "party" as const,
      value,
      confidence: 0.86,
      spans: [],
    })),
    ...(document.entities_mentioned ?? []).map((value, index) => ({
      id: `ent:${id}:entity:${index}`,
      type: /\d+\s+\w+/.test(value) ? ("address" as const) : ("org" as const),
      value,
      confidence: 0.8,
      spans: [],
    })),
  ]

  return {
    ...document,
    id,
    title: titleFromFilename(document.filename),
    kind: document.document_type,
    pageCount: document.pages,
    fileSize: formatBytes(document.bytes),
    dateUploaded: document.uploaded_at,
    dateFiled: document.date_observed ?? undefined,
    status: document.processing_status,
    entities,
    chunks,
    clauses: [],
    linkedFacts: facts.filter((fact) => fact.sourceDocumentIds.includes(id)),
    issues: document.contradictions_flagged
      ? [
          {
            id: `issue:${id}:contradictions`,
            type: "contradiction",
            severity: document.contradictions_flagged > 1 ? "high" : "med",
            status: "open",
            label: `${document.contradictions_flagged} contradiction${document.contradictions_flagged === 1 ? "" : "s"}`,
            title: "Contradictions flagged",
            detail: "The inspector found statements that conflict with other matter evidence.",
          },
        ]
      : [],
  }
}

function normalizeEvent(event: LegacyEvent): CaseEvent {
  const kindMap: Record<string, CaseEvent["kind"]> = {
    filing: "filing",
    communication: "communication",
    incident: "incident",
    payment: "payment",
    notice: "notice",
    court_event: "court",
  }
  return {
    ...event,
    id: event.event_id,
    title: event.description,
    kind: kindMap[event.category] ?? "other",
    category: event.category,
    status: "complete",
  }
}

function normalizeClaim(claim: LegacyClaim): CaseClaim {
  return {
    ...claim,
    id: claim.claim_id,
    kind: claim.count_label.toLowerCase().includes("counterclaim") ? "counterclaim" : "claim",
    title: claim.name,
    cause: claim.claim_type.replace(/_/g, " "),
    theory: claim.legal_theory,
    against: "Northpoint Holdings LLC / ABC Property Management LLC",
    risk: claim.risk_level,
    status: claim.status ?? "candidate",
    elements: claim.elements.map((element) => ({
      ...element,
      id: element.element_id,
      title: element.text.replace(/\.$/, ""),
      description: element.text,
      status: factStatusToElementStatus(element.satisfied),
      legalAuthority: element.authority,
      supportingFactIds: element.fact_ids,
    })),
    supportingFactIds: claim.fact_ids,
    counterArguments: [],
  }
}

function normalizeDefenseAsClaim(defense: CaseDefense): CaseClaim {
  return {
    id: defense.defense_id,
    defense_id: defense.defense_id,
    matter_id: defense.matter_id,
    kind: "defense",
    title: defense.name,
    cause: "Affirmative defense",
    theory: defense.basis,
    against: defense.applies_to_claim_ids.join(", ") || "Opposing party",
    risk: defense.viability,
    risk_level: defense.viability,
    status: defense.status === "candidate" ? "candidate" : "asserted",
    elements: defense.required_facts.map((requiredFact, index) => ({
      id: `${defense.defense_id}:el:${index + 1}`,
      title: requiredFact,
      description: requiredFact,
      status: defense.fact_ids.length > index ? "supported" : "missing",
      supportingFactIds: defense.fact_ids,
      fact_ids: defense.fact_ids,
      evidence_ids: defense.evidence_ids,
    })),
    fact_ids: defense.fact_ids,
    evidence_ids: defense.evidence_ids,
    authorities: defense.authorities,
    supportingFactIds: defense.fact_ids,
    counterArguments: [],
  }
}

function normalizeDeadline(deadline: LegacyDeadline): CaseDeadline {
  const linkedTasks = tasksSmithAbc
    .filter((task) => task.related_deadline_id === deadline.deadline_id)
    .map((task) => ({
      id: task.task_id,
      label: task.title,
      done: task.status === "done",
      assignee: task.assigned_to ?? undefined,
    }))

  return {
    ...deadline,
    id: deadline.deadline_id,
    title: deadline.description,
    category: deadline.source_citation?.startsWith("ORCP") ? "filing" : "trial",
    kind: deadline.source_citation?.startsWith("ORS") ? "statutory" : "rule",
    dueDate: deadline.due_date,
    daysRemaining: deadline.days_remaining,
    source: deadline.source_citation ?? "Matter calendar",
    sourceCitation: deadline.source_citation,
    sourceCanonicalId: deadline.source_canonical_id,
    computedFrom: deadline.triggered_by_event_id,
    tasks: linkedTasks,
  }
}

function normalizeDraft(draft: LegacyDraft): CaseDraft {
  const sections = draft.paragraphs.map((paragraph) => ({
    id: paragraph.paragraph_id,
    heading: paragraph.heading_level ? paragraph.text : paragraph.role.replace(/_/g, " "),
    tone: "formal" as const,
    body: paragraph.text,
    citations: paragraph.authorities.map((authority, index) => ({
      id: `cite:${paragraph.paragraph_id}:${index}`,
      sourceId: authority.canonical_id,
      sourceKind: "statute" as const,
      shortLabel: authority.citation,
      fullLabel: authority.pinpoint ? `${authority.citation} ${authority.pinpoint}` : authority.citation,
      verified: paragraph.factcheck_status !== "citation_issue",
    })),
    comments: paragraph.factcheck_note
      ? [
          {
            id: `comment:${paragraph.paragraph_id}`,
            author: "AI Inspector",
            body: paragraph.factcheck_note,
            timestamp: draft.updated_at,
          },
        ]
      : [],
    suggestions:
      paragraph.factcheck_status === "supported"
        ? []
        : [
            {
              id: `suggestion:${paragraph.paragraph_id}`,
              kind: paragraph.factcheck_status === "needs_authority" ? ("add_authority" as const) : ("factcheck" as const),
              original: paragraph.text,
              proposed: paragraph.text,
              rationale: paragraph.factcheck_note ?? "Review this paragraph before filing.",
              sources: [...paragraph.fact_ids, ...paragraph.evidence_ids],
              confidence: 0.76,
            },
          ],
  }))

  return {
    ...draft,
    id: draft.draft_id,
    description: "Answer, affirmative defenses, and counterclaims generated from the matter graph.",
    kind: draft.draft_type,
    lastEdited: draft.updated_at,
    wordCount: draft.word_count,
    sections,
    citeCheckIssues: draft.paragraphs.flatMap((paragraph) =>
      paragraph.factcheck_status === "citation_issue" || paragraph.factcheck_status === "needs_authority"
        ? [
            {
              id: `cite-issue:${paragraph.paragraph_id}`,
              citationId: paragraph.authorities[0]?.canonical_id ?? paragraph.paragraph_id,
              kind: paragraph.factcheck_status === "citation_issue" ? "unverified" : "missing_authority",
              severity: "med" as const,
              message: paragraph.factcheck_note ?? "Citation needs review.",
              title: "Citation review needed",
              detail: paragraph.text,
              sectionId: paragraph.paragraph_id,
            },
          ]
        : [],
    ),
    versions: [
      {
        id: `${draft.draft_id}:v3`,
        label: "v3",
        timestamp: draft.updated_at,
        author: "CaseBuilder",
        summary: "Current AI-assisted working draft.",
      },
    ],
  }
}

function citationsFromContext(message: MatterAskMessage): MatterChatCitation[] {
  const context = message.context_used
  if (!context) return []

  let index = 1
  const nextIndex = () => String(index++)
  const documentCitations = context.document_ids.map((documentId) => {
    const doc = getDocumentsByMatter(SMITH_MATTER_ID).find((document) => document.id === documentId)
    return {
      id: `chat-cite:${message.message_id}:doc:${documentId}`,
      indexLabel: nextIndex(),
      kind: "document" as const,
      refId: documentId,
      sourceId: documentId,
      sourceKind: "document" as const,
      shortLabel: doc?.title ?? documentId,
      fullLabel: doc?.filename ?? documentId,
      title: doc?.title ?? documentId,
      snippet: doc?.summary,
    }
  })
  const factCitations = context.fact_ids.map((factId) => {
    const fact = getFactsByMatter(SMITH_MATTER_ID).find((item) => item.id === factId)
    return {
      id: `chat-cite:${message.message_id}:fact:${factId}`,
      indexLabel: nextIndex(),
      kind: "fact" as const,
      refId: factId,
      sourceId: factId,
      sourceKind: "fact" as const,
      shortLabel: factId,
      fullLabel: fact?.statement ?? factId,
      title: fact?.statement ?? factId,
      snippet: fact?.statement,
    }
  })
  const authorityCitations = context.authorities.map((authority) => ({
    id: `chat-cite:${message.message_id}:auth:${authority.canonical_id}`,
    indexLabel: nextIndex(),
    kind: "statute" as const,
    refId: authority.canonical_id,
    sourceId: authority.canonical_id,
    sourceKind: "statute" as const,
    shortLabel: authority.citation,
    fullLabel: authority.citation,
    title: authority.citation,
  }))

  return [...documentCitations, ...factCitations, ...authorityCitations]
}

function normalizeChatMessage(message: MatterAskMessage): MatterChatMessage {
  return {
    id: message.message_id,
    role: message.role,
    content: message.text,
    timestamp: message.timestamp,
    citations: citationsFromContext(message),
    reasoning: message.caveats,
  }
}

function buildMatter(summary: MatterSummary): Matter {
  const parties = getPartiesByMatter(summary.matter_id)
  const documents = getDocumentsByMatter(summary.matter_id)
  const facts = getFactsByMatter(summary.matter_id)
  const timeline = getEventsByMatter(summary.matter_id)
  const evidence = getEvidenceByMatter(summary.matter_id)
  const defenses = getDefensesByMatter(summary.matter_id)
  const claims = [...getClaimsByMatter(summary.matter_id), ...defenses.map(normalizeDefenseAsClaim)]
  const deadlines = getDeadlinesByMatter(summary.matter_id)
  const tasks = getTasksByMatter(summary.matter_id)
  const drafts = getDraftsByMatter(summary.matter_id)
  const chatHistory = getAskMatterSeed(summary.matter_id).map(normalizeChatMessage)

  return {
    ...summary,
    id: summary.matter_id,
    title: summary.name,
    shortName: summary.shortName ?? shortMatterName(summary.name),
    parties,
    documents,
    facts,
    timeline,
    claims,
    evidence,
    defenses,
    deadlines,
    tasks,
    drafts,
    work_products: [],
    fact_check_findings: [],
    citation_check_findings: [],
    chatHistory,
    recentThreads: chatHistory.length
      ? [
          {
            id: "thread:fed-defenses",
            title: "FED complaint defenses",
            preview: chatHistory[chatHistory.length - 1]?.content.slice(0, 120) ?? "",
            date: chatHistory[0]?.timestamp ?? summary.updated_at,
            lastMessageAt: chatHistory[chatHistory.length - 1]?.timestamp ?? summary.updated_at,
            messageCount: chatHistory.length,
          },
        ]
      : [],
    milestones: timeline.slice(0, 5).map((event) => ({
      id: `milestone:${event.id}`,
      title: event.title,
      date: event.date,
      kind: event.kind === "filing" ? "filed" : event.kind === "court" ? "trial" : "intake",
      status: "complete",
      label: event.category,
      description: event.description,
    })),
  }
}

export function getMatterById(id: string): Matter | undefined {
  const normalizedId = normalizeId(id)
  const summary = matters.find((m) => m.matter_id === normalizedId)
  return summary ? buildMatter(summary) : undefined
}

export function getDocumentsByMatter(id: string): CaseDocument[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  const facts = getFactsByMatter(id)
  return (documentsSmithAbc as LegacyDocument[]).map((document) => normalizeDocument(document, facts))
}

export function getDocumentById(matterId: string, docId: string): CaseDocument | undefined {
  const normalizedDocId = normalizeId(docId)
  return getDocumentsByMatter(matterId).find((d) => d.document_id === normalizedDocId || d.id === normalizedDocId)
}

export function getFactsByMatter(id: string): CaseFact[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return (factsSmithAbc as LegacyFact[]).map(normalizeFact)
}

export function getEventsByMatter(id: string): CaseEvent[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return (eventsSmithAbc as LegacyEvent[]).map(normalizeEvent)
}

export function getEvidenceByMatter(id: string): CaseEvidence[] {
  if (normalizeId(id) === SMITH_MATTER_ID) return evidenceSmithAbc
  return []
}

export function getClaimsByMatter(id: string): CaseClaim[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return (claimsSmithAbc as LegacyClaim[]).map(normalizeClaim)
}

export function getDefensesByMatter(id: string): CaseDefense[] {
  if (normalizeId(id) === SMITH_MATTER_ID) return defensesSmithAbc
  return []
}

export function getDeadlinesByMatter(id: string): CaseDeadline[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return (deadlinesSmithAbc as LegacyDeadline[]).map(normalizeDeadline)
}

export function getTasksByMatter(id: string): CaseTask[] {
  if (normalizeId(id) === SMITH_MATTER_ID) return tasksSmithAbc
  return []
}

export function getPartiesByMatter(id: string): MatterParty[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return partiesSmithAbc.map(normalizeParty)
}

export function getDraftsByMatter(id: string): CaseDraft[] {
  if (normalizeId(id) !== SMITH_MATTER_ID) return []
  return [normalizeDraft(draftAnswerSmithAbc as LegacyDraft)]
}

export function getDraftById(matterId: string, draftId: string): CaseDraft | undefined {
  const normalizedDraftId = normalizeId(draftId)
  return getDraftsByMatter(matterId).find((d) => d.draft_id === normalizedDraftId || d.id === normalizedDraftId)
}

export function getDocumentExtraction(matterId: string, documentId: string): DocumentExtraction | undefined {
  if (normalizeId(matterId) !== SMITH_MATTER_ID) return undefined
  const normalizedDocumentId = normalizeId(documentId)
  return (
    documentExtractionsSmithAbc[normalizedDocumentId] ?? {
      document_id: normalizedDocumentId,
      summary: "Extraction pending - this document has been processed but no AI inspector summary is stored in the demo seed.",
      key_dates: [],
      parties: [],
      entities: [],
      possible_facts: [],
      possible_claims: [],
      possible_defenses: [],
      citations: [],
      contradictions: [],
      exhibit_notes: [],
    }
  )
}

export function getAskMatterSeed(matterId: string): MatterAskMessage[] {
  if (normalizeId(matterId) === SMITH_MATTER_ID) return askMatterSmithAbc as MatterAskMessage[]
  return []
}
