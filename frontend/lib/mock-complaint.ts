import type { ComplaintAnalysis } from "./types"

export const complaintAnalysis: ComplaintAnalysis = {
  complaint_id: "cmp:2026-04-22:henderson",
  filename: "Henderson_v_Multnomah_Complaint.pdf",
  uploaded_at: "2026-04-22T10:14:00Z",
  court: "Multnomah County Circuit Court",
  case_number: "26CV-04482",
  user_role: "defendant",
  service_date: "2026-04-18",
  summary:
    "Plaintiff Sarah Henderson, individually and on behalf of her minor child M.H., brings four claims against Multnomah County and DHS arising from a juvenile dependency placement decision made on March 12, 2026. Plaintiff alleges procedural due-process violations, negligence, intentional infliction of emotional distress, and statutory tort claims. The complaint seeks $850,000 in damages plus injunctive relief.",
  parties: [
    { party_id: "pty1", name: "Sarah Henderson", role: "plaintiff", type: "individual" },
    { party_id: "pty2", name: "M.H. (a minor, by Sarah Henderson)", role: "plaintiff", type: "individual" },
    { party_id: "pty3", name: "Multnomah County", role: "defendant", type: "government" },
    { party_id: "pty4", name: "Oregon Department of Human Services", role: "defendant", type: "government" },
    { party_id: "pty5", name: "Jane Doe (DHS caseworker)", role: "defendant", type: "individual" },
  ],
  claims: [
    {
      claim_id: "cl1",
      count_label: "Count I",
      title: "Procedural Due Process — 42 U.S.C. § 1983",
      cause_of_action: "civil_rights_section_1983",
      required_elements: [
        {
          element_id: "el1",
          text: "Defendant acted under color of state law",
          alleged: true,
          proven: true,
          authority: "42 U.S.C. § 1983",
        },
        {
          element_id: "el2",
          text: "Defendant deprived plaintiff of a constitutional right",
          alleged: true,
          proven: false,
          authority: "U.S. Const. amend. XIV",
        },
        {
          element_id: "el3",
          text: "Plaintiff received inadequate process before deprivation",
          alleged: true,
          proven: false,
          authority: "Mathews v. Eldridge balancing",
        },
      ],
      alleged_facts: [
        "March 12, 2026 — DHS removed M.H. without prior judicial authorization.",
        "No notice of right to be heard was provided before removal.",
        "Hearing on placement was held 7 days post-removal.",
      ],
      missing_facts: [
        "Whether emergency exception applied at the time of removal.",
        "Whether plaintiff received written notice within 24 hours per ORS 419B.150.",
      ],
      potential_defenses: [
        { name: "Qualified immunity (individual defendant)", authority: "Saucier v. Katz", viability: "high" },
        { name: "Emergency exigency exception", authority: "ORS 419B.150", viability: "medium" },
        { name: "Discretionary function immunity (OTCA)", authority: "ORS 30.265(6)(c)", viability: "medium" },
      ],
      relevant_law: [
        {
          citation: "ORS 419B.150",
          canonical_id: "or:ors:419B.150",
          reason: "Emergency removal procedures and notice requirements.",
        },
        {
          citation: "ORS 419B.100",
          canonical_id: "or:ors:419B.100",
          reason: "Juvenile court jurisdiction over dependency cases.",
        },
      ],
      risk_level: "high",
    },
    {
      claim_id: "cl2",
      count_label: "Count II",
      title: "Negligence",
      cause_of_action: "negligence",
      required_elements: [
        { element_id: "el4", text: "Duty of care", alleged: true, proven: true, authority: "ORS 30.265" },
        { element_id: "el5", text: "Breach of duty", alleged: true, proven: false, authority: "Restatement (Second) Torts § 282" },
        { element_id: "el6", text: "Causation", alleged: true, proven: false, authority: "Joshi v. Providence" },
        { element_id: "el7", text: "Damages", alleged: true, proven: true, authority: "ORS 30.265(2)" },
      ],
      alleged_facts: [
        "DHS caseworker failed to verify reporting party's identity.",
        "Caseworker did not interview M.H. before removal.",
      ],
      missing_facts: [
        "Specific standard of care for emergency dependency intake.",
        "Causal link between breach and alleged emotional damages.",
      ],
      potential_defenses: [
        {
          name: "OTCA discretionary immunity",
          authority: "ORS 30.265(6)(c)",
          viability: "high",
        },
        {
          name: "Failure to file timely tort claim notice",
          authority: "ORS 30.275(2)(b)",
          viability: "low",
        },
      ],
      relevant_law: [
        {
          citation: "ORS 30.265",
          canonical_id: "or:ors:30.265",
          reason: "Oregon Tort Claims Act — public body liability framework.",
        },
        {
          citation: "ORS 30.275",
          canonical_id: "or:ors:30.275",
          reason: "Tort claim notice (270 days for personal injury).",
        },
      ],
      risk_level: "medium",
    },
    {
      claim_id: "cl3",
      count_label: "Count III",
      title: "Intentional Infliction of Emotional Distress",
      cause_of_action: "iied",
      required_elements: [
        {
          element_id: "el8",
          text: "Intentional or reckless conduct",
          alleged: true,
          proven: false,
          authority: "McGanty v. Staudenraus",
        },
        {
          element_id: "el9",
          text: "Extreme and outrageous conduct",
          alleged: true,
          proven: false,
          authority: "Delaney v. Clifton",
        },
        {
          element_id: "el10",
          text: "Severe emotional distress",
          alleged: true,
          proven: false,
          authority: "Restatement (Second) Torts § 46",
        },
      ],
      alleged_facts: [
        "Caseworker allegedly stated plaintiff would 'never see her child again.'",
        "Removal occurred during plaintiff's birthday gathering.",
      ],
      missing_facts: [
        "Independent witness corroboration of statement.",
        "Medical/psychological documentation of distress.",
      ],
      potential_defenses: [
        { name: "First-element insufficiency (mere insults)", authority: "Patton v. J.C. Penney", viability: "high" },
        { name: "OTCA cap and limitations", authority: "ORS 30.272", viability: "medium" },
      ],
      relevant_law: [
        {
          citation: "ORS 30.272",
          canonical_id: "or:ors:30.272",
          reason: "OTCA damages caps for tort actions against local public bodies.",
        },
      ],
      risk_level: "medium",
    },
    {
      claim_id: "cl4",
      count_label: "Count IV",
      title: "Statutory Tort — Failure to Comply with ORS 419B.150",
      cause_of_action: "statutory_tort",
      required_elements: [
        {
          element_id: "el11",
          text: "Statute imposes mandatory duty",
          alleged: true,
          proven: true,
          authority: "ORS 419B.150(1)",
        },
        {
          element_id: "el12",
          text: "Plaintiff is within protected class",
          alleged: true,
          proven: true,
          authority: "ORS 419B.005(1)(b)",
        },
        {
          element_id: "el13",
          text: "Failure to perform mandated duty",
          alleged: true,
          proven: false,
          authority: "ORS 419B.150(2)",
        },
        {
          element_id: "el14",
          text: "Resulting damages",
          alleged: true,
          proven: false,
          authority: "ORS 30.265(2)",
        },
      ],
      alleged_facts: [
        "ORS 419B.150 requires written notice within 24 hours of removal.",
        "Plaintiff alleges no written notice was received until day 5.",
      ],
      missing_facts: ["Documentary proof of when notice was actually delivered."],
      potential_defenses: [
        { name: "Substantial compliance doctrine", authority: "ORS 174.020", viability: "medium" },
        { name: "Implied private right of action lacking", authority: "Doyle v. Oregon Bank", viability: "high" },
      ],
      relevant_law: [
        {
          citation: "ORS 419B.150",
          canonical_id: "or:ors:419B.150",
          reason: "Mandatory notice requirements for emergency removals.",
        },
      ],
      risk_level: "low",
    },
  ],
  allegations: [
    {
      allegation_id: "a1",
      paragraph: 1,
      text: "Plaintiff Sarah Henderson is a resident of Multnomah County, Oregon.",
      suggested_response: "admit",
      reason: "Identity and residency verified by client intake.",
      evidence_needed: [],
    },
    {
      allegation_id: "a2",
      paragraph: 2,
      text: "Defendant Multnomah County is a political subdivision of the State of Oregon.",
      suggested_response: "admit",
      reason: "Public record; not a contested fact.",
      evidence_needed: [],
    },
    {
      allegation_id: "a3",
      paragraph: 3,
      text: "On March 12, 2026, Defendant removed M.H. from Plaintiff's custody without prior court order.",
      suggested_response: "deny_in_part",
      reason: "Removal date confirmed; deny lack of authority — emergency removal authority under ORS 419B.150.",
      evidence_needed: ["Caseworker report from 3/12/2026", "Emergency assessment form ER-12"],
    },
    {
      allegation_id: "a4",
      paragraph: 4,
      text: "Defendant failed to provide written notice of the removal to Plaintiff for at least 5 days.",
      suggested_response: "deny",
      reason: "Records show notice delivered within 24 hours via certified mail.",
      evidence_needed: ["Certified mail receipt", "Form NOR-3 timestamp"],
    },
    {
      allegation_id: "a5",
      paragraph: 5,
      text: "Defendant's conduct was extreme and outrageous and exceeded all bounds of decency.",
      suggested_response: "legal_conclusion",
      reason: "Conclusory characterization, not a factual allegation requiring response.",
      evidence_needed: [],
    },
    {
      allegation_id: "a6",
      paragraph: 6,
      text: "DHS caseworker Jane Doe stated to Plaintiff that 'you will never see your child again.'",
      suggested_response: "lack_knowledge",
      reason: "No internal record of statement; caseworker disputes; await discovery.",
      evidence_needed: ["Caseworker deposition", "Recording of intake (if any)"],
    },
    {
      allegation_id: "a7",
      paragraph: 7,
      text: "Plaintiff has suffered severe emotional distress including anxiety, depression, and sleep disturbance.",
      suggested_response: "lack_knowledge",
      reason: "Lacks documentation; standard response pending medical records review.",
      evidence_needed: ["Medical records", "Treatment provider statements"],
    },
    {
      allegation_id: "a8",
      paragraph: 8,
      text: "Plaintiff timely filed notice of tort claim on April 1, 2026.",
      suggested_response: "needs_review",
      reason: "Notice received but compliance with ORS 30.275(4) content requirements requires legal review.",
      evidence_needed: ["Tort claim notice copy", "Receipt log from County risk management"],
    },
  ],
  deadlines: [
    {
      deadline_id: "dl1",
      description: "Answer due (30 days after service)",
      due_date: "2026-05-18",
      days_remaining: 20,
      source_citation: "ORCP 7 D",
      severity: "critical",
    },
    {
      deadline_id: "dl2",
      description: "Motion to dismiss deadline (in lieu of answer)",
      due_date: "2026-05-18",
      days_remaining: 20,
      source_citation: "ORCP 21",
      severity: "critical",
    },
    {
      deadline_id: "dl3",
      description: "Tort claim notice review (270-day window from incident)",
      due_date: "2026-12-07",
      days_remaining: 223,
      source_citation: "ORS 30.275(2)(b)",
      severity: "info",
    },
    {
      deadline_id: "dl4",
      description: "Initial discovery disclosures",
      due_date: "2026-06-15",
      days_remaining: 48,
      source_citation: "UTCR 5.150",
      severity: "warning",
    },
  ],
  defense_candidates: [
    {
      name: "Qualified immunity",
      authority: "Saucier v. Katz; Pearson v. Callahan",
      rationale: "Strong fit for individual caseworker defendant on § 1983 claim.",
      viability: "high",
    },
    {
      name: "OTCA discretionary function immunity",
      authority: "ORS 30.265(6)(c)",
      rationale: "Removal decisions in dependency intake are discretionary policy choices.",
      viability: "high",
    },
    {
      name: "Failure to state IIED",
      authority: "Patton v. J.C. Penney",
      rationale: "Pleaded conduct does not meet 'extreme and outrageous' threshold.",
      viability: "high",
    },
    {
      name: "OTCA damages cap",
      authority: "ORS 30.272",
      rationale: "Applies to all surviving tort claims regardless of liability.",
      viability: "medium",
    },
  ],
  motion_candidates: [
    {
      name: "Motion to Dismiss (Count III — IIED)",
      authority: "ORCP 21 A(8)",
      basis: "Failure to state a claim — facts insufficient as a matter of law.",
    },
    {
      name: "Motion for Summary Judgment (Counts II & IV)",
      authority: "ORCP 47",
      basis: "OTCA discretionary immunity disposes of negligence and statutory tort claims.",
    },
    {
      name: "Motion to Strike Conclusory Allegations",
      authority: "ORCP 21 E",
      basis: "Paragraphs 5, 12, 18 contain only legal conclusions.",
    },
  ],
  counterclaim_candidates: [],
  evidence_checklist: [
    { item: "Caseworker incident report (3/12/2026)", obtained: true, needed_for: "Counts I, II, IV" },
    { item: "Form NOR-3 notice timestamp + delivery receipt", obtained: false, needed_for: "Count IV" },
    { item: "Emergency assessment form ER-12", obtained: true, needed_for: "Count I" },
    { item: "Caseworker training records (Jane Doe)", obtained: false, needed_for: "Counts I, II" },
    { item: "Plaintiff's medical/psych records", obtained: false, needed_for: "Counts II, III" },
    { item: "DHS internal policy DPM-419", obtained: false, needed_for: "Discretionary immunity defense" },
    { item: "Tort claim notice (4/1/2026)", obtained: true, needed_for: "Procedural defense" },
  ],
  draft_answer_preview: `IN THE CIRCUIT COURT FOR THE STATE OF OREGON
FOR THE COUNTY OF MULTNOMAH

SARAH HENDERSON, individually and as guardian
ad litem of M.H., a minor,
                                  Plaintiffs,
        v.                                       Case No. 26CV-04482
MULTNOMAH COUNTY, et al.,
                                  Defendants.

ANSWER, AFFIRMATIVE DEFENSES, AND DEMAND FOR JURY TRIAL

Defendant Multnomah County, by and through its undersigned counsel, hereby answers
Plaintiff's Complaint as follows:

   PARAGRAPH 1: ADMITTED.
   PARAGRAPH 2: ADMITTED.
   PARAGRAPH 3: DENIED IN PART. Defendant admits that on or about March 12, 2026,
      M.H. was placed in protective custody, but DENIES the characterization that
      such removal was unauthorized. The removal was conducted pursuant to the
      emergency removal authority of ORS 419B.150.
   PARAGRAPH 4: DENIED. Written notice was timely delivered within the period
      required by ORS 419B.150(2).
   PARAGRAPH 5: This paragraph contains a legal conclusion to which no response
      is required. To the extent a response is deemed required, Defendant DENIES.
   ...

   AFFIRMATIVE DEFENSES
   1. Discretionary function immunity, ORS 30.265(6)(c).
   2. Tort claim notice insufficiency, ORS 30.275.
   3. Damages cap, ORS 30.272.
   4. Failure to state a claim (Count III), ORCP 21 A(8).
   ...`,
}
