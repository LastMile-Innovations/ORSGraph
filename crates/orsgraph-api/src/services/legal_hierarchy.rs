use crate::models::search::SearchResult;

pub const AUTHORITY_LEVEL_US_CONSTITUTION: i32 = 100;
pub const AUTHORITY_LEVEL_FEDERAL_STATUTE: i32 = 92;
pub const AUTHORITY_LEVEL_STATE_CONSTITUTION: i32 = 91;
pub const AUTHORITY_LEVEL_STATE_STATUTE: i32 = 90;
pub const AUTHORITY_LEVEL_FEDERAL_RULE: i32 = 84;
pub const AUTHORITY_LEVEL_STATE_RULE: i32 = 80;
pub const AUTHORITY_LEVEL_LOCAL_RULE: i32 = 75;
pub const AUTHORITY_LEVEL_OFFICIAL_COMMENTARY: i32 = 65;
pub const AUTHORITY_LEVEL_CASE_LAW: i32 = 60;
pub const AUTHORITY_LEVEL_SECONDARY: i32 = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct LegalHierarchyMetadata {
    pub authority_level: i32,
    pub authority_tier: &'static str,
    pub source_role: &'static str,
    pub jurisdiction_id: &'static str,
    pub is_primary_law: bool,
    pub is_official_commentary: bool,
    pub controlling_weight: f32,
}

impl LegalHierarchyMetadata {
    pub fn for_result(result: &SearchResult) -> Self {
        let family = result
            .authority_family
            .as_deref()
            .or_else(|| result.citation.as_deref().and_then(infer_authority_family))
            .unwrap_or("secondary");
        let level = result
            .authority_level
            .unwrap_or_else(|| authority_level_for_family(family));
        let tier = authority_tier_for_level(level);
        let role = result
            .source_role
            .as_deref()
            .map(normalize_source_role)
            .unwrap_or_else(|| source_role_for_family(family));
        let jurisdiction_id = result
            .jurisdiction_id
            .as_deref()
            .map(normalize_jurisdiction)
            .unwrap_or_else(|| jurisdiction_for_family(family, result.corpus_id.as_deref()));
        let is_primary_law = role == "primary_law";
        let is_official_commentary = role == "official_commentary";

        Self {
            authority_level: level,
            authority_tier: tier,
            source_role: role,
            jurisdiction_id,
            is_primary_law,
            is_official_commentary,
            controlling_weight: controlling_weight(level, role, jurisdiction_id),
        }
    }
}

pub fn enrich_result(result: &mut SearchResult) {
    let metadata = LegalHierarchyMetadata::for_result(result);
    result.authority_level = Some(metadata.authority_level);
    result.authority_tier = Some(metadata.authority_tier.to_string());
    result.source_role = Some(metadata.source_role.to_string());
    result.jurisdiction_id = Some(metadata.jurisdiction_id.to_string());
    result.primary_law = Some(metadata.is_primary_law);
    result.official_commentary = Some(metadata.is_official_commentary);
    result.controlling_weight = Some(metadata.controlling_weight);
}

pub fn authority_level_for_family(authority_family: &str) -> i32 {
    match normalize_family(authority_family).as_str() {
        "USCONST" => AUTHORITY_LEVEL_US_CONSTITUTION,
        "FEDERALSTATUTE" | "USC" => AUTHORITY_LEVEL_FEDERAL_STATUTE,
        "STATECONSTITUTION" => AUTHORITY_LEVEL_STATE_CONSTITUTION,
        "ORS" => AUTHORITY_LEVEL_STATE_STATUTE,
        "FEDERALRULE" | "FRCP" | "FRE" | "FRAP" | "CFR" => AUTHORITY_LEVEL_FEDERAL_RULE,
        "UTCR" | "ORCP" | "ORAP" | "OAR" => AUTHORITY_LEVEL_STATE_RULE,
        "SLR" | "LOCALRULE" => AUTHORITY_LEVEL_LOCAL_RULE,
        "CONAN" | "OFFICIALCOMMENTARY" => AUTHORITY_LEVEL_OFFICIAL_COMMENTARY,
        "CASELAW" => AUTHORITY_LEVEL_CASE_LAW,
        _ => AUTHORITY_LEVEL_SECONDARY,
    }
}

pub fn authority_type_for_family(authority_family: &str) -> &'static str {
    match normalize_family(authority_family).as_str() {
        "USCONST" | "STATECONSTITUTION" => "constitution",
        "FEDERALSTATUTE" | "USC" | "ORS" => "statute",
        "FEDERALRULE" | "FRCP" | "FRE" | "FRAP" | "CFR" | "UTCR" | "ORCP" | "ORAP" | "OAR"
        | "SLR" | "LOCALRULE" => "rule",
        "CONAN" | "OFFICIALCOMMENTARY" => "official_commentary",
        "CASELAW" => "case_law",
        _ => "secondary",
    }
}

pub fn corpus_id_for_family(authority_family: &str) -> Option<&'static str> {
    match normalize_family(authority_family).as_str() {
        "USCONST" => Some("us:constitution"),
        "CONAN" => Some("us:conan"),
        "USC" | "FEDERALSTATUTE" => Some("us:usc"),
        "CFR" => Some("us:cfr"),
        "FRCP" => Some("us:frcp"),
        "FRE" => Some("us:fre"),
        "FRAP" => Some("us:frap"),
        "UTCR" => Some("or:utcr"),
        "ORS" => Some("or:ors"),
        _ => None,
    }
}

pub fn authority_tier_for_level(level: i32) -> &'static str {
    match level {
        100 => "constitution",
        91..=99 => "statute",
        84..=90 => "statute",
        75..=83 => "rule",
        65..=74 => "official_commentary",
        60..=64 => "case_law",
        _ => "secondary",
    }
}

pub fn source_role_for_family(authority_family: &str) -> &'static str {
    match normalize_family(authority_family).as_str() {
        "USCONST" | "FEDERALSTATUTE" | "USC" | "STATECONSTITUTION" | "ORS" | "FEDERALRULE"
        | "FRCP" | "FRE" | "FRAP" | "CFR" | "UTCR" | "ORCP" | "ORAP" | "OAR" | "SLR"
        | "LOCALRULE" => "primary_law",
        "CONAN" | "OFFICIALCOMMENTARY" => "official_commentary",
        "CASELAW" => "case_law",
        _ => "secondary",
    }
}

pub fn jurisdiction_for_family(authority_family: &str, corpus_id: Option<&str>) -> &'static str {
    if matches!(
        normalize_family(authority_family).as_str(),
        "USCONST" | "CONAN"
    ) || corpus_id.is_some_and(|id| id.starts_with("us:"))
    {
        "us"
    } else if matches!(
        normalize_family(authority_family).as_str(),
        "SLR" | "LOCALRULE"
    ) {
        "local"
    } else {
        "or:state"
    }
}

pub fn infer_authority_family(citation: &str) -> Option<&'static str> {
    let upper = citation.trim().to_ascii_uppercase();
    if upper.starts_with("U.S. CONST")
        || upper.starts_with("US CONST")
        || upper.starts_with("UNITED STATES CONST")
        || upper.contains(" AMENDMENT")
        || upper.contains("DUE PROCESS CLAUSE")
    {
        Some("USCONST")
    } else if upper.starts_with("CONAN ") || upper.starts_with("AMDT") || upper.starts_with("ART") {
        Some("CONAN")
    } else if upper.starts_with("UTCR ") {
        Some("UTCR")
    } else if upper.starts_with("ORS ") {
        Some("ORS")
    } else {
        None
    }
}

pub fn matches_authority_tier(result: &SearchResult, tier: &str) -> bool {
    let expected = normalize_tier(tier);
    LegalHierarchyMetadata::for_result(result).authority_tier == expected
}

pub fn matches_source_role(result: &SearchResult, role: &str) -> bool {
    let expected = normalize_source_role(role);
    LegalHierarchyMetadata::for_result(result).source_role == expected
}

fn controlling_weight(level: i32, role: &str, jurisdiction_id: &str) -> f32 {
    let base = match level {
        100 => 4.0,
        91..=99 => 2.8,
        84..=90 => 2.2,
        75..=83 => 1.4,
        65..=74 => 0.8,
        60..=64 => 0.6,
        _ => 0.0,
    };
    let role_weight = match role {
        "primary_law" => 0.8,
        "official_commentary" => -0.2,
        "case_law" => 0.1,
        _ => -0.4,
    };
    let jurisdiction_weight = if jurisdiction_id == "us" { 0.25 } else { 0.0 };
    base + role_weight + jurisdiction_weight
}

fn normalize_family(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn normalize_tier(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "constitution" | "constitutional" => "constitution",
        "statute" | "statutes" => "statute",
        "rule" | "rules" | "court_rule" | "court_rules" => "rule",
        "official_commentary" | "commentary" | "analysis" => "official_commentary",
        "case" | "case_law" | "caselaw" => "case_law",
        _ => "secondary",
    }
}

fn normalize_source_role(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "primary" | "primary_law" | "controlling" => "primary_law",
        "official_commentary" | "commentary" | "analysis" => "official_commentary",
        "case" | "case_law" | "caselaw" => "case_law",
        _ => "secondary",
    }
}

fn normalize_jurisdiction(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "us" | "federal" | "united_states" | "united states" => "us",
        "local" => "local",
        _ => "or:state",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constitution_controls_above_commentary() {
        let constitution = authority_level_for_family("USCONST");
        let conan = authority_level_for_family("CONAN");
        assert!(constitution > conan);
        assert_eq!(source_role_for_family("CONAN"), "official_commentary");
        assert_eq!(authority_type_for_family("USCONST"), "constitution");
    }
}
