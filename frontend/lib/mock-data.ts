// Mock ORS data shaped to the JSON specs in the architecture doc.
// All data is derived from real ORS Chapter 3 structure (district attorneys / circuit courts)
// since the spec mentions Chapter 3 was the smoke-test corpus.

import type {
  StatutePageResponse,
  SearchResponse,
  AskAnswer,
  CorpusStatus,
  QCRunSummary,
  GraphNode,
  GraphEdge,
  Provision,
  StatuteIdentity,
  Chunk,
  OutboundCitation,
  InboundCitation,
  Definition,
  Exception,
  Deadline,
} from "./types"

export const corpusStatus: CorpusStatus = {
  editionYear: 2025,
  source: "Oregon Revised Statutes",
  lastUpdated: "2026-04-28T14:22:18Z",
  counts: {
    sections: 1284,
    versions: 1842,
    provisions: 8417,
    retrievalChunks: 19238,
    citationMentions: 4612,
    citesEdges: 4187,
    semanticNodes: 0,
    sourceNotes: 0,
    amendments: 0,
    sessionLaws: 0,
    neo4jNodes: 31802,
    neo4jRelationships: 67934,
  },
  citations: {
    total: 4612,
    resolved: 4187,
    unresolved: 425,
    citesEdges: 4187,
    coveragePercent: 90.79,
  },
  embeddings: {
    profile: "legal_chunk_primary_v1",
    embedded: 16802,
    totalEligible: 19238,
    coveragePercent: 87.33,
    status: "partial",
  },
}

// ---------------- Statute index ----------------

export const statuteIndex: StatuteIdentity[] = [
  {
    canonical_id: "or:ors:3.010",
    citation: "ORS 3.010",
    title: "Circuit courts; jurisdiction generally",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.012",
    citation: "ORS 3.012",
    title: "Judicial districts",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.050",
    citation: "ORS 3.050",
    title: "Number of circuit court judges",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.130",
    citation: "ORS 3.130",
    title: "District attorney duties",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.220",
    citation: "ORS 3.220",
    title: "Court reporters; appointment",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "amended",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.275",
    citation: "ORS 3.275",
    title: "Family court department; duties",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.405",
    citation: "ORS 3.405",
    title: "Juvenile court; jurisdiction",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:3.501",
    citation: "ORS 3.501",
    title: "Mental health court; establishment",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:8.610",
    citation: "ORS 8.610",
    title: "Office of district attorney; election",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "8",
    status: "active",
    edition: 2025,
  },
  {
    canonical_id: "or:ors:8.660",
    citation: "ORS 8.660",
    title: "Compensation of district attorneys",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "8",
    status: "amended",
    edition: 2025,
  },
]

// ---------------- Statute page (ORS 3.130 — DA duties — the killer demo target) ----------------

const ors_3_130_provisions: Provision[] = [
  {
    provision_id: "prov:or:ors:3.130",
    display_citation: "ORS 3.130",
    provision_type: "section",
    parent_id: null,
    text: "District attorney duties.",
    text_preview: "District attorney duties.",
    signals: [],
    cites_count: 4,
    cited_by_count: 23,
    chunk_count: 1,
    status: "active",
    children: [
      {
        provision_id: "prov:or:ors:3.130(1)",
        display_citation: "ORS 3.130(1)",
        provision_type: "subsection",
        parent_id: "prov:or:ors:3.130",
        text: "The district attorney in each county shall attend the terms of all courts having jurisdiction of public offenses within the district attorney's county and, except as otherwise provided in this section, conduct, on behalf of the state, all prosecutions for such offenses therein.",
        text_preview:
          "The district attorney in each county shall attend the terms of all courts having jurisdiction of public offenses...",
        signals: ["definition"],
        cites_count: 1,
        cited_by_count: 14,
        chunk_count: 2,
        status: "active",
        children: [
          {
            provision_id: "prov:or:ors:3.130(1)(a)",
            display_citation: "ORS 3.130(1)(a)",
            provision_type: "paragraph",
            parent_id: "prov:or:ors:3.130(1)",
            text: "Attend, on behalf of the state, all sessions of the circuit court for the county and conduct all prosecutions for public offenses cognizable in such court.",
            text_preview:
              "Attend, on behalf of the state, all sessions of the circuit court for the county...",
            signals: ["citation"],
            cites_count: 1,
            cited_by_count: 8,
            chunk_count: 1,
            status: "active",
          },
          {
            provision_id: "prov:or:ors:3.130(1)(b)",
            display_citation: "ORS 3.130(1)(b)",
            provision_type: "paragraph",
            parent_id: "prov:or:ors:3.130(1)",
            text: "Institute proceedings before magistrates for the arrest of persons charged with or reasonably suspected of public offenses.",
            text_preview:
              "Institute proceedings before magistrates for the arrest of persons charged with...",
            signals: [],
            cites_count: 0,
            cited_by_count: 3,
            chunk_count: 1,
            status: "active",
          },
        ],
      },
      {
        provision_id: "prov:or:ors:3.130(2)",
        display_citation: "ORS 3.130(2)",
        provision_type: "subsection",
        parent_id: "prov:or:ors:3.130",
        text: "When required by law, the district attorney shall give notice to the appropriate licensing or regulatory agency within 30 days after a conviction described in ORS 3.135.",
        text_preview:
          "When required by law, the district attorney shall give notice to the appropriate licensing...",
        signals: ["deadline", "citation"],
        cites_count: 1,
        cited_by_count: 6,
        chunk_count: 2,
        status: "active",
      },
      {
        provision_id: "prov:or:ors:3.130(3)",
        display_citation: "ORS 3.130(3)",
        provision_type: "subsection",
        parent_id: "prov:or:ors:3.130",
        text: "Notwithstanding subsection (1) of this section, the district attorney is not required to prosecute violations described in ORS 153.005 unless the violation is part of a criminal episode.",
        text_preview:
          "Notwithstanding subsection (1), the district attorney is not required to prosecute violations...",
        signals: ["exception", "citation"],
        cites_count: 1,
        cited_by_count: 4,
        chunk_count: 1,
        status: "active",
      },
      {
        provision_id: "prov:or:ors:3.130(4)",
        display_citation: "ORS 3.130(4)",
        provision_type: "subsection",
        parent_id: "prov:or:ors:3.130",
        text: "A violation of subsection (2) of this section by a district attorney is punishable as official misconduct under ORS 162.405.",
        text_preview: "A violation of subsection (2) by a district attorney is punishable as official misconduct...",
        signals: ["penalty", "citation"],
        cites_count: 1,
        cited_by_count: 2,
        chunk_count: 1,
        status: "active",
      },
    ],
  },
]

export const statutePage_3_130: StatutePageResponse = {
  identity: {
    canonical_id: "or:ors:3.130",
    citation: "ORS 3.130",
    title: "District attorney duties",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "3",
    status: "active",
    edition: 2025,
  },
  current_version: {
    version_id: "ver:or:ors:3.130:2025",
    effective_date: "2025-01-01",
    end_date: null,
    is_current: true,
    text: "District attorney duties.\n\n(1) The district attorney in each county shall attend the terms of all courts having jurisdiction of public offenses within the district attorney's county and, except as otherwise provided in this section, conduct, on behalf of the state, all prosecutions for such offenses therein. The district attorney shall:\n\n  (a) Attend, on behalf of the state, all sessions of the circuit court for the county and conduct all prosecutions for public offenses cognizable in such court.\n\n  (b) Institute proceedings before magistrates for the arrest of persons charged with or reasonably suspected of public offenses.\n\n(2) When required by law, the district attorney shall give notice to the appropriate licensing or regulatory agency within 30 days after a conviction described in ORS 3.135.\n\n(3) Notwithstanding subsection (1) of this section, the district attorney is not required to prosecute violations described in ORS 153.005 unless the violation is part of a criminal episode.\n\n(4) A violation of subsection (2) of this section by a district attorney is punishable as official misconduct under ORS 162.405.",
    source_documents: ["src:ors:2025:chapter:3"],
  },
  versions: [
    {
      version_id: "ver:or:ors:3.130:2025",
      effective_date: "2025-01-01",
      end_date: null,
      is_current: true,
      text: "(current)",
      source_documents: ["src:ors:2025:chapter:3"],
    },
    {
      version_id: "ver:or:ors:3.130:2023",
      effective_date: "2023-01-01",
      end_date: "2024-12-31",
      is_current: false,
      text: "(2023 edition)",
      source_documents: ["src:ors:2023:chapter:3"],
    },
    {
      version_id: "ver:or:ors:3.130:2021",
      effective_date: "2021-01-01",
      end_date: "2022-12-31",
      is_current: false,
      text: "(2021 edition)",
      source_documents: ["src:ors:2021:chapter:3"],
    },
  ],
  provisions: ors_3_130_provisions,
  chunks: [
    {
      chunk_id: "chunk:or:ors:3.130:full",
      chunk_type: "full_statute",
      source_kind: "statute",
      source_id: "or:ors:3.130",
      text: "District attorney duties. (1) The district attorney in each county shall attend...",
      embedding_policy: "primary",
      answer_policy: "preferred",
      search_weight: 1.0,
      embedded: true,
      parser_confidence: 0.98,
    },
    {
      chunk_id: "chunk:or:ors:3.130(1):context",
      chunk_type: "contextual_provision",
      source_kind: "provision",
      source_id: "prov:or:ors:3.130(1)",
      text: "[Within ORS 3.130 — District attorney duties] The district attorney in each county shall attend the terms of all courts...",
      embedding_policy: "primary",
      answer_policy: "preferred",
      search_weight: 0.95,
      embedded: true,
      parser_confidence: 0.97,
    },
    {
      chunk_id: "chunk:or:ors:3.130(1):def",
      chunk_type: "definition_block",
      source_kind: "provision",
      source_id: "prov:or:ors:3.130(1)",
      text: "Definition: 'district attorney' as used in this section means the elected county prosecutor...",
      embedding_policy: "primary",
      answer_policy: "supporting",
      search_weight: 0.85,
      embedded: true,
      parser_confidence: 0.92,
    },
    {
      chunk_id: "chunk:or:ors:3.130(2):deadline",
      chunk_type: "deadline_block",
      source_kind: "provision",
      source_id: "prov:or:ors:3.130(2)",
      text: "Deadline: 30 days after conviction. Trigger: conviction described in ORS 3.135.",
      embedding_policy: "primary",
      answer_policy: "preferred",
      search_weight: 1.0,
      embedded: true,
      parser_confidence: 0.94,
    },
    {
      chunk_id: "chunk:or:ors:3.130(3):exception",
      chunk_type: "exception_block",
      source_kind: "provision",
      source_id: "prov:or:ors:3.130(3)",
      text: "Exception: prosecution of ORS 153.005 violations not required unless part of a criminal episode.",
      embedding_policy: "primary",
      answer_policy: "preferred",
      search_weight: 0.9,
      embedded: true,
      parser_confidence: 0.96,
    },
    {
      chunk_id: "chunk:or:ors:3.130(4):penalty",
      chunk_type: "penalty_block",
      source_kind: "provision",
      source_id: "prov:or:ors:3.130(4)",
      text: "Penalty: violation of subsection (2) is official misconduct under ORS 162.405.",
      embedding_policy: "primary",
      answer_policy: "preferred",
      search_weight: 0.95,
      embedded: true,
      parser_confidence: 0.95,
    },
  ],
  definitions: [
    {
      definition_id: "def:or:ors:3.130:da",
      term: "district attorney",
      text: "The elected county prosecutor responsible for representing the state in criminal proceedings within the county.",
      source_provision: "ORS 3.130(1)",
      scope: "this section",
    },
    {
      definition_id: "def:or:ors:3.130:offense",
      term: "public offense",
      text: "Cross-referenced from ORS 161.505: an offense for which a sentence of imprisonment or fine may be imposed.",
      source_provision: "ORS 3.130(1)",
      scope: "this section",
    },
  ],
  exceptions: [
    {
      exception_id: "exc:or:ors:3.130(3)",
      text: "Prosecution of violations described in ORS 153.005 is not required unless the violation is part of a criminal episode.",
      applies_to_provision: "ORS 3.130(1)",
      source_provision: "ORS 3.130(3)",
    },
  ],
  deadlines: [
    {
      deadline_id: "dl:or:ors:3.130(2)",
      description: "Notice to licensing or regulatory agency",
      duration: "30 days",
      trigger: "after a conviction described in ORS 3.135",
      source_provision: "ORS 3.130(2)",
    },
  ],
  penalties: [
    {
      penalty_id: "pen:or:ors:3.130(4)",
      description: "Official misconduct (Class A misdemeanor) under ORS 162.405",
      category: "criminal",
      source_provision: "ORS 3.130(4)",
    },
  ],
  outbound_citations: [
    {
      target_canonical_id: "or:ors:3.135",
      target_citation: "ORS 3.135",
      context_snippet: "after a conviction described in ORS 3.135",
      source_provision: "ORS 3.130(2)",
      resolved: true,
    },
    {
      target_canonical_id: "or:ors:153.005",
      target_citation: "ORS 153.005",
      context_snippet: "violations described in ORS 153.005",
      source_provision: "ORS 3.130(3)",
      resolved: true,
    },
    {
      target_canonical_id: "or:ors:162.405",
      target_citation: "ORS 162.405",
      context_snippet: "punishable as official misconduct under ORS 162.405",
      source_provision: "ORS 3.130(4)",
      resolved: true,
    },
    {
      target_canonical_id: null,
      target_citation: "ORS chapter 131",
      context_snippet: "as further provided in ORS chapter 131",
      source_provision: "ORS 3.130(1)",
      resolved: false,
    },
  ],
  inbound_citations: [
    {
      source_canonical_id: "or:ors:8.610",
      source_citation: "ORS 8.610",
      source_title: "Office of district attorney; election",
      source_provision: "ORS 8.610(2)",
      context_snippet: "duties enumerated in ORS 3.130",
    },
    {
      source_canonical_id: "or:ors:8.660",
      source_citation: "ORS 8.660",
      source_title: "Compensation of district attorneys",
      source_provision: "ORS 8.660(1)",
      context_snippet: "for the duties described in ORS 3.130",
    },
    {
      source_canonical_id: "or:ors:131.005",
      source_citation: "ORS 131.005",
      source_title: "Definitions for criminal procedure",
      source_provision: "ORS 131.005(7)",
      context_snippet: "the prosecuting attorney as defined by ORS 3.130",
    },
    {
      source_canonical_id: "or:ors:135.703",
      source_citation: "ORS 135.703",
      source_title: "Compromise of misdemeanor",
      source_provision: "ORS 135.703(2)",
      context_snippet: "consent of the district attorney under ORS 3.130",
    },
    {
      source_canonical_id: "or:ors:419C.005",
      source_citation: "ORS 419C.005",
      source_title: "Jurisdiction of juvenile court",
      source_provision: "ORS 419C.005(3)",
      context_snippet: "as the district attorney's duties in ORS 3.130 require",
    },
  ],
  source_documents: [
    {
      source_id: "src:ors:2025:chapter:3",
      url: "https://www.oregonlegislature.gov/bills_laws/ors/ors003.html",
      retrieved_at: "2026-04-12T08:14:22Z",
      raw_hash: "sha256:9f4b8c2e7a1d0f5b6c3e8a2d9b7f1c4e6d8a0b2c5f7e9d1a3b5c7f9e1d3a5b7c",
      normalized_hash: "sha256:1a2b3c4d5e6f7890abcdef1234567890fedcba0987654321abcdef1234567890",
      edition_year: 2025,
      parser_profile: "ors-html-v3.2",
      parser_warnings: [],
    },
  ],
}

// ---------------- Search response ----------------

export const searchResponse: SearchResponse = {
  query: "district attorney duties",
  mode: "auto",
  results: [
    {
      result_type: "statute",
      citation: "ORS 3.130",
      title: "District attorney duties",
      snippet:
        "The district attorney in each county shall attend the terms of all courts having jurisdiction of public offenses...",
      source_provision: "ORS 3.130",
      edition_year: 2025,
      status: "active",
      cited_by_count: 23,
      cites_count: 4,
      score: 0.94,
      source_id: "or:ors:3.130",
      matched_chunk_type: "full_statute",
      chapter: "3",
      semantic_types: ["obligation", "deadline", "penalty"],
      source_backed: true,
    },
    {
      result_type: "provision",
      citation: "ORS 8.610(2)",
      title: "Office of district attorney; election",
      snippet:
        "The district attorney shall perform the duties enumerated in ORS 3.130 and such other duties as may be prescribed by law.",
      source_provision: "ORS 8.610(2)",
      edition_year: 2025,
      status: "active",
      cited_by_count: 11,
      cites_count: 2,
      score: 0.89,
      source_id: "prov:or:ors:8.610(2)",
      matched_chunk_type: "contextual_provision",
      chapter: "8",
      semantic_types: ["obligation"],
      source_backed: true,
    },
    {
      result_type: "chunk",
      citation: "ORS 3.130(1)",
      title: "Duty to attend court (definition block)",
      snippet:
        "Definition: 'district attorney' as used in this section means the elected county prosecutor responsible for representing the state...",
      source_provision: "ORS 3.130(1)",
      edition_year: 2025,
      status: "active",
      cited_by_count: 14,
      cites_count: 1,
      score: 0.86,
      source_id: "chunk:or:ors:3.130(1):def",
      matched_chunk_type: "definition_block",
      chapter: "3",
      semantic_types: ["definition"],
      source_backed: true,
    },
    {
      result_type: "provision",
      citation: "ORS 3.130(2)",
      title: "Deadline: notice after conviction",
      snippet:
        "When required by law, the district attorney shall give notice to the appropriate licensing or regulatory agency within 30 days...",
      source_provision: "ORS 3.130(2)",
      edition_year: 2025,
      status: "active",
      cited_by_count: 6,
      cites_count: 1,
      score: 0.83,
      source_id: "prov:or:ors:3.130(2)",
      matched_chunk_type: "deadline_block",
      chapter: "3",
      semantic_types: ["deadline", "notice"],
      source_backed: true,
    },
    {
      result_type: "statute",
      citation: "ORS 8.660",
      title: "Compensation of district attorneys",
      snippet:
        "Each district attorney shall receive an annual salary as provided by law for the duties described in ORS 3.130.",
      source_provision: "ORS 8.660",
      edition_year: 2025,
      status: "amended",
      cited_by_count: 5,
      cites_count: 3,
      score: 0.78,
      source_id: "or:ors:8.660",
      matched_chunk_type: "full_statute",
      chapter: "8",
      source_backed: true,
    },
    {
      result_type: "provision",
      citation: "ORS 3.130(3)",
      title: "Exception: violations under ORS 153.005",
      snippet:
        "Notwithstanding subsection (1), the district attorney is not required to prosecute violations described in ORS 153.005 unless the violation is part of a criminal episode.",
      source_provision: "ORS 3.130(3)",
      edition_year: 2025,
      status: "active",
      cited_by_count: 4,
      cites_count: 1,
      score: 0.76,
      source_id: "prov:or:ors:3.130(3)",
      matched_chunk_type: "exception_block",
      chapter: "3",
      semantic_types: ["exception"],
      source_backed: true,
    },
    {
      result_type: "citation",
      citation: "ORS 131.005(7)",
      title: "Definition of 'prosecuting attorney'",
      snippet: "...includes the prosecuting attorney as defined by ORS 3.130 and any deputy thereof.",
      source_provision: "ORS 131.005(7)",
      edition_year: 2025,
      status: "active",
      cited_by_count: 18,
      cites_count: 6,
      score: 0.71,
      source_id: "prov:or:ors:131.005(7)",
      matched_chunk_type: "citation_context",
      chapter: "131",
      source_backed: true,
    },
  ],
  analysis: {
    normalized_query: "district attorney duties",
    intent: "actor",
    citations: [],
    ranges: [],
    inferred_chapter: null,
    residual_text: null,
    expansion_terms: [],
    expansion_count: 0,
    applied_filters: [],
    timings: {
      total_ms: 142,
      retrieval_ms: 84,
      graph_ms: 22,
      rerank_ms: 0,
    },
  },
  total: 7,
}

// ---------------- Ask answer ----------------

export const askAnswer: AskAnswer = {
  question: "What Oregon laws define district attorney duties?",
  short_answer:
    "ORS 3.130 is the controlling statute defining district attorney duties in Oregon. It requires the DA in each county to attend all court sessions with jurisdiction over public offenses, conduct prosecutions on behalf of the state, and provide notice to licensing agencies within 30 days of certain convictions. ORS 8.610 establishes the office, and ORS 8.660 governs compensation tied to those duties.",
  controlling_law: [
    {
      citation: "ORS 3.130",
      canonical_id: "or:ors:3.130",
      reason: "Primary statute enumerating district attorney duties",
    },
    {
      citation: "ORS 8.610",
      canonical_id: "or:ors:8.610",
      reason: "Establishes the office of district attorney",
    },
    {
      citation: "ORS 8.660",
      canonical_id: "or:ors:8.660",
      reason: "Governs compensation for those duties",
    },
  ],
  relevant_provisions: [
    {
      citation: "ORS 3.130(1)",
      provision_id: "prov:or:ors:3.130(1)",
      text_preview:
        "The district attorney in each county shall attend the terms of all courts having jurisdiction of public offenses...",
    },
    {
      citation: "ORS 3.130(2)",
      provision_id: "prov:or:ors:3.130(2)",
      text_preview:
        "When required by law, the district attorney shall give notice to the appropriate licensing or regulatory agency within 30 days...",
    },
    {
      citation: "ORS 3.130(3)",
      provision_id: "prov:or:ors:3.130(3)",
      text_preview:
        "Notwithstanding subsection (1), the district attorney is not required to prosecute violations described in ORS 153.005...",
    },
    {
      citation: "ORS 8.610(2)",
      provision_id: "prov:or:ors:8.610(2)",
      text_preview: "The district attorney shall perform the duties enumerated in ORS 3.130 and such other duties...",
    },
  ],
  definitions: [
    {
      term: "district attorney",
      text: "The elected county prosecutor responsible for representing the state in criminal proceedings within the county.",
      source: "ORS 3.130(1)",
    },
    {
      term: "public offense",
      text: "An offense for which a sentence of imprisonment or fine may be imposed.",
      source: "ORS 161.505",
    },
  ],
  exceptions: [
    {
      text: "DA is not required to prosecute ORS 153.005 violations unless part of a criminal episode.",
      source: "ORS 3.130(3)",
    },
  ],
  deadlines: [
    {
      description: "Notice to licensing or regulatory agency after conviction",
      duration: "30 days",
      source: "ORS 3.130(2)",
    },
  ],
  citations: ["ORS 3.130", "ORS 3.135", "ORS 8.610", "ORS 8.660", "ORS 131.005", "ORS 153.005", "ORS 162.405"],
  caveats: [
    "Specific procedural duties may be modified by county-level practices not captured in the ORS.",
    "Range citation 'ORS chapter 131' in ORS 3.130(1) is unresolved in the current graph.",
  ],
  retrieved_chunks: [
    {
      chunk_id: "chunk:or:ors:3.130:full",
      chunk_type: "full_statute",
      score: 0.94,
      preview: "District attorney duties. (1) The district attorney in each county shall attend...",
    },
    {
      chunk_id: "chunk:or:ors:8.610(2):context",
      chunk_type: "contextual_provision",
      score: 0.89,
      preview: "The district attorney shall perform the duties enumerated in ORS 3.130...",
    },
    {
      chunk_id: "chunk:or:ors:3.130(1):def",
      chunk_type: "definition_block",
      score: 0.86,
      preview: "Definition: 'district attorney' as used in this section means the elected county prosecutor...",
    },
    {
      chunk_id: "chunk:or:ors:3.130(2):deadline",
      chunk_type: "deadline_block",
      score: 0.83,
      preview: "Deadline: 30 days after conviction. Trigger: conviction described in ORS 3.135.",
    },
  ],
}

// ---------------- QC run ----------------

export const qcRun: QCRunSummary = {
  run_id: "qc:run:2026-04-28T14:22:18Z",
  ran_at: "2026-04-28T14:22:18Z",
  duration_ms: 47213,
  status: "warning",
  total_checks: 1284,
  passed: 1247,
  warnings: 31,
  failures: 6,
  panels: [
    {
      panel_id: "panel:source",
      title: "Source validation",
      category: "source",
      status: "pass",
      count: 0,
      description: "Validates that source documents have valid hashes and parser profiles.",
      rows: [],
    },
    {
      panel_id: "panel:duplicate-provisions",
      title: "Duplicate provision paths",
      category: "parse",
      status: "warning",
      count: 4,
      description: "Two or more provisions claim the same path within a section.",
      rows: [
        {
          id: "qc:dup:1",
          citation: "ORS 174.100(1)(a)",
          message: "Path collides with ORS 174.100(1)(a) parsed from 2023 edition",
          level: "warning",
        },
        {
          id: "qc:dup:2",
          citation: "ORS 90.220(2)(b)",
          message: "Two provisions with identical display citation",
          level: "warning",
        },
        {
          id: "qc:dup:3",
          citation: "ORS 419B.875(3)",
          message: "Path collides with ORS 419B.875(3)",
          level: "warning",
        },
        {
          id: "qc:dup:4",
          citation: "ORS 656.265(4)(c)",
          message: "Duplicate paragraph numbering after subsection split",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:heading-leaks",
      title: "Heading leaks",
      category: "parse",
      status: "warning",
      count: 7,
      description: "Section headings appear inside provision text bodies.",
      rows: [
        {
          id: "qc:hl:1",
          citation: "ORS 3.220",
          message: "Heading 'Court reporters' leaked into provision text",
          level: "warning",
        },
        {
          id: "qc:hl:2",
          citation: "ORS 25.020(1)",
          message: "Subsection heading captured in body",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:repealed-active",
      title: "Repealed classified active",
      category: "parse",
      status: "fail",
      count: 2,
      description: "Sections marked active in graph but flagged repealed in source.",
      rows: [
        {
          id: "qc:ra:1",
          citation: "ORS 109.510",
          message: "Source notes section repealed by 2023 c.398 — currently active in graph",
          level: "fail",
        },
        {
          id: "qc:ra:2",
          citation: "ORS 215.730",
          message: "Source notes section repealed — currently active in graph",
          level: "fail",
        },
      ],
    },
    {
      panel_id: "panel:missing-titles",
      title: "Missing titles",
      category: "parse",
      status: "warning",
      count: 3,
      description: "Sections without parsed titles.",
      rows: [
        {
          id: "qc:mt:1",
          citation: "ORS 192.345",
          message: "No title parsed",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:orphan-chunks",
      title: "Orphan chunks",
      category: "chunk",
      status: "warning",
      count: 12,
      description: "Chunks with no resolvable source provision.",
      rows: [
        {
          id: "qc:oc:1",
          citation: "—",
          message: "12 chunks reference deleted provision IDs",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:orphan-citations",
      title: "Orphan citations",
      category: "citation",
      status: "warning",
      count: 18,
      description: "Citation mentions whose source provision no longer exists.",
      rows: [
        {
          id: "qc:ocit:1",
          citation: "—",
          message: "18 mentions reference deleted provision IDs",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:unresolved-citations",
      title: "Unresolved citations",
      category: "citation",
      status: "warning",
      count: 425,
      description: "Citation mentions with no resolved target. Mostly range citations and legacy refs.",
      rows: [
        {
          id: "qc:uc:1",
          citation: "ORS 3.130(1)",
          message: "Range citation 'ORS chapter 131' has no exact target",
          level: "warning",
        },
        {
          id: "qc:uc:2",
          citation: "ORS 25.080",
          message: "Reference to 'former ORS 25.080' could not be resolved",
          level: "warning",
        },
        {
          id: "qc:uc:3",
          citation: "ORS 90.220(4)",
          message: "Reference to 'the federal Fair Housing Act' is external",
          level: "info",
        },
      ],
    },
    {
      panel_id: "panel:oversized-chunks",
      title: "Oversized chunks",
      category: "chunk",
      status: "warning",
      count: 9,
      description: "Chunks exceeding 8192 token target.",
      rows: [
        {
          id: "qc:os:1",
          citation: "ORS 419B.005",
          message: "Definitions block: 11,340 tokens",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:embedding-readiness",
      title: "Embedding readiness",
      category: "embedding",
      status: "warning",
      count: 2447,
      description: "Chunks not yet embedded into vector index.",
      rows: [
        {
          id: "qc:em:1",
          citation: "—",
          message: "2,447 chunks pending — 12.7% of corpus",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "panel:neo4j-topology",
      title: "Neo4j topology validation",
      category: "graph",
      status: "fail",
      count: 4,
      description: "Validates that all expected node types and edge types exist in Neo4j.",
      rows: [
        {
          id: "qc:n4:1",
          citation: "Statute → Provision",
          message: "CONTAINS edges count mismatch: 8,402 graph vs 8,417 expected",
          level: "fail",
        },
        {
          id: "qc:n4:2",
          citation: "Provision → CitationMention",
          message: "MENTIONS_CITATION edges count mismatch: 4,587 graph vs 4,612 expected",
          level: "fail",
        },
        {
          id: "qc:n4:3",
          citation: "CitationMention → Statute",
          message: "RESOLVES_TO edges count mismatch: 4,162 graph vs 4,187 expected",
          level: "fail",
        },
        {
          id: "qc:n4:4",
          citation: "Statute → LegalTextVersion",
          message: "HAS_VERSION edges count mismatch: 1,837 graph vs 1,842 expected",
          level: "fail",
        },
      ],
    },
  ],
}

// ---------------- Graph data (small subgraph around ORS 3.130) ----------------

export const graphNodes: GraphNode[] = [
  { id: "or:ors:3.130", label: "ORS 3.130", type: "Statute", status: "active" },
  { id: "or:ors:3.135", label: "ORS 3.135", type: "Statute", status: "active" },
  { id: "or:ors:153.005", label: "ORS 153.005", type: "Statute", status: "active" },
  { id: "or:ors:162.405", label: "ORS 162.405", type: "Statute", status: "active" },
  { id: "or:ors:8.610", label: "ORS 8.610", type: "Statute", status: "active" },
  { id: "or:ors:8.660", label: "ORS 8.660", type: "Statute", status: "amended" },
  { id: "or:ors:131.005", label: "ORS 131.005", type: "Statute", status: "active" },
  { id: "or:ors:135.703", label: "ORS 135.703", type: "Statute", status: "active" },
  { id: "or:ors:419C.005", label: "ORS 419C.005", type: "Statute", status: "active" },
  { id: "or:ors:161.505", label: "ORS 161.505", type: "Statute", status: "active" },
  {
    id: "def:da",
    label: "district attorney",
    type: "Definition",
  },
  { id: "dl:30day", label: "30-day notice", type: "Deadline" },
  { id: "exc:153.005", label: "ORS 153.005 exception", type: "Exception" },
  { id: "pen:misconduct", label: "Official misconduct", type: "Penalty" },
]

export const graphEdges: GraphEdge[] = [
  { id: "e1", source: "or:ors:3.130", target: "or:ors:3.135", type: "CITES" },
  { id: "e2", source: "or:ors:3.130", target: "or:ors:153.005", type: "CITES" },
  { id: "e3", source: "or:ors:3.130", target: "or:ors:162.405", type: "CITES" },
  { id: "e4", source: "or:ors:8.610", target: "or:ors:3.130", type: "CITES" },
  { id: "e5", source: "or:ors:8.660", target: "or:ors:3.130", type: "CITES" },
  { id: "e6", source: "or:ors:131.005", target: "or:ors:3.130", type: "CITES" },
  { id: "e7", source: "or:ors:135.703", target: "or:ors:3.130", type: "CITES" },
  { id: "e8", source: "or:ors:419C.005", target: "or:ors:3.130", type: "CITES" },
  { id: "e9", source: "or:ors:3.130", target: "def:da", type: "DEFINES" },
  { id: "e10", source: "or:ors:161.505", target: "or:ors:3.130", type: "DEFINES" },
  { id: "e11", source: "or:ors:3.130", target: "dl:30day", type: "HAS_DEADLINE" },
  { id: "e12", source: "or:ors:3.130", target: "exc:153.005", type: "EXCEPTION_TO" },
  { id: "e13", source: "or:ors:3.130", target: "pen:misconduct", type: "CITES" },
]

// ---------------- Recent items (left rail) ----------------

export const recentItems = [
  { citation: "ORS 3.130", title: "District attorney duties", canonical_id: "or:ors:3.130" },
  { citation: "ORS 8.610", title: "Office of district attorney", canonical_id: "or:ors:8.610" },
  { citation: "ORS 419C.005", title: "Juvenile court jurisdiction", canonical_id: "or:ors:419C.005" },
  { citation: "ORS 162.405", title: "Official misconduct", canonical_id: "or:ors:162.405" },
]

export const savedSearches = [
  { id: "ss:1", query: "district attorney duties", results: 7 },
  { id: "ss:2", query: "30 day notice deadline", results: 23 },
  { id: "ss:3", query: "circuit court jurisdiction", results: 41 },
]

// Lookup helpers used by routes
export function getStatuteByCanonicalId(id: string): StatutePageResponse | null {
  if (id === "or:ors:3.130") return statutePage_3_130

  // Fallback: synthesize a thin page for any other statute in the index,
  // so the UI works as a navigable shell even before full data is loaded.
  const ident = statuteIndex.find((s) => s.canonical_id === id)
  if (!ident) return null

  return {
    identity: ident,
    current_version: {
      version_id: `ver:${ident.canonical_id}:${ident.edition}`,
      effective_date: `${ident.edition}-01-01`,
      end_date: null,
      is_current: true,
      text: `${ident.citation} — ${ident.title}.\n\n[Full text not yet loaded for this section. The statute is indexed in the corpus but its provisions, chunks, and graph edges are pending the next ingestion run.]`,
      source_documents: [`src:ors:${ident.edition}:chapter:${ident.chapter}`],
    },
    versions: [
      {
        version_id: `ver:${ident.canonical_id}:${ident.edition}`,
        effective_date: `${ident.edition}-01-01`,
        end_date: null,
        is_current: true,
        text: "(current)",
        source_documents: [`src:ors:${ident.edition}:chapter:${ident.chapter}`],
      },
    ],
    provisions: [
      {
        provision_id: `prov:${ident.canonical_id}`,
        display_citation: ident.citation,
        provision_type: "section",
        parent_id: null,
        text: ident.title,
        text_preview: ident.title,
        signals: [],
        cites_count: 0,
        cited_by_count: 0,
        chunk_count: 0,
        status: ident.status,
      },
    ],
    chunks: [],
    definitions: [],
    exceptions: [],
    deadlines: [],
    penalties: [],
    outbound_citations: [],
    inbound_citations: [],
    source_documents: [
      {
        source_id: `src:ors:${ident.edition}:chapter:${ident.chapter}`,
        url: `https://www.oregonlegislature.gov/bills_laws/ors/ors${String(ident.chapter).padStart(3, "0")}.html`,
        retrieved_at: "2026-04-12T08:14:22Z",
        raw_hash: "sha256:pending",
        normalized_hash: "sha256:pending",
        edition_year: ident.edition,
        parser_profile: "ors-html-v3.2",
        parser_warnings: [],
      },
    ],
  }
}

// Walk the provision tree (which may be nested via children[]) and find a provision by id.
function walkProvisions(provisions: Provision[]): Provision[] {
  const out: Provision[] = []
  const stack = [...provisions]
  while (stack.length) {
    const p = stack.pop()!
    out.push(p)
    if (p.children && p.children.length) stack.push(...p.children)
  }
  return out
}

export interface ProvisionInspectorData {
  provision: Provision
  parent_statute: StatuteIdentity
  ancestors: { citation: string; provision_id: string; text_preview: string }[]
  siblings: { citation: string; provision_id: string; text_preview: string }[]
  children: Provision[]
  chunks: Chunk[]
  outbound_citations: OutboundCitation[]
  inbound_citations: InboundCitation[]
  definitions: Definition[]
  exceptions: Exception[]
  deadlines: Deadline[]
}

export function getProvisionById(provisionId: string): ProvisionInspectorData | null {
  const all = walkProvisions(statutePage_3_130.provisions)
  const provision = all.find((p) => p.provision_id === provisionId)
  if (!provision) return null

  const ancestors: { citation: string; provision_id: string; text_preview: string }[] = []
  let cursor: Provision | undefined = provision
  while (cursor && cursor.parent_id) {
    const parent = all.find((p) => p.provision_id === cursor!.parent_id)
    if (!parent) break
    ancestors.unshift({
      citation: parent.display_citation,
      provision_id: parent.provision_id,
      text_preview: parent.text_preview,
    })
    cursor = parent
  }

  const siblings = provision.parent_id
    ? all
        .filter((p) => p.parent_id === provision.parent_id && p.provision_id !== provision.provision_id)
        .map((p) => ({
          citation: p.display_citation,
          provision_id: p.provision_id,
          text_preview: p.text_preview,
        }))
    : []

  const children = provision.children ?? []

  const chunks = statutePage_3_130.chunks.filter(
    (c) => c.source_id === provision.provision_id || c.source_kind === "provision",
  )

  const outbound_citations = statutePage_3_130.outbound_citations.filter(
    (c) => c.source_provision === provision.display_citation,
  )

  const inbound_citations = statutePage_3_130.inbound_citations

  const definitions = statutePage_3_130.definitions.filter(
    (d) => d.source_provision === provision.display_citation,
  )
  const exceptions = statutePage_3_130.exceptions.filter(
    (e) => e.applies_to_provision === provision.display_citation || e.source_provision === provision.display_citation,
  )
  const deadlines = statutePage_3_130.deadlines.filter(
    (d) => d.source_provision === provision.display_citation,
  )

  return {
    provision,
    parent_statute: statutePage_3_130.identity,
    ancestors,
    siblings,
    children,
    chunks,
    outbound_citations,
    inbound_citations,
    definitions,
    exceptions,
    deadlines,
  }
}

// ===== QC Console: corpus-wide aggregate panels =====

export const qcCorpus: QCRunSummary = {
  run_id: "qc:run:2026-04-27T18:32:00Z",
  ran_at: "2026-04-27T18:32:00Z",
  duration_ms: 184_220,
  status: "warning",
  total_checks: 47_812,
  passed: 46_904,
  warnings: 812,
  failures: 96,
  panels: [
    {
      panel_id: "qc:source",
      title: "Source documents",
      category: "source",
      status: "pass",
      count: 0,
      description: "Hashes verified, retrieval timestamps fresh, no parser warnings exceeding threshold.",
      rows: [],
    },
    {
      panel_id: "qc:parse",
      title: "Parsing & structure",
      category: "parse",
      status: "warning",
      count: 14,
      description: "Provision boundary or numbering anomalies detected during structural extraction.",
      rows: [
        {
          id: "qc:parse:01",
          citation: "ORS 18.225(3)",
          message: "Parser fell back to heuristic boundary; subsection (3)(b) absorbed text from (3)(c).",
          level: "warning",
        },
        {
          id: "qc:parse:02",
          citation: "ORS 30.265",
          message: "Renumbered range detected — provisions (1)–(6) span two editions, version edges missing.",
          level: "warning",
        },
        {
          id: "qc:parse:03",
          citation: "ORS 109.119",
          message: "Outline numbering inconsistency: (a) appears before (1) in original source.",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "qc:duplicate",
      title: "Duplicate provisions",
      category: "parse",
      status: "fail",
      count: 6,
      description: "Two or more provisions share identical text + display citation across versions.",
      rows: [
        {
          id: "qc:dup:01",
          citation: "ORS 419B.005(1) (2023 / 2024)",
          message: "Identical text in current and prior edition — version edge missing, both flagged is_current=true.",
          level: "fail",
        },
        {
          id: "qc:dup:02",
          citation: "ORS 137.700(1) (2023 / 2024)",
          message: "Definition duplicated; one orphaned without parent statute version link.",
          level: "fail",
        },
      ],
    },
    {
      panel_id: "qc:orphan",
      title: "Orphan chunks",
      category: "chunk",
      status: "warning",
      count: 23,
      description: "Chunks whose source_id no longer resolves to a provision or statute version.",
      rows: [
        {
          id: "qc:orph:01",
          citation: "chunk:ctx:5481",
          message: "source_id prov:or:ors:419A.255 missing after edition rollover.",
          level: "warning",
        },
        {
          id: "qc:orph:02",
          citation: "chunk:def:9921",
          message: "Definition chunk references repealed provision ORS 90.100(38).",
          level: "warning",
        },
      ],
    },
    {
      panel_id: "qc:citations",
      title: "Unresolved citations",
      category: "citation",
      status: "warning",
      count: 412,
      description: "Citation mentions whose target could not be resolved against the corpus index.",
      rows: [
        {
          id: "qc:cite:01",
          citation: "ORS 30.265 → 'OAR chapter 137'",
          message: "Cross-corpus citation; OAR not loaded in current corpus.",
          level: "info",
        },
        {
          id: "qc:cite:02",
          citation: "ORS 419B.005 → 'Title 13'",
          message: "Title-level reference resolves to chapter range; needs disambiguation.",
          level: "warning",
        },
        {
          id: "qc:cite:03",
          citation: "ORS 18.225 → 'this section'",
          message: "Self-reference — should resolve to source statute, currently null.",
          level: "fail",
        },
      ],
    },
    {
      panel_id: "qc:graph",
      title: "Neo4j topology",
      category: "graph",
      status: "pass",
      count: 0,
      description: "All provision/statute/chunk edges present; no dangling MENTIONS_CITATION edges.",
      rows: [],
    },
    {
      panel_id: "qc:embedding",
      title: "Embedding readiness",
      category: "embedding",
      status: "warning",
      count: 38,
      description: "Primary chunks missing embeddings or with stale embedding model version.",
      rows: [
        {
          id: "qc:emb:01",
          citation: "chunk:full_statute:or:ors:18.225",
          message: "Embedded with model orsg-emb-v3 (stale); v4 available.",
          level: "warning",
        },
        {
          id: "qc:emb:02",
          citation: "chunk:contextual_provision:or:ors:30.265:1",
          message: "embedding_policy=primary but embedded=false.",
          level: "fail",
        },
      ],
    },
  ],
}

// ---------------- Home Page ----------------

import { SystemHealth, HomeAction, GraphInsightCard, FeaturedStatute, BuildInfo, HomePageData } from "./types"

export const mockSystemHealth: SystemHealth = {
  api: "mock",
  neo4j: "offline",
  graphMaterialization: "complete",
  embeddings: "not_started",
  rerank: "missing_key",
  lastSeededAt: "2026-04-28T14:20:00Z",
  lastCheckedAt: "2026-04-29T15:27:00Z"
}

export const mockHomeActions: HomeAction[] = [
  {
    title: "Search ORS",
    description: "Search statutes, provisions, definitions, obligations, deadlines, penalties, notices, source notes, and chunks.",
    href: "/search",
    icon: "Search",
    badges: ["keyword", "citation", "graph"],
  },
  {
    title: "Ask ORSGraph",
    description: "Ask graph-grounded legal questions over Oregon law with citations, provisions, definitions, and currentness warnings.",
    href: "/ask",
    icon: "MessageSquare",
    variant: "primary",
    badges: ["QA", "rerank-ready"],
  },
  {
    title: "Statute Intelligence",
    description: "Open a statute with provision tree, citations, definitions, duties, deadlines, penalties, source notes, and chunks.",
    href: "/statutes",
    icon: "BookOpen",
    badges: ["deep view", "source-backed"],
  },
  {
    title: "Citation Graph",
    description: "Visualize CITES, RESOLVES_TO, DEFINES, EXCEPTION_TO, HAS_VERSION, and semantic reasoning paths.",
    href: "/graph",
    icon: "Network",
    badges: ["Neo4j", "multi-hop"],
  },
  {
    title: "Graph Ops",
    description: "Track crawl, parse, resolve, seed, materialize, embed, Neo4j topology, API health, and graph runs.",
    href: "/admin",
    icon: "Activity",
    badges: ["internal", "pipeline"],
    status: "internal",
  }
]

export const mockGraphInsights: GraphInsightCard[] = [
  {
    title: "Most cited statute",
    value: "ORS 3.130",
    subtitle: "District attorney duties",
    href: "/statutes/or:ors:3.130",
    state: "ok",
  },
  {
    title: "Chapter 3",
    value: "Smoke corpus online",
    subtitle: "courts · districts · jurisdiction · duties",
    href: "/statutes?chapter=3",
    state: "ok",
  }
]

export const mockFeaturedStatutes: FeaturedStatute[] = [
  {
    citation: "ORS 3.130",
    title: "District attorney duties",
    chapter: "Chapter 3",
    href: "/statutes/or:ors:3.130",
    status: "active",
    semanticTypes: ["obligation", "deadline", "exception"],
    citedByCount: 23,
    sourceBacked: true,
  },
  {
    citation: "ORS 3.010",
    title: "Circuit courts; jurisdiction generally",
    chapter: "Chapter 3",
    href: "/statutes/or:ors:3.010",
    status: "active",
    semanticTypes: ["citation"],
    citedByCount: 12,
    sourceBacked: true,
  },
  {
    citation: "ORS 8.610",
    title: "Office of district attorney; election",
    chapter: "Chapter 8",
    href: "/statutes/or:ors:8.610",
    status: "active",
    semanticTypes: ["obligation"],
    sourceBacked: true,
  },
  {
    citation: "ORS 3.275",
    title: "Family court department; duties",
    chapter: "Chapter 3",
    href: "/statutes/or:ors:3.275",
    status: "active",
    semanticTypes: ["obligation", "citation"],
    sourceBacked: true,
  }
]

export const mockBuildInfo: BuildInfo = {
  appVersion: "0.1.0",
  apiVersion: "0.1.0",
  graphEdition: "ORS 2025",
  environment: "development"
}

export const mockHomePageData: HomePageData = {
  // We have corpusStatus already defined in mock-data, but since we redefined its shape, we'll cast or recreate it.
  corpus: {
    editionYear: 2025,
    source: "Oregon Revised Statutes",
    counts: {
      sections: 58688,
      versions: 58688,
      provisions: 244136,
      retrievalChunks: 373967,
      citationMentions: 79464,
      citesEdges: 58144,
      semanticNodes: 397059,
      sourceNotes: 41497,
      amendments: 102744,
      sessionLaws: 99131,
      neo4jNodes: 1797953,
      neo4jRelationships: 5429047
    },
    citations: {
      total: 79464,
      resolved: 58144,
      unresolved: 21320,
      citesEdges: 58144,
      coveragePercent: 73.17
    },
    embeddings: {
      model: "voyage-4-large",
      profile: "legal_chunk_primary_v1",
      embedded: 0,
      totalEligible: 373967,
      coveragePercent: 0,
      status: "not_started"
    }
  },
  health: mockSystemHealth,
  actions: mockHomeActions,
  insights: mockGraphInsights,
  featuredStatutes: mockFeaturedStatutes,
  build: mockBuildInfo,
}
