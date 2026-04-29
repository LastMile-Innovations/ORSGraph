import type { FactCheckReport } from "./types"

export const factCheckReport: FactCheckReport = {
  document: {
    document_id: "doc:fc:2026-04-27:001",
    title: "Memorandum in Support of Motion to Dismiss — Henderson v. Multnomah County",
    doc_type: "memo",
    word_count: 1_247,
    uploaded_at: "2026-04-27T16:18:00Z",
    paragraphs: [
      {
        paragraph_id: "p1",
        index: 1,
        text: "Plaintiff seeks judicial review of a juvenile dependency determination under ORS chapter 419B. As set forth below, the Court should dismiss for failure to state a claim upon which relief can be granted.",
      },
      {
        paragraph_id: "p2",
        index: 2,
        text: "The circuit court has exclusive original jurisdiction over juvenile matters pursuant to ORS 3.130, which vests circuit courts with general civil jurisdiction across the State of Oregon.",
      },
      {
        paragraph_id: "p3",
        index: 3,
        text: "Under ORS 419B.005(1), a 'child' includes any person under 18 years of age. The statute was last amended in 2019 and remains the controlling definition for dependency proceedings.",
      },
      {
        paragraph_id: "p4",
        index: 4,
        text: "Pursuant to ORS 419B.100, the juvenile court has jurisdiction in any case involving a person under 18 years of age whose conditions or circumstances are such as to endanger the welfare of the person.",
      },
      {
        paragraph_id: "p5",
        index: 5,
        text: "Plaintiff was required to file the petition within 10 days of the placement decision under ORS 419B.875(2). Plaintiff filed on the eleventh day, rendering the petition untimely.",
      },
      {
        paragraph_id: "p6",
        index: 6,
        text: "The Oregon Supreme Court held in State v. Smith, 290 Or 350 (1981), that procedural deadlines in dependency proceedings are jurisdictional and may not be waived by the parties.",
      },
      {
        paragraph_id: "p7",
        index: 7,
        text: "Defendant Multnomah County is immune from suit under ORS 30.265, the Oregon Tort Claims Act, which provides absolute immunity for all government tort liability.",
      },
      {
        paragraph_id: "p8",
        index: 8,
        text: "ORS 30.275 requires notice of tort claim within 270 days of the alleged loss. Plaintiff failed to provide such notice and the claim is therefore barred.",
      },
      {
        paragraph_id: "p9",
        index: 9,
        text: "Finally, the Department of Human Services has primary administrative authority over dependency cases under former ORS 418.005, which has not been substantively changed in recent legislative sessions.",
      },
      {
        paragraph_id: "p10",
        index: 10,
        text: "For the foregoing reasons, Defendant respectfully requests the Court grant this Motion to Dismiss with prejudice.",
      },
    ],
  },
  findings: [
    {
      finding_id: "f1",
      paragraph_id: "p2",
      paragraph_index: 2,
      claim: "Circuit courts have exclusive original jurisdiction over juvenile matters under ORS 3.130.",
      status: "partially_supported",
      confidence: 0.82,
      explanation:
        "ORS 3.130 vests circuit courts with general civil jurisdiction, but juvenile dependency jurisdiction is established by ORS 419B.100, not ORS 3.130. The cited authority is correct as to general jurisdiction but does not specifically grant juvenile jurisdiction.",
      suggested_fix:
        "Cite ORS 419B.100 in addition to ORS 3.130, or replace with ORS 419B.100 which is the controlling juvenile-jurisdiction provision.",
      sources: [
        {
          citation: "ORS 3.130",
          canonical_id: "or:ors:3.130",
          quote: "The circuit courts have general civil jurisdiction throughout the state.",
          edition_year: 2025,
          status: "active",
        },
        {
          citation: "ORS 419B.100",
          canonical_id: "or:ors:419B.100",
          quote: "The juvenile court has jurisdiction in any case involving a person under 18 years of age...",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f2",
      paragraph_id: "p3",
      paragraph_index: 3,
      claim: "ORS 419B.005(1) defines 'child' as any person under 18 years of age and was last amended in 2019.",
      status: "wrong_citation",
      confidence: 0.91,
      explanation:
        "The 'child' definition in dependency proceedings is at ORS 419B.005(1)(b), not ORS 419B.005(1). Additionally, ORS 419B.005 was last substantively amended in 2021, not 2019.",
      suggested_fix:
        "Update citation to ORS 419B.005(1)(b) and amendment year to 2021. The QC graph flags this as a duplicate-provision warning.",
      sources: [
        {
          citation: "ORS 419B.005(1)(b)",
          canonical_id: "or:ors:419B.005",
          quote: "'Child' means a person who is under 18 years of age.",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f3",
      paragraph_id: "p4",
      paragraph_index: 4,
      claim: "ORS 419B.100 grants juvenile court jurisdiction over persons under 18 in endangerment circumstances.",
      status: "supported",
      confidence: 0.97,
      explanation:
        "Quoted text is verbatim from the official source. The provision is current as of the 2025 edition with no pending amendments.",
      suggested_fix: null,
      sources: [
        {
          citation: "ORS 419B.100",
          canonical_id: "or:ors:419B.100",
          quote:
            "The juvenile court has jurisdiction in any case involving a person under 18 years of age whose conditions or circumstances are such as to endanger the welfare of the person...",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f4",
      paragraph_id: "p5",
      paragraph_index: 5,
      claim:
        "ORS 419B.875(2) requires petition within 10 days of placement decision; eleventh-day filing is untimely.",
      status: "contradicted",
      confidence: 0.94,
      explanation:
        "ORS 419B.875(2) actually requires filing within 14 days, not 10 days. Plaintiff's eleventh-day filing is therefore TIMELY under the actual statute. This argument should be removed.",
      suggested_fix:
        "Remove this argument. Filing on day 11 is well within the 14-day statutory window. Continued reliance on this point would be a Rule 11 concern.",
      sources: [
        {
          citation: "ORS 419B.875(2)",
          canonical_id: "or:ors:419B.875",
          quote: "...within 14 days after the date of the placement decision.",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f5",
      paragraph_id: "p6",
      paragraph_index: 6,
      claim:
        "State v. Smith, 290 Or 350 (1981) held procedural deadlines in dependency proceedings are jurisdictional.",
      status: "needs_source",
      confidence: 0.55,
      explanation:
        "Case law is outside the current ORSGraph corpus (statutes only). Citation cannot be verified against the graph. Manual Westlaw/Lexis verification recommended.",
      suggested_fix:
        "Verify citation independently. ORSGraph case-law corpus is on the V2 roadmap.",
      sources: [],
    },
    {
      finding_id: "f6",
      paragraph_id: "p7",
      paragraph_index: 7,
      claim: "ORS 30.265 (Oregon Tort Claims Act) provides absolute immunity for all government tort liability.",
      status: "unsupported",
      confidence: 0.93,
      explanation:
        "ORS 30.265 establishes the Oregon Tort Claims Act framework but does NOT provide 'absolute immunity.' The Act provides limited tort liability for public bodies subject to specified caps, exclusions, and notice requirements. Several enumerated exceptions in ORS 30.265(6) preserve liability.",
      suggested_fix:
        "Rewrite to accurately describe the OTCA as a limited-liability and cap framework, citing the specific exceptions in ORS 30.265(6) that may or may not apply to plaintiff's claim.",
      sources: [
        {
          citation: "ORS 30.265",
          canonical_id: "or:ors:30.265",
          quote:
            "Subject to the limitations of ORS 30.260 to 30.300, every public body is subject to civil action for its torts and those of its officers, employees and agents...",
          edition_year: 2025,
          status: "active",
        },
        {
          citation: "ORS 30.265(6)",
          canonical_id: "or:ors:30.265",
          quote: "Every public body and its officers... are immune from liability for: ...",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f7",
      paragraph_id: "p8",
      paragraph_index: 8,
      claim: "ORS 30.275 requires tort claim notice within 270 days of the alleged loss.",
      status: "supported",
      confidence: 0.99,
      explanation:
        "ORS 30.275(2)(b) provides for 270-day notice period for personal injury claims against public bodies. Citation is correct and current.",
      suggested_fix: null,
      sources: [
        {
          citation: "ORS 30.275(2)(b)",
          canonical_id: "or:ors:30.275",
          quote:
            "Notice of claim shall be given within the following applicable period of time... 270 days after the alleged loss or injury.",
          edition_year: 2025,
          status: "active",
        },
      ],
    },
    {
      finding_id: "f8",
      paragraph_id: "p9",
      paragraph_index: 9,
      claim: "Former ORS 418.005 grants DHS primary administrative authority over dependency cases.",
      status: "stale_law",
      confidence: 0.88,
      explanation:
        "Former ORS 418.005 was repealed in 2017 and replaced by ORS 418.005 (renumbered). The substantive authority is now codified at ORS 418.005 (2018 ed.) and reorganized by 2023 amendments. Citing 'former ORS 418.005' as still controlling is misleading.",
      suggested_fix:
        "Update citation to current ORS 418.005 and note the 2023 reorganization. Remove 'has not been substantively changed' language.",
      sources: [
        {
          citation: "ORS 418.005",
          canonical_id: "or:ors:418.005",
          quote: "(current text of reorganized provision)",
          edition_year: 2025,
          status: "amended",
        },
      ],
    },
  ],
  summary: {
    total: 8,
    supported: 2,
    partial: 1,
    unsupported: 1,
    contradicted: 1,
    wrong_citation: 1,
    stale_law: 1,
    needs_source: 1,
  },
  citation_table: [
    {
      raw_citation: "ORS 3.130",
      resolved_citation: "ORS 3.130",
      canonical_id: "or:ors:3.130",
      edition_year: 2025,
      status: "active",
      qc_status: "pass",
      occurrences: [2],
    },
    {
      raw_citation: "ORS 419B.005(1)",
      resolved_citation: "ORS 419B.005(1)(b)",
      canonical_id: "or:ors:419B.005",
      edition_year: 2025,
      status: "active",
      qc_status: "warning",
      occurrences: [3],
    },
    {
      raw_citation: "ORS 419B.100",
      resolved_citation: "ORS 419B.100",
      canonical_id: "or:ors:419B.100",
      edition_year: 2025,
      status: "active",
      qc_status: "pass",
      occurrences: [4],
    },
    {
      raw_citation: "ORS 419B.875(2)",
      resolved_citation: "ORS 419B.875(2)",
      canonical_id: "or:ors:419B.875",
      edition_year: 2025,
      status: "active",
      qc_status: "pass",
      occurrences: [5],
    },
    {
      raw_citation: "State v. Smith, 290 Or 350 (1981)",
      resolved_citation: null,
      canonical_id: null,
      edition_year: null,
      status: "unresolved",
      qc_status: "warning",
      occurrences: [6],
    },
    {
      raw_citation: "ORS 30.265",
      resolved_citation: "ORS 30.265",
      canonical_id: "or:ors:30.265",
      edition_year: 2025,
      status: "active",
      qc_status: "pass",
      occurrences: [7],
    },
    {
      raw_citation: "ORS 30.275",
      resolved_citation: "ORS 30.275(2)(b)",
      canonical_id: "or:ors:30.275",
      edition_year: 2025,
      status: "active",
      qc_status: "pass",
      occurrences: [8],
    },
    {
      raw_citation: "former ORS 418.005",
      resolved_citation: "ORS 418.005",
      canonical_id: "or:ors:418.005",
      edition_year: 2025,
      status: "amended",
      qc_status: "warning",
      occurrences: [9],
    },
  ],
}
