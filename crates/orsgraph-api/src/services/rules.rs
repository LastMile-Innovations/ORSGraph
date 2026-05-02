use crate::error::{ApiError, ApiResult};
use crate::models::rules::{
    ApplicableUtcrEdition, CourtRulesRegistryResponse, CourtRulesRegistrySourceSummary,
    RuleApplicabilityResponse, RuleAuthoritySummary, RuleJurisdictionResponse,
    SupplementaryLocalRuleEditionResponse,
};
use neo4rs::{Row, query};
use regex::Regex;
use std::sync::{Arc, LazyLock};

static ISO_DATE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());

pub struct RuleApplicabilityResolver {
    graph: Arc<neo4rs::Graph>,
}

impl RuleApplicabilityResolver {
    pub fn new(graph: Arc<neo4rs::Graph>) -> Self {
        Self { graph }
    }

    pub async fn registry(&self) -> ApiResult<CourtRulesRegistryResponse> {
        let sources = self.registry_sources().await?;
        let authorities = self
            .authority_query(
                "MATCH (doc:RuleAuthorityDocument)
                 OPTIONAL MATCH (doc)-[:HAS_TOPIC]->(topic:RuleTopic)
                 WITH doc, collect(DISTINCT topic.name) AS topics
                 RETURN doc.authority_document_id AS authority_document_id,
                        doc.title AS title,
                        doc.jurisdiction_id AS jurisdiction_id,
                        doc.jurisdiction AS jurisdiction,
                        doc.subcategory AS subcategory,
                        doc.authority_kind AS authority_kind,
                        doc.authority_identifier AS authority_identifier,
                        doc.effective_start_date AS effective_start_date,
                        doc.effective_end_date AS effective_end_date,
                        doc.publication_bucket AS publication_bucket,
                        doc.date_status AS date_status,
                        coalesce(doc.status_flags, []) AS status_flags,
                        topics AS topics,
                        doc.source_url AS source_url
                 ORDER BY doc.jurisdiction_id, doc.effective_start_date, doc.subcategory, doc.title",
            )
            .await?;
        Ok(CourtRulesRegistryResponse {
            sources,
            authorities,
        })
    }

    pub async fn current_for_jurisdiction(
        &self,
        jurisdiction: &str,
    ) -> ApiResult<RuleJurisdictionResponse> {
        let jurisdiction_id = normalize_jurisdiction_id(jurisdiction);
        let jurisdiction_ids = self.jurisdiction_scope_ids(&jurisdiction_id).await?;
        let q = format!(
            "{AUTHORITY_RETURN_PREFIX}
             WHERE doc.jurisdiction_id IN $jurisdiction_ids
               AND date(doc.effective_start_date) <= date()
               AND (doc.effective_end_date IS NULL OR date(doc.effective_end_date) >= date())
             {AUTHORITY_RETURN_SUFFIX}"
        );
        let authorities = self
            .authority_query_params(&q, vec![("jurisdiction_ids", jurisdiction_ids)])
            .await?;
        Ok(RuleJurisdictionResponse {
            jurisdiction_id,
            as_of_date: None,
            authorities,
        })
    }

    pub async fn history_for_jurisdiction(
        &self,
        jurisdiction: &str,
    ) -> ApiResult<RuleJurisdictionResponse> {
        let jurisdiction_id = normalize_jurisdiction_id(jurisdiction);
        let jurisdiction_ids = self.jurisdiction_scope_ids(&jurisdiction_id).await?;
        let q = format!(
            "{AUTHORITY_RETURN_PREFIX}
             WHERE doc.jurisdiction_id IN $jurisdiction_ids
             {AUTHORITY_RETURN_SUFFIX}"
        );
        let authorities = self
            .authority_query_params(&q, vec![("jurisdiction_ids", jurisdiction_ids)])
            .await?;
        Ok(RuleJurisdictionResponse {
            jurisdiction_id,
            as_of_date: None,
            authorities,
        })
    }

    pub async fn applicable(
        &self,
        jurisdiction: &str,
        court: Option<&str>,
        work_product_type: &str,
        filing_date: &str,
    ) -> ApiResult<RuleApplicabilityResponse> {
        validate_iso_date(filing_date)?;
        let jurisdiction_id = normalize_jurisdiction_id(jurisdiction);
        let jurisdiction_ids = self.jurisdiction_scope_ids(&jurisdiction_id).await?;
        let state_id = state_id_for_jurisdiction_id(&jurisdiction_id);
        let court_id = court.and_then(normalize_court_id);
        let q = format!(
            "{AUTHORITY_RETURN_PREFIX}
             WHERE doc.jurisdiction_id IN $jurisdiction_ids
               AND date(doc.effective_start_date) <= date($filing_date)
               AND (doc.effective_end_date IS NULL OR date(doc.effective_end_date) >= date($filing_date))
             {AUTHORITY_RETURN_SUFFIX}"
        );
        let mut result = self
            .graph
            .execute(
                query(&q)
                    .param("jurisdiction_ids", jurisdiction_ids)
                    .param("filing_date", filing_date.to_string()),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut authorities = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            authorities.push(row_to_authority(row));
        }

        let utcr = self.utcr_for_date(filing_date, &state_id).await?;
        let slr_edition = authorities
            .iter()
            .filter(|doc| {
                doc.jurisdiction_id == jurisdiction_id
                    && doc.authority_kind == "SupplementaryLocalRuleEdition"
            })
            .max_by(|a, b| a.effective_start_date.cmp(&b.effective_start_date))
            .cloned();
        let statewide_orders = authorities
            .iter()
            .filter(|doc| {
                doc.jurisdiction_id == state_id && doc.authority_kind == "ChiefJusticeOrder"
            })
            .cloned()
            .collect::<Vec<_>>();
        let local_orders = authorities
            .iter()
            .filter(|doc| {
                doc.jurisdiction_id == jurisdiction_id
                    && doc.authority_kind == "PresidingJudgeOrder"
            })
            .cloned()
            .collect::<Vec<_>>();
        let out_of_cycle_amendments = authorities
            .iter()
            .filter(|doc| {
                doc.jurisdiction_id == jurisdiction_id
                    && doc.authority_kind == "OutOfCycleAmendment"
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut currentness_warnings = Vec::new();
        if slr_edition.is_none() && jurisdiction_id != "or:state" {
            currentness_warnings.push(format!(
                "No active SLR edition found for {jurisdiction_id} on {filing_date}."
            ));
        }
        if utcr.is_none() {
            currentness_warnings.push("No UTCR corpus edition was found in the graph.".to_string());
        }

        Ok(RuleApplicabilityResponse {
            jurisdiction_id,
            jurisdiction: jurisdiction.to_string(),
            court: court.map(str::to_string),
            court_id,
            work_product_type: work_product_type.to_string(),
            filing_date: filing_date.to_string(),
            utcr,
            slr_edition,
            statewide_orders,
            local_orders,
            out_of_cycle_amendments,
            currentness_warnings,
        })
    }

    pub async fn order(&self, authority_document_id: &str) -> ApiResult<RuleAuthoritySummary> {
        let q = format!(
            "{AUTHORITY_RETURN_PREFIX}
             WHERE doc.authority_document_id = $authority_document_id
             {AUTHORITY_RETURN_SUFFIX}
             LIMIT 1"
        );
        let rows = self
            .authority_query_param(
                &q,
                "authority_document_id",
                authority_document_id.to_string(),
            )
            .await?;
        rows.into_iter().next().ok_or_else(|| {
            ApiError::NotFound(format!(
                "Rule authority document {authority_document_id} was not found"
            ))
        })
    }

    pub async fn slr_edition(
        &self,
        jurisdiction: &str,
        year: i64,
    ) -> ApiResult<SupplementaryLocalRuleEditionResponse> {
        let jurisdiction_id = normalize_jurisdiction_id(jurisdiction);
        let q = format!(
            "{AUTHORITY_RETURN_PREFIX}
             WHERE doc:SupplementaryLocalRuleEdition
               AND doc.jurisdiction_id = $jurisdiction_id
               AND doc.edition_year = $year
             {AUTHORITY_RETURN_SUFFIX}
             LIMIT 1"
        );
        let mut result = self
            .graph
            .execute(
                query(&q)
                    .param("jurisdiction_id", jurisdiction_id.clone())
                    .param("year", year),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;
        if let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            Ok(SupplementaryLocalRuleEditionResponse {
                jurisdiction_id,
                year,
                edition: row_to_authority(row),
            })
        } else {
            Err(ApiError::NotFound(format!(
                "SLR edition {jurisdiction_id}/{year} was not found"
            )))
        }
    }

    async fn registry_sources(&self) -> ApiResult<Vec<CourtRulesRegistrySourceSummary>> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (src:CourtRulesRegistrySource)
                 RETURN src.registry_source_id AS registry_source_id,
                        src.source_type AS source_type,
                        src.jurisdiction AS jurisdiction,
                        src.jurisdiction_id AS jurisdiction_id,
                        src.source_url AS source_url,
                        src.snapshot_date AS snapshot_date,
                        coalesce(src.contains_current_future, false) AS contains_current_future,
                        coalesce(src.contains_prior, false) AS contains_prior
                 ORDER BY src.jurisdiction_id, src.snapshot_date DESC",
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rows.push(CourtRulesRegistrySourceSummary {
                registry_source_id: row.get("registry_source_id").unwrap_or_default(),
                source_type: row.get("source_type").unwrap_or_default(),
                jurisdiction: row.get("jurisdiction").unwrap_or_default(),
                jurisdiction_id: row.get("jurisdiction_id").unwrap_or_default(),
                source_url: row.get("source_url").unwrap_or_default(),
                snapshot_date: row.get("snapshot_date").unwrap_or_default(),
                contains_current_future: row.get("contains_current_future").unwrap_or(false),
                contains_prior: row.get("contains_prior").unwrap_or(false),
            });
        }
        Ok(rows)
    }

    async fn authority_query(&self, q: &str) -> ApiResult<Vec<RuleAuthoritySummary>> {
        let mut result = self
            .graph
            .execute(query(q))
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rows.push(row_to_authority(row));
        }
        Ok(rows)
    }

    async fn authority_query_param(
        &self,
        q: &str,
        name: &str,
        value: String,
    ) -> ApiResult<Vec<RuleAuthoritySummary>> {
        let mut result = self
            .graph
            .execute(query(q).param(name, value))
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rows.push(row_to_authority(row));
        }
        Ok(rows)
    }

    async fn authority_query_params(
        &self,
        q: &str,
        params: Vec<(&str, Vec<String>)>,
    ) -> ApiResult<Vec<RuleAuthoritySummary>> {
        let mut cypher = query(q);
        for (name, value) in params {
            cypher = cypher.param(name, value);
        }
        let mut result = self
            .graph
            .execute(cypher)
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rows.push(row_to_authority(row));
        }
        Ok(rows)
    }

    async fn jurisdiction_scope_ids(&self, jurisdiction_id: &str) -> ApiResult<Vec<String>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (target:Jurisdiction {jurisdiction_id: $jurisdiction_id})
                     MATCH (target)-[:PART_OF*0..]->(scope:Jurisdiction)
                     RETURN collect(DISTINCT scope.jurisdiction_id) AS jurisdiction_ids",
                )
                .param("jurisdiction_id", jurisdiction_id.to_string()),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut ids = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .and_then(|row| row.get::<Vec<String>>("jurisdiction_ids").ok())
            .unwrap_or_default();
        ids.push(jurisdiction_id.to_string());
        let state_id = state_id_for_jurisdiction_id(jurisdiction_id);
        if jurisdiction_id != state_id {
            ids.push(state_id);
        }
        ids.push("us".to_string());
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    async fn utcr_for_date(
        &self,
        filing_date: &str,
        state_id: &str,
    ) -> ApiResult<Option<ApplicableUtcrEdition>> {
        let corpus_id = if state_id == "or:state" {
            "or:utcr".to_string()
        } else {
            format!("{}:trial_court_rules", state_id.trim_end_matches(":state"))
        };
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (edition:CorpusEdition {corpus_id: $corpus_id})
                     WHERE edition.effective_date IS NULL OR date(edition.effective_date) <= date($filing_date)
                     RETURN edition.edition_id AS edition_id,
                            edition.corpus_id AS corpus_id,
                            edition.edition_year AS edition_year,
                            edition.effective_date AS effective_date,
                            edition.source_label AS source_label
                     ORDER BY coalesce(edition.effective_date, '') DESC, edition.edition_year DESC
                     LIMIT 1",
                )
                .param("filing_date", filing_date.to_string())
                .param("corpus_id", corpus_id),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;
        Ok(result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .map(|row| ApplicableUtcrEdition {
                edition_id: row
                    .get("edition_id")
                    .unwrap_or_else(|_| "or:utcr@2025".to_string()),
                corpus_id: row
                    .get("corpus_id")
                    .unwrap_or_else(|_| "or:utcr".to_string()),
                edition_year: row.get::<i64>("edition_year").unwrap_or(2025),
                effective_date: row.get("effective_date").ok(),
                source_label: row.get("source_label").ok(),
            }))
    }
}

const AUTHORITY_RETURN_PREFIX: &str = "
    MATCH (doc:RuleAuthorityDocument)
    OPTIONAL MATCH (doc)-[:HAS_TOPIC]->(topic:RuleTopic)
    WITH doc, collect(DISTINCT topic.name) AS topics
";

const AUTHORITY_RETURN_SUFFIX: &str = "
    RETURN doc.authority_document_id AS authority_document_id,
           doc.title AS title,
           doc.jurisdiction_id AS jurisdiction_id,
           doc.jurisdiction AS jurisdiction,
           doc.subcategory AS subcategory,
           doc.authority_kind AS authority_kind,
           doc.authority_identifier AS authority_identifier,
           doc.effective_start_date AS effective_start_date,
           doc.effective_end_date AS effective_end_date,
           doc.publication_bucket AS publication_bucket,
           doc.date_status AS date_status,
           coalesce(doc.status_flags, []) AS status_flags,
           topics AS topics,
           doc.source_url AS source_url
    ORDER BY doc.jurisdiction_id, doc.subcategory, doc.effective_start_date, doc.title
";

fn row_to_authority(row: Row) -> RuleAuthoritySummary {
    RuleAuthoritySummary {
        authority_document_id: row.get("authority_document_id").unwrap_or_default(),
        title: row.get("title").unwrap_or_default(),
        jurisdiction_id: row.get("jurisdiction_id").unwrap_or_default(),
        jurisdiction: row.get("jurisdiction").unwrap_or_default(),
        subcategory: row.get("subcategory").unwrap_or_default(),
        authority_kind: row.get("authority_kind").unwrap_or_default(),
        authority_identifier: row.get("authority_identifier").ok(),
        effective_start_date: row.get("effective_start_date").unwrap_or_default(),
        effective_end_date: row.get("effective_end_date").ok(),
        publication_bucket: row.get("publication_bucket").unwrap_or_default(),
        date_status: row.get("date_status").unwrap_or_default(),
        status_flags: row.get("status_flags").unwrap_or_default(),
        topics: row.get("topics").unwrap_or_default(),
        source_url: row.get("source_url").ok(),
    }
}

fn validate_iso_date(value: &str) -> ApiResult<()> {
    if ISO_DATE_RE.is_match(value) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Expected date in YYYY-MM-DD format, got {value}"
        )))
    }
}

fn normalize_jurisdiction_id(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "statewide" | "oregon" | "or:state" => "or:state".to_string(),
        other if other.starts_with("or:") => other.to_string(),
        other if other.ends_with(" county") => {
            format!("or:{}", slug(other.trim_end_matches(" county")))
        }
        other => format!("or:{}", slug(other)),
    }
}

fn normalize_court_id(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => None,
        other if other.starts_with("or:") => Some(other.to_string()),
        other if other.ends_with(" circuit court") => {
            let county = other
                .trim_end_matches(" circuit court")
                .trim_end_matches(" county")
                .trim();
            Some(format!("or:{}:circuit_court", slug(county)))
        }
        other if other.ends_with(" county") => Some(format!(
            "or:{}:circuit_court",
            slug(other.trim_end_matches(" county"))
        )),
        other => Some(format!("or:{}:circuit_court", slug(other))),
    }
}

fn state_id_for_jurisdiction_id(jurisdiction_id: &str) -> String {
    if jurisdiction_id == "us" {
        return "us".to_string();
    }
    let prefix = jurisdiction_id.split(':').next().unwrap_or("or");
    format!("{prefix}:state")
}

fn slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
