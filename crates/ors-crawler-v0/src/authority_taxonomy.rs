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

pub fn authority_level_for_family(authority_family: &str) -> i32 {
    match normalize_family(authority_family).as_str() {
        "USCONST" => AUTHORITY_LEVEL_US_CONSTITUTION,
        "FEDERALSTATUTE" | "USC" => AUTHORITY_LEVEL_FEDERAL_STATUTE,
        "ORCONST" | "STATECONSTITUTION" => AUTHORITY_LEVEL_STATE_CONSTITUTION,
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
        "USCONST" | "ORCONST" | "STATECONSTITUTION" => "constitution",
        "FEDERALSTATUTE" | "USC" | "ORS" => "statute",
        "FEDERALRULE" | "FRCP" | "FRE" | "FRAP" | "CFR" | "UTCR" | "ORCP" | "ORAP" | "OAR"
        | "SLR" | "LOCALRULE" => "rule",
        "CONAN" | "OFFICIALCOMMENTARY" => "official_commentary",
        "CASELAW" => "case_law",
        _ => "secondary",
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
        "USCONST" | "FEDERALSTATUTE" | "USC" | "ORCONST" | "STATECONSTITUTION" | "ORS"
        | "FEDERALRULE" | "FRCP" | "FRE" | "FRAP" | "CFR" | "UTCR" | "ORCP" | "ORAP" | "OAR"
        | "SLR" | "LOCALRULE" => "primary_law",
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

fn normalize_family(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}
