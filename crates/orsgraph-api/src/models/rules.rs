use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleAuthoritySummary {
    pub authority_document_id: String,
    pub title: String,
    pub jurisdiction_id: String,
    pub jurisdiction: String,
    pub subcategory: String,
    pub authority_kind: String,
    pub authority_identifier: Option<String>,
    pub effective_start_date: String,
    pub effective_end_date: Option<String>,
    pub publication_bucket: String,
    pub date_status: String,
    pub status_flags: Vec<String>,
    pub topics: Vec<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CourtRulesRegistrySourceSummary {
    pub registry_source_id: String,
    pub source_type: String,
    pub jurisdiction: String,
    pub jurisdiction_id: String,
    pub source_url: String,
    pub snapshot_date: String,
    pub contains_current_future: bool,
    pub contains_prior: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CourtRulesRegistryResponse {
    pub sources: Vec<CourtRulesRegistrySourceSummary>,
    pub authorities: Vec<RuleAuthoritySummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleJurisdictionResponse {
    pub jurisdiction_id: String,
    pub as_of_date: Option<String>,
    pub authorities: Vec<RuleAuthoritySummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicableUtcrEdition {
    pub edition_id: String,
    pub corpus_id: String,
    pub edition_year: i64,
    pub effective_date: Option<String>,
    pub source_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleApplicabilityResponse {
    pub jurisdiction_id: String,
    pub jurisdiction: String,
    pub court: Option<String>,
    pub court_id: Option<String>,
    pub work_product_type: String,
    pub filing_date: String,
    pub utcr: Option<ApplicableUtcrEdition>,
    pub slr_edition: Option<RuleAuthoritySummary>,
    pub statewide_orders: Vec<RuleAuthoritySummary>,
    pub local_orders: Vec<RuleAuthoritySummary>,
    pub out_of_cycle_amendments: Vec<RuleAuthoritySummary>,
    pub currentness_warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupplementaryLocalRuleEditionResponse {
    pub jurisdiction_id: String,
    pub year: i64,
    pub edition: RuleAuthoritySummary,
}
