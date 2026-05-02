use crate::models::search::QueryExpansionTerm;

struct ConceptPack {
    id: &'static str,
    triggers: &'static [&'static str],
    terms: &'static [&'static str],
    source_citation: &'static str,
}

const CONCEPT_PACKS: &[ConceptPack] = &[
    ConceptPack {
        id: "deadlines",
        triggers: &["deadline", "due date", "time to", "within", "days after"],
        terms: &[
            "deadline",
            "time limit",
            "service date",
            "filing date",
            "computation of time",
        ],
        source_citation: "LegalConceptPack deadlines v1",
    },
    ConceptPack {
        id: "notices",
        triggers: &["notice", "notify", "serve notice", "written notice"],
        terms: &[
            "written notice",
            "service",
            "mailing",
            "delivery",
            "recipient",
        ],
        source_citation: "LegalConceptPack notices v1",
    },
    ConceptPack {
        id: "remedies",
        triggers: &["remedy", "damages", "relief", "injunction", "penalty"],
        terms: &[
            "damages",
            "equitable relief",
            "civil penalty",
            "injunction",
            "attorney fees",
        ],
        source_citation: "LegalConceptPack remedies v1",
    },
    ConceptPack {
        id: "definitions",
        triggers: &["define", "definition", "means", "term"],
        terms: &[
            "means",
            "includes",
            "does not include",
            "defined term",
            "scope",
        ],
        source_citation: "LegalConceptPack definitions v1",
    },
    ConceptPack {
        id: "landlord_tenant",
        triggers: &["landlord", "tenant", "rental", "dwelling", "habitability"],
        terms: &[
            "residential tenancy",
            "dwelling unit",
            "habitability",
            "possession",
            "rent",
        ],
        source_citation: "LegalConceptPack landlord_tenant v1",
    },
    ConceptPack {
        id: "filing_service",
        triggers: &["file", "filing", "serve", "service", "certificate"],
        terms: &[
            "filing requirement",
            "service requirement",
            "certificate of service",
            "efiling",
            "proof of service",
        ],
        source_citation: "LegalConceptPack filing_service v1",
    },
    ConceptPack {
        id: "jurisdiction",
        triggers: &["court", "venue", "jurisdiction", "county", "circuit"],
        terms: &[
            "subject matter jurisdiction",
            "venue",
            "circuit court",
            "local rule",
            "statewide rule",
        ],
        source_citation: "LegalConceptPack jurisdiction v1",
    },
];

pub fn expand_concepts(query: &str, limit: usize) -> Vec<QueryExpansionTerm> {
    let lower = query.to_ascii_lowercase();
    let mut terms = Vec::new();
    for pack in CONCEPT_PACKS {
        if !pack.triggers.iter().any(|trigger| lower.contains(trigger)) {
            continue;
        }
        for term in pack.terms {
            if terms.len() >= limit {
                return terms;
            }
            terms.push(QueryExpansionTerm {
                term: (*term).to_string(),
                normalized_term: Some(term.to_ascii_lowercase()),
                kind: "legal_concept_pack".to_string(),
                source_id: Some(format!("legal_concept_pack:{}", pack.id)),
                source_citation: Some(pack.source_citation.to_string()),
                score: 0.35,
            });
        }
    }
    terms
}
