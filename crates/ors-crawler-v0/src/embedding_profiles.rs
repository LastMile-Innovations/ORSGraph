use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct EmbeddingProfile {
    pub name: &'static str,
    pub label: &'static str,
    pub model: &'static str,
    pub output_dimension: i32,
    pub output_dtype: &'static str,
    pub neo4j_property: &'static str,
    pub neo4j_index_name: &'static str,
    pub purpose: &'static str,
}

pub const LEGAL_CHUNK_PRIMARY: EmbeddingProfile = EmbeddingProfile {
    name: "legal_chunk_primary_v1",
    label: "RetrievalChunk",
    model: "voyage-4-large",
    output_dimension: 1024,
    output_dtype: "float",
    neo4j_property: "embedding",
    neo4j_index_name: "retrieval_chunk_embedding_1024",
    purpose: "high-accuracy legal authority retrieval",
};

pub const LEGAL_PROVISION_PRIMARY: EmbeddingProfile = EmbeddingProfile {
    name: "legal_provision_primary_v1",
    label: "Provision",
    model: "voyage-4-large",
    output_dimension: 1024,
    output_dtype: "float",
    neo4j_property: "embedding",
    neo4j_index_name: "provision_embedding_1024",
    purpose: "exact provision-level retrieval",
};

pub const LEGAL_VERSION_PRIMARY: EmbeddingProfile = EmbeddingProfile {
    name: "legal_version_primary_v1",
    label: "LegalTextVersion",
    model: "voyage-4-large",
    output_dimension: 1024,
    output_dtype: "float",
    neo4j_property: "embedding",
    neo4j_index_name: "legal_text_version_embedding_1024",
    purpose: "whole-statute retrieval where statute fits context",
};

pub const LEGAL_CHUNK_COMPACT: EmbeddingProfile = EmbeddingProfile {
    name: "legal_chunk_compact_v1",
    label: "RetrievalChunk",
    model: "voyage-4-large",
    output_dimension: 256,
    output_dtype: "float",
    neo4j_property: "embedding_256",
    neo4j_index_name: "retrieval_chunk_embedding_256",
    purpose: "fast broad recall / cheap secondary index",
};

macro_rules! profile {
    ($const_name:ident, $name:literal, $label:literal, $index:literal, $purpose:literal) => {
        pub const $const_name: EmbeddingProfile = EmbeddingProfile {
            name: $name,
            label: $label,
            model: "voyage-4-large",
            output_dimension: 1024,
            output_dtype: "float",
            neo4j_property: "embedding",
            neo4j_index_name: $index,
            purpose: $purpose,
        };
    };
}

profile!(
    LEGAL_SEMANTIC_NODE_PRIMARY,
    "legal_semantic_node_primary_v1",
    "LegalSemanticNode",
    "legal_semantic_node_embedding_1024",
    "semantic legal exploration for generic semantic nodes"
);
profile!(
    LEGAL_OBLIGATION_PRIMARY,
    "legal_obligation_primary_v1",
    "Obligation",
    "obligation_embedding_1024",
    "legal duty and obligation retrieval"
);
profile!(
    LEGAL_DEADLINE_PRIMARY,
    "legal_deadline_primary_v1",
    "Deadline",
    "deadline_embedding_1024",
    "legal deadline retrieval"
);
profile!(
    LEGAL_PENALTY_PRIMARY,
    "legal_penalty_primary_v1",
    "Penalty",
    "penalty_embedding_1024",
    "penalty and sanction retrieval"
);
profile!(
    LEGAL_EXCEPTION_PRIMARY,
    "legal_exception_primary_v1",
    "Exception",
    "exception_embedding_1024",
    "exception and carveout retrieval"
);
profile!(
    LEGAL_REMEDY_PRIMARY,
    "legal_remedy_primary_v1",
    "Remedy",
    "remedy_embedding_1024",
    "legal remedy retrieval"
);
profile!(
    LEGAL_DEFINITION_PRIMARY,
    "legal_definition_primary_v1",
    "Definition",
    "definition_embedding_1024",
    "legal definition retrieval"
);
profile!(
    LEGAL_DEFINED_TERM_PRIMARY,
    "legal_defined_term_primary_v1",
    "DefinedTerm",
    "defined_term_embedding_1024",
    "defined term autocomplete and search"
);
profile!(
    LEGAL_DEFINITION_SCOPE_PRIMARY,
    "legal_definition_scope_primary_v1",
    "DefinitionScope",
    "definition_scope_embedding_1024",
    "scope-sensitive definition retrieval"
);
profile!(
    LEGAL_SOURCE_NOTE_PRIMARY,
    "legal_source_note_primary_v1",
    "SourceNote",
    "source_note_embedding_1024",
    "legal source note retrieval"
);
profile!(
    LEGAL_STATUS_EVENT_PRIMARY,
    "legal_status_event_primary_v1",
    "StatusEvent",
    "status_event_embedding_1024",
    "status/currentness event retrieval"
);
profile!(
    LEGAL_TEMPORAL_EFFECT_PRIMARY,
    "legal_temporal_effect_primary_v1",
    "TemporalEffect",
    "temporal_effect_embedding_1024",
    "temporal effect retrieval"
);
profile!(
    LEGAL_LINEAGE_EVENT_PRIMARY,
    "legal_lineage_event_primary_v1",
    "LineageEvent",
    "lineage_event_embedding_1024",
    "statutory lineage retrieval"
);
profile!(
    LEGAL_AMENDMENT_PRIMARY,
    "legal_amendment_primary_v1",
    "Amendment",
    "amendment_embedding_1024",
    "amendment retrieval"
);
profile!(
    LEGAL_SESSION_LAW_PRIMARY,
    "legal_session_law_primary_v1",
    "SessionLaw",
    "session_law_embedding_1024",
    "session law retrieval"
);
profile!(
    LEGAL_REQUIRED_NOTICE_PRIMARY,
    "legal_required_notice_primary_v1",
    "RequiredNotice",
    "required_notice_embedding_1024",
    "required notice retrieval"
);
profile!(
    LEGAL_FORM_TEXT_PRIMARY,
    "legal_form_text_primary_v1",
    "FormText",
    "form_text_embedding_1024",
    "statutory form text retrieval"
);
profile!(
    LEGAL_MONEY_AMOUNT_PRIMARY,
    "legal_money_amount_primary_v1",
    "MoneyAmount",
    "money_amount_embedding_1024",
    "money amount retrieval"
);
profile!(
    LEGAL_TAX_RULE_PRIMARY,
    "legal_tax_rule_primary_v1",
    "TaxRule",
    "tax_rule_embedding_1024",
    "tax rule retrieval"
);
profile!(
    LEGAL_RATE_LIMIT_PRIMARY,
    "legal_rate_limit_primary_v1",
    "RateLimit",
    "rate_limit_embedding_1024",
    "rate limit retrieval"
);
profile!(
    LEGAL_ACTOR_PRIMARY,
    "legal_actor_primary_v1",
    "LegalActor",
    "legal_actor_embedding_1024",
    "legal actor retrieval"
);
profile!(
    LEGAL_ACTION_PRIMARY,
    "legal_action_primary_v1",
    "LegalAction",
    "legal_action_embedding_1024",
    "legal action retrieval"
);

pub const PRIMARY_PROFILES: &[&EmbeddingProfile] = &[
    &LEGAL_CHUNK_PRIMARY,
    &LEGAL_PROVISION_PRIMARY,
    &LEGAL_VERSION_PRIMARY,
    &LEGAL_SEMANTIC_NODE_PRIMARY,
    &LEGAL_OBLIGATION_PRIMARY,
    &LEGAL_EXCEPTION_PRIMARY,
    &LEGAL_DEADLINE_PRIMARY,
    &LEGAL_PENALTY_PRIMARY,
    &LEGAL_REMEDY_PRIMARY,
    &LEGAL_DEFINITION_PRIMARY,
    &LEGAL_DEFINED_TERM_PRIMARY,
    &LEGAL_DEFINITION_SCOPE_PRIMARY,
    &LEGAL_SOURCE_NOTE_PRIMARY,
    &LEGAL_STATUS_EVENT_PRIMARY,
    &LEGAL_TEMPORAL_EFFECT_PRIMARY,
    &LEGAL_LINEAGE_EVENT_PRIMARY,
    &LEGAL_AMENDMENT_PRIMARY,
    &LEGAL_SESSION_LAW_PRIMARY,
    &LEGAL_REQUIRED_NOTICE_PRIMARY,
    &LEGAL_FORM_TEXT_PRIMARY,
    &LEGAL_MONEY_AMOUNT_PRIMARY,
    &LEGAL_TAX_RULE_PRIMARY,
    &LEGAL_RATE_LIMIT_PRIMARY,
    &LEGAL_ACTOR_PRIMARY,
    &LEGAL_ACTION_PRIMARY,
];

pub fn get_embedding_profile(name: &str) -> Option<&'static EmbeddingProfile> {
    match name {
        "legal_chunk_primary_v1" => Some(&LEGAL_CHUNK_PRIMARY),
        "legal_provision_primary_v1" => Some(&LEGAL_PROVISION_PRIMARY),
        "legal_version_primary_v1" => Some(&LEGAL_VERSION_PRIMARY),
        "legal_semantic_node_primary_v1" => Some(&LEGAL_SEMANTIC_NODE_PRIMARY),
        "legal_obligation_primary_v1" => Some(&LEGAL_OBLIGATION_PRIMARY),
        "legal_exception_primary_v1" => Some(&LEGAL_EXCEPTION_PRIMARY),
        "legal_deadline_primary_v1" => Some(&LEGAL_DEADLINE_PRIMARY),
        "legal_penalty_primary_v1" => Some(&LEGAL_PENALTY_PRIMARY),
        "legal_remedy_primary_v1" => Some(&LEGAL_REMEDY_PRIMARY),
        "legal_definition_primary_v1" => Some(&LEGAL_DEFINITION_PRIMARY),
        "legal_defined_term_primary_v1" => Some(&LEGAL_DEFINED_TERM_PRIMARY),
        "legal_definition_scope_primary_v1" => Some(&LEGAL_DEFINITION_SCOPE_PRIMARY),
        "legal_source_note_primary_v1" => Some(&LEGAL_SOURCE_NOTE_PRIMARY),
        "legal_status_event_primary_v1" => Some(&LEGAL_STATUS_EVENT_PRIMARY),
        "legal_temporal_effect_primary_v1" => Some(&LEGAL_TEMPORAL_EFFECT_PRIMARY),
        "legal_lineage_event_primary_v1" => Some(&LEGAL_LINEAGE_EVENT_PRIMARY),
        "legal_amendment_primary_v1" => Some(&LEGAL_AMENDMENT_PRIMARY),
        "legal_session_law_primary_v1" => Some(&LEGAL_SESSION_LAW_PRIMARY),
        "legal_required_notice_primary_v1" => Some(&LEGAL_REQUIRED_NOTICE_PRIMARY),
        "legal_form_text_primary_v1" => Some(&LEGAL_FORM_TEXT_PRIMARY),
        "legal_money_amount_primary_v1" => Some(&LEGAL_MONEY_AMOUNT_PRIMARY),
        "legal_tax_rule_primary_v1" => Some(&LEGAL_TAX_RULE_PRIMARY),
        "legal_rate_limit_primary_v1" => Some(&LEGAL_RATE_LIMIT_PRIMARY),
        "legal_actor_primary_v1" => Some(&LEGAL_ACTOR_PRIMARY),
        "legal_action_primary_v1" => Some(&LEGAL_ACTION_PRIMARY),
        "legal_chunk_compact_v1" => Some(&LEGAL_CHUNK_COMPACT),
        _ => None,
    }
}

pub fn default_chunk_profile() -> &'static EmbeddingProfile {
    &LEGAL_CHUNK_PRIMARY
}

/// Truncate a float32 embedding to a shorter dimension.
/// For Matryoshka embeddings, the leading dimensions remain valid.
pub fn truncate_embedding_f32(v: &[f32], dim: usize) -> Vec<f32> {
    v.iter().copied().take(dim).collect()
}

/// Normalize a float32 embedding to unit vector (L2 norm).
pub fn normalize_embedding_f32(v: &mut [f32]) -> Result<()> {
    let norm: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return Err(anyhow!("Cannot normalize embedding with zero norm"));
    }
    for val in v.iter_mut() {
        *val /= norm;
    }
    Ok(())
}

/// Truncate and normalize a float32 embedding in one operation.
pub fn truncate_and_normalize_embedding_f32(v: &[f32], dim: usize) -> Result<Vec<f32>> {
    let mut truncated = truncate_embedding_f32(v, dim);
    normalize_embedding_f32(&mut truncated)?;
    Ok(truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_embedding_profile() {
        assert_eq!(
            get_embedding_profile("legal_chunk_primary_v1")
                .unwrap()
                .name,
            "legal_chunk_primary_v1"
        );
        assert_eq!(
            get_embedding_profile("legal_provision_primary_v1")
                .unwrap()
                .name,
            "legal_provision_primary_v1"
        );
        assert!(get_embedding_profile("unknown_profile").is_none());
    }

    #[test]
    fn test_default_chunk_profile() {
        assert_eq!(default_chunk_profile().name, "legal_chunk_primary_v1");
    }

    #[test]
    fn test_truncate_embedding_f32() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let truncated = truncate_embedding_f32(&v, 3);
        assert_eq!(truncated, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_normalize_embedding_f32() {
        let mut v = vec![3.0, 4.0];
        normalize_embedding_f32(&mut v).unwrap();
        let norm: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero_norm() {
        let mut v = vec![0.0, 0.0];
        assert!(normalize_embedding_f32(&mut v).is_err());
    }

    #[test]
    fn test_truncate_and_normalize_embedding_f32() {
        let v = vec![3.0, 4.0, 5.0, 6.0];
        let result = truncate_and_normalize_embedding_f32(&v, 2).unwrap();
        assert_eq!(result.len(), 2);
        let norm: f32 = result.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}
