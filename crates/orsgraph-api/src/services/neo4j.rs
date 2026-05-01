use crate::error::{ApiError, ApiResult};
use crate::models::api::*;
use crate::models::search::{SearchResult as SearchResultModel, *};
use neo4rs::{query, Graph, Row};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Neo4jService {
    graph: Arc<Graph>,
}

impl Neo4jService {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    pub async fn run_rows(&self, q: neo4rs::Query) -> ApiResult<Vec<Row>> {
        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(ApiError::Neo4jConnection)?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rows.push(row);
        }
        Ok(rows)
    }

    pub async fn ensure_indexes(&self) -> ApiResult<()> {
        let indexes = [
            "CREATE INDEX legal_identity_citation IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.citation)",
            "CREATE INDEX legal_identity_authority_family IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.authority_family)",
            "CREATE INDEX legal_identity_canonical IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.canonical_id)",
            "CREATE INDEX legal_version_id IF NOT EXISTS FOR (n:LegalTextVersion) ON (n.version_id)",
            "CREATE INDEX legal_version_canonical IF NOT EXISTS FOR (n:LegalTextVersion) ON (n.canonical_id)",
            "CREATE INDEX provision_display_citation IF NOT EXISTS FOR (n:Provision) ON (n.display_citation)",
            "CREATE INDEX provision_authority_family IF NOT EXISTS FOR (n:Provision) ON (n.authority_family)",
            "CREATE INDEX provision_id IF NOT EXISTS FOR (n:Provision) ON (n.provision_id)",
            "CREATE INDEX provision_version_id IF NOT EXISTS FOR (n:Provision) ON (n.version_id)",
            "CREATE INDEX provision_canonical_id IF NOT EXISTS FOR (n:Provision) ON (n.canonical_id)",
            "CREATE INDEX retrieval_chunk_id IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.chunk_id)",
            "CREATE INDEX legal_semantic_id IF NOT EXISTS FOR (n:LegalSemanticNode) ON (n.semantic_id)",
            "CREATE INDEX citation_mention_source_provision IF NOT EXISTS FOR (n:CitationMention) ON (n.source_provision_id)",
            "CREATE INDEX legal_semantic_source_provision IF NOT EXISTS FOR (n:LegalSemanticNode) ON (n.source_provision_id)",
            "CREATE INDEX obligation_id IF NOT EXISTS FOR (n:Obligation) ON (n.obligation_id)",
            "CREATE INDEX exception_id IF NOT EXISTS FOR (n:Exception) ON (n.exception_id)",
            "CREATE INDEX deadline_id IF NOT EXISTS FOR (n:Deadline) ON (n.deadline_id)",
            "CREATE INDEX penalty_id IF NOT EXISTS FOR (n:Penalty) ON (n.penalty_id)",
            "CREATE INDEX remedy_id IF NOT EXISTS FOR (n:Remedy) ON (n.remedy_id)",
            "CREATE INDEX required_notice_id IF NOT EXISTS FOR (n:RequiredNotice) ON (n.required_notice_id)",
            "CREATE INDEX form_text_id IF NOT EXISTS FOR (n:FormText) ON (n.form_text_id)",
            "CREATE INDEX procedural_requirement_id IF NOT EXISTS FOR (n:ProceduralRequirement) ON (n.requirement_id)",
            "CREATE INDEX definition_source_provision IF NOT EXISTS FOR (n:Definition) ON (n.source_provision_id)",
            "CREATE INDEX definition_id IF NOT EXISTS FOR (n:Definition) ON (n.definition_id)",
            "CREATE INDEX defined_term_id IF NOT EXISTS FOR (n:DefinedTerm) ON (n.defined_term_id)",
            "CREATE INDEX source_note_version_id IF NOT EXISTS FOR (n:SourceNote) ON (n.version_id)",
            "CREATE INDEX source_note_provision_id IF NOT EXISTS FOR (n:SourceNote) ON (n.provision_id)",
            "CREATE INDEX source_note_canonical_id IF NOT EXISTS FOR (n:SourceNote) ON (n.canonical_id)",
            "CREATE INDEX source_note_id IF NOT EXISTS FOR (n:SourceNote) ON (n.source_note_id)",
            "CREATE INDEX chapter_version_id IF NOT EXISTS FOR (n:ChapterVersion) ON (n.chapter_id)",
            "CREATE INDEX sidebar_saved_search_scope IF NOT EXISTS FOR (n:SavedSearch) ON (n.scope)",
            "CREATE INDEX sidebar_saved_search_id IF NOT EXISTS FOR (n:SavedSearch) ON (n.saved_search_id)",
            "CREATE INDEX sidebar_saved_statute_scope IF NOT EXISTS FOR (n:SavedStatute) ON (n.scope)",
            "CREATE INDEX sidebar_recent_statute_scope IF NOT EXISTS FOR (n:RecentStatute) ON (n.scope)",
            "CREATE INDEX status_event_canonical_id IF NOT EXISTS FOR (n:StatusEvent) ON (n.canonical_id)",
            "CREATE INDEX status_event_version_id IF NOT EXISTS FOR (n:StatusEvent) ON (n.version_id)",
            "CREATE INDEX status_event_id IF NOT EXISTS FOR (n:StatusEvent) ON (n.status_event_id)",
            "CREATE INDEX temporal_effect_canonical_id IF NOT EXISTS FOR (n:TemporalEffect) ON (n.canonical_id)",
            "CREATE INDEX temporal_effect_version_id IF NOT EXISTS FOR (n:TemporalEffect) ON (n.version_id)",
            "CREATE INDEX temporal_effect_source_provision IF NOT EXISTS FOR (n:TemporalEffect) ON (n.source_provision_id)",
            "CREATE INDEX temporal_effect_id IF NOT EXISTS FOR (n:TemporalEffect) ON (n.temporal_effect_id)",
            "CREATE INDEX session_law_id IF NOT EXISTS FOR (n:SessionLaw) ON (n.session_law_id)",
            "CREATE INDEX amendment_id IF NOT EXISTS FOR (n:Amendment) ON (n.amendment_id)",
            "CREATE INDEX lineage_event_id IF NOT EXISTS FOR (n:LineageEvent) ON (n.lineage_event_id)",
            "CREATE INDEX tax_rule_id IF NOT EXISTS FOR (n:TaxRule) ON (n.tax_rule_id)",
            "CREATE INDEX money_amount_id IF NOT EXISTS FOR (n:MoneyAmount) ON (n.money_amount_id)",
            "CREATE INDEX rate_limit_id IF NOT EXISTS FOR (n:RateLimit) ON (n.rate_limit_id)",
            "CREATE INDEX legal_actor_id IF NOT EXISTS FOR (n:LegalActor) ON (n.actor_id)",
            "CREATE INDEX legal_action_id IF NOT EXISTS FOR (n:LegalAction) ON (n.action_id)",
            "CREATE INDEX rule_authority_document_id IF NOT EXISTS FOR (n:RuleAuthorityDocument) ON (n.authority_document_id)",
            "CREATE INDEX rule_authority_document_applicability IF NOT EXISTS FOR (n:RuleAuthorityDocument) ON (n.jurisdiction_id, n.effective_start_date, n.effective_end_date)",
            "CREATE INDEX rule_authority_document_kind IF NOT EXISTS FOR (n:RuleAuthorityDocument) ON (n.authority_kind, n.date_status)",
            "CREATE INDEX rule_publication_entry_jurisdiction IF NOT EXISTS FOR (n:RulePublicationEntry) ON (n.jurisdiction_id, n.publication_bucket)",
            "CREATE INDEX court_rules_registry_source_id IF NOT EXISTS FOR (n:CourtRulesRegistrySource) ON (n.registry_source_id)",
            "CREATE INDEX court_rules_registry_snapshot_id IF NOT EXISTS FOR (n:CourtRulesRegistrySnapshot) ON (n.registry_snapshot_id)",
            "CREATE FULLTEXT INDEX statute_fulltext IF NOT EXISTS FOR (n:LegalTextIdentity|LegalTextVersion) ON EACH [n.citation, n.title, n.text]",
            "CREATE FULLTEXT INDEX provision_fulltext IF NOT EXISTS FOR (n:Provision) ON EACH [n.display_citation, n.text, n.normalized_text]",
            "CREATE FULLTEXT INDEX definition_fulltext IF NOT EXISTS FOR (n:Definition|DefinedTerm) ON EACH [n.term, n.normalized_term, n.definition_text]",
            "CREATE FULLTEXT INDEX semantic_fulltext IF NOT EXISTS FOR (n:LegalSemanticNode|ProceduralRequirement|Obligation|Exception|Deadline|Penalty|Remedy|RequiredNotice|FormText) ON EACH [n.text, n.normalized_text, n.actor_text, n.action_text, n.object_text, n.trigger_event, n.summary]",
            "CREATE FULLTEXT INDEX history_fulltext IF NOT EXISTS FOR (n:SourceNote|StatusEvent|TemporalEffect|SessionLaw|Amendment|LineageEvent) ON EACH [n.text, n.normalized_text, n.status_text, n.trigger_text, n.citation, n.raw_text]",
            "CREATE FULLTEXT INDEX chunk_fulltext IF NOT EXISTS FOR (n:RetrievalChunk) ON EACH [n.text, n.breadcrumb, n.citation]",
            "CREATE FULLTEXT INDEX actor_action_fulltext IF NOT EXISTS FOR (n:LegalActor|LegalAction) ON EACH [n.actor_text, n.normalized_actor, n.verb, n.object_text, n.normalized_action]",
            "CREATE FULLTEXT INDEX specialized_legal_fulltext IF NOT EXISTS FOR (n:TaxRule|MoneyAmount|RateLimit|LegalAction|LegalActor) ON EACH [n.text, n.normalized_text, n.actor_text, n.normalized_actor, n.action_text, n.normalized_action, n.verb, n.object_text, n.tax_type, n.rate_type, n.amount_type]",
            "CREATE VECTOR INDEX retrieval_chunk_embedding_1024 IF NOT EXISTS FOR (n:RetrievalChunk) ON n.embedding OPTIONS {indexConfig: {`vector.dimensions`: 1024, `vector.similarity_function`: 'cosine'}}",
        ];

        for idx in indexes {
            let mut result = self
                .graph
                .execute(query(idx))
                .await
                .map_err(ApiError::Neo4jConnection)?;
            let _ = result.next().await;
        }

        Ok(())
    }

    pub async fn vector_index_exists(&self, index_name: &str) -> ApiResult<bool> {
        let mut result = self
            .graph
            .execute(
                query(
                    "SHOW INDEXES
                     YIELD name, type, state
                     WHERE name = $index_name AND type = 'VECTOR' AND state = 'ONLINE'
                     RETURN count(*) as count",
                )
                .param("index_name", index_name),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        Ok(result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .and_then(|row| row.get::<i64>("count").ok())
            .unwrap_or(0)
            > 0)
    }

    pub async fn health_check(&self) -> ApiResult<bool> {
        let mut result = self
            .graph
            .execute(query("RETURN 1 as test"))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        Ok(result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .is_some())
    }

    pub async fn search_exact_statute(
        &self,
        citation: &str,
        authority_family: Option<&str>,
    ) -> ApiResult<Option<SearchResultModel>> {
        let citation_upper = citation.to_ascii_uppercase();
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (n:LegalTextIdentity)
                     WHERE (n.citation = $c OR n.citation = $c_upper OR n.canonical_id = $c)
                       AND ($authority_family = '' OR coalesce(n.authority_family, 'ORS') = $authority_family)
                     RETURN n.canonical_id as id, n.citation as citation, n.title as title,
                            n.chapter as chapter, n.status as status,
                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule' ELSE 'statute' END as kind,
                            coalesce(n.text, n.title) as text,
                            n.authority_family as authority_family,
                            n.authority_type as authority_type,
                            n.corpus_id as corpus_id
                     LIMIT 1",
                )
                .param("c", citation)
                .param("c_upper", citation_upper)
                .param("authority_family", authority_family.unwrap_or_default()),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .map(|row| self.row_to_search_result(row, 4.0))
            .transpose()
    }

    pub async fn search_exact_provision(
        &self,
        citation: &str,
        authority_family: Option<&str>,
    ) -> ApiResult<Option<SearchResultModel>> {
        let citation_upper = citation.to_ascii_uppercase();
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (n:Provision)
                     WHERE (n.display_citation = $c OR n.display_citation = $c_upper)
                       AND ($authority_family = '' OR coalesce(n.authority_family, 'ORS') = $authority_family)
                     RETURN n.provision_id as id, n.display_citation as citation, null as title,
                            n.chapter as chapter, n.status as status,
                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule_provision' ELSE 'provision' END as kind,
                            n.text as text,
                            n.authority_family as authority_family,
                            n.authority_type as authority_type,
                            n.corpus_id as corpus_id
                     LIMIT 1",
                )
                .param("c", citation)
                .param("c_upper", citation_upper)
                .param("authority_family", authority_family.unwrap_or_default()),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .map(|row| self.row_to_search_result(row, 4.0))
            .transpose()
    }

    pub async fn get_stats(&self) -> ApiResult<StatsResponse> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (n) RETURN count(n) as nodes
                 UNION ALL
                 MATCH ()-[r]->() RETURN count(r) as relationships
                 UNION ALL
                 MATCH (n:SourceDocument) RETURN count(n) as chapters
                 UNION ALL
                 MATCH (n:LegalTextIdentity) RETURN count(n) as sections
                 UNION ALL
                 MATCH (n:Provision) RETURN count(n) as provisions
                 UNION ALL
                 MATCH (n:RetrievalChunk) RETURN count(n) as chunks
                 UNION ALL
                 MATCH (n:CitationMention) RETURN count(n) as citations
                 UNION ALL
                 MATCH ()-[r:CITES]->() RETURN count(r) as cites_edges
                 UNION ALL
                 MATCH (n:LegalSemanticNode) RETURN count(n) as semantic_nodes",
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut counts = vec![0u64; 9];
        let mut index = 0;

        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            if index < counts.len() {
                let value = row
                    .get("test")
                    .or_else(|_| row.get("nodes"))
                    .or_else(|_| row.get("relationships"))
                    .ok()
                    .and_then(|v: i64| Some(v as u64))
                    .unwrap_or(0);
                counts[index] = value;
                index += 1;
            }
        }

        Ok(StatsResponse {
            nodes: counts[0],
            relationships: counts[1],
            chapters: counts[2],
            sections: counts[3],
            provisions: counts[4],
            chunks: counts[5],
            citations: counts[6],
            cites_edges: counts[7],
            semantic_nodes: counts[8],
            last_seeded_at: None,
        })
    }

    pub async fn search_exact(
        &self,
        citation: &str,
        authority_family: Option<&str>,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let mut results = Vec::new();
        let citation_upper = citation.to_ascii_uppercase();
        let authority_family = authority_family.unwrap_or_default();

        // Statute lookup
        let mut statute_res = self
            .graph
            .execute(
                query(
                    "MATCH (n:LegalTextIdentity)
		                   WHERE (n.citation = $c OR n.citation = $c_upper OR n.canonical_id = $c)
		                     AND ($authority_family = '' OR coalesce(n.authority_family, 'ORS') = $authority_family)
		                   RETURN n.canonical_id as id, n.citation as citation, n.title as title,
		                          n.chapter as chapter, n.status as status,
		                          CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule' ELSE 'statute' END as kind,
		                          n.title as text,
		                          n.authority_family as authority_family,
		                          n.authority_type as authority_type,
		                          n.corpus_id as corpus_id",
                )
                .param("c", citation)
                .param("c_upper", citation_upper.clone())
                .param("authority_family", authority_family),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        while let Some(row) = statute_res
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
        {
            results.push(self.row_to_search_result(row, 4.0)?);
        }

        // Provision lookup
        let mut prov_res = self
            .graph
            .execute(
                query(
                    "MATCH (n:Provision)
		                   WHERE (n.display_citation = $c OR n.display_citation = $c_upper)
		                     AND ($authority_family = '' OR coalesce(n.authority_family, 'ORS') = $authority_family)
		                   RETURN n.provision_id as id, n.display_citation as citation, null as title,
		                          n.chapter as chapter, n.status as status,
		                          CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule_provision' ELSE 'provision' END as kind,
		                          n.text as text,
		                          n.authority_family as authority_family,
		                          n.authority_type as authority_type,
		                          n.corpus_id as corpus_id",
                )
                .param("c", citation)
                .param("c_upper", citation_upper)
                .param("authority_family", authority_family),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        while let Some(row) = prov_res.next().await.map_err(ApiError::Neo4jConnection)? {
            results.push(self.row_to_search_result(row, 4.0)?);
        }

        // Chapter lookup
        let chapter_number = citation
            .strip_prefix("Chapter ")
            .or_else(|| citation.strip_prefix("chapter "))
            .or_else(|| citation.strip_prefix("UTCR Chapter "))
            .or_else(|| citation.strip_prefix("utcr chapter "))
            .or_else(|| citation.strip_prefix("ORS Chapter "))
            .or_else(|| citation.strip_prefix("ors chapter "));
        if let Some(chapter) = chapter_number {
            let mut chapter_res = self
                .graph
                .execute(
                    query(
                        "MATCH (n:ChapterVersion)
                         WHERE (n.chapter = $chapter OR n.chapter_number = $chapter)
                           AND ($authority_family = '' OR coalesce(n.authority_family, 'ORS') = $authority_family)
                         RETURN n.chapter_id as id, 'Chapter ' + n.chapter as citation, n.title as title,
                                n.chapter as chapter, null as status,
                                CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule_chapter' ELSE 'chapter' END as kind,
                                coalesce(n.title, n.summary, n.chapter) as text,
                                n.authority_family as authority_family,
                                n.authority_type as authority_type,
                                n.corpus_id as corpus_id
                         LIMIT 5",
                    )
                    .param("chapter", chapter)
                    .param("authority_family", authority_family),
                )
                .await
                .map_err(ApiError::Neo4jConnection)?;

            while let Some(row) = chapter_res
                .next()
                .await
                .map_err(ApiError::Neo4jConnection)?
            {
                results.push(self.row_to_search_result(row, 3.5)?);
            }
        }

        Ok(results)
    }

    pub async fn search_citation_range(
        &self,
        range: &QueryCitationRange,
        limit: u32,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let upper_end = format!("{}~", range.end);
        let mut result = self
            .graph
            .execute(
                query(
                    "CALL {
                       MATCH (n:LegalTextIdentity)
                       WHERE n.chapter = $chapter
                         AND coalesce(n.authority_family, 'ORS') = $authority_family
                         AND n.citation >= $start
                         AND n.citation <= $upper_end
                       RETURN n.canonical_id as id, n.citation as citation, n.title as title,
                              n.chapter as chapter, n.status as status,
                              CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule' ELSE 'statute' END as kind,
                              coalesce(n.text, n.title) as text,
                              n.authority_family as authority_family,
                              n.authority_type as authority_type,
                              n.corpus_id as corpus_id
                       UNION
                       MATCH (n:Provision)
                       WHERE n.chapter = $chapter
                         AND coalesce(n.authority_family, 'ORS') = $authority_family
                         AND n.display_citation >= $start
                         AND n.display_citation <= $upper_end
                       RETURN n.provision_id as id, n.display_citation as citation, null as title,
                              n.chapter as chapter, n.status as status,
                              CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule_provision' ELSE 'provision' END as kind,
                              n.text as text,
                              n.authority_family as authority_family,
                              n.authority_type as authority_type,
                              n.corpus_id as corpus_id
                     }
                     RETURN id, citation, title, chapter, status, kind, text, authority_family, authority_type, corpus_id
                     ORDER BY citation
                     LIMIT $limit",
                )
                .param("chapter", range.chapter.clone())
                .param("authority_family", range.authority_family.clone())
                .param("start", range.start.clone())
                .param("upper_end", upper_end)
                .param("limit", limit.max(1) as i64),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut results = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            results.push(self.row_to_search_result(row, 3.0)?);
        }
        Ok(results)
    }

    pub async fn search_fulltext(
        &self,
        q: &str,
        filters: &SearchRetrievalFilters,
        limit: u32,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let mut results = Vec::new();
        let sanitized_query = sanitize_fulltext_query(q);
        if sanitized_query.is_empty() {
            return Ok(results);
        }

        let limit = limit.max(1) as usize;
        let semantic_kind = "CASE
                   WHEN node:Definition OR toLower(coalesce(node.semantic_type, '')) = 'definition' THEN 'definition'
                   WHEN node:Obligation OR toLower(coalesce(node.semantic_type, '')) = 'obligation' THEN 'obligation'
                   WHEN node:Exception OR toLower(coalesce(node.semantic_type, '')) = 'exception' THEN 'exception'
                   WHEN node:Deadline OR toLower(coalesce(node.semantic_type, '')) = 'deadline' THEN 'deadline'
                   WHEN node:Penalty OR toLower(coalesce(node.semantic_type, '')) = 'penalty' THEN 'penalty'
                   WHEN node:Remedy OR toLower(coalesce(node.semantic_type, '')) = 'remedy' THEN 'remedy'
                   WHEN node:RequiredNotice OR toLower(coalesce(node.semantic_type, '')) IN ['requirednotice', 'required_notice', 'notice'] THEN 'requirednotice'
                   WHEN node:ProceduralRequirement OR toLower(coalesce(node.semantic_type, '')) = 'proceduralrequirement' THEN 'proceduralrequirement'
                   ELSE toLower(labels(node)[0])
                 END";
        let history_kind = "CASE
                   WHEN node:SourceNote THEN 'sourcenote'
                   WHEN node:StatusEvent THEN 'statusevent'
                   WHEN node:TemporalEffect THEN 'temporaleffect'
                   WHEN node:SessionLaw THEN 'sessionlaw'
                   WHEN node:Amendment THEN 'amendment'
                   WHEN node:LineageEvent THEN 'lineageevent'
                   ELSE toLower(labels(node)[0])
                 END";
        let specialized_kind = "CASE
                   WHEN node:TaxRule THEN 'taxrule'
                   WHEN node:MoneyAmount THEN 'moneyamount'
                   WHEN node:RateLimit THEN 'ratelimit'
                   WHEN node:LegalActor THEN 'legalactor'
                   WHEN node:LegalAction THEN 'legalaction'
                   ELSE toLower(labels(node)[0])
                 END";
        let inferred_authority =
            "CASE WHEN coalesce(node.authority_family, '') <> '' THEN node.authority_family
                  WHEN coalesce(node.source_provision_id, node.citation, '') STARTS WITH 'UTCR ' THEN 'UTCR'
                  ELSE 'ORS'
             END";
        let search_id = search_node_id_expr("node");

        let queries: Vec<(&str, String, f32)> = match filters
            .result_type
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("statute") => vec![(
                "statute_fulltext",
                "MATCH (node)
                 OPTIONAL MATCH (node)-[:HAS_VERSION]->(v:LegalTextVersion)
                 WITH coalesce(node, v) as n, score
                 RETURN n.canonical_id as id, n.citation as citation, n.title as title,
                        n.chapter as chapter, n.status as status, 'statute' as kind, score,
                        coalesce(n.text, n.title) as text,
                        coalesce(n.authority_family, 'ORS') as authority_family,
                        n.authority_type as authority_type,
                        n.corpus_id as corpus_id"
                    .to_string(),
                1.05,
            )],
            Some("provision") => vec![(
                "provision_fulltext",
                "MATCH (node)
                 RETURN node.provision_id as id, node.display_citation as citation, null as title,
                        node.chapter as chapter, node.status as status, 'provision' as kind, score,
                        node.text as text,
                        coalesce(node.authority_family, 'ORS') as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"
                    .to_string(),
                1.15,
            )],
            Some("definition") | Some("definedterm") => vec![(
                "definition_fulltext",
                format!("MATCH (node)
                 RETURN {search_id} as id, node.term as citation, null as title,
                        null as chapter, null as status, 'definition' as kind, score,
                        coalesce(node.definition_text, node.term) as text,
                        {inferred_authority} as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"),
                1.1,
            )],
            Some("semantic") | Some("obligation") | Some("exception") | Some("deadline")
            | Some("penalty") | Some("notice") | Some("requirednotice") | Some("remedy") => {
                vec![(
                    "semantic_fulltext",
                    format!("MATCH (node)
                     RETURN {search_id} as id, node.citation as citation, null as title,
                            node.chapter as chapter, null as status, {semantic_kind} as kind, score,
                            node.text as text,
                            {inferred_authority} as authority_family,
                            node.authority_type as authority_type,
                            node.corpus_id as corpus_id"),
                    1.0,
                )]
            }
            Some("history") | Some("sourcenote") | Some("source_note") | Some("temporaleffect")
            | Some("temporal_effect") | Some("sessionlaw") | Some("amendment") => vec![(
                "history_fulltext",
                format!("MATCH (node)
                 RETURN {search_id} as id, node.citation as citation, null as title,
                        null as chapter, null as status, {history_kind} as kind, score,
                        coalesce(node.text, node.raw_text) as text,
                        {inferred_authority} as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"),
                0.9,
            )],
            Some("chunk") => vec![(
                "chunk_fulltext",
                "MATCH (node)
                 RETURN node.chunk_id as id, node.citation as citation, null as title,
                        null as chapter, null as status, 'chunk' as kind, score,
                        node.text as text,
                        coalesce(node.authority_family, 'ORS') as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"
                    .to_string(),
                0.85,
            )],
            Some("actor") => vec![(
                "actor_action_fulltext",
                format!("MATCH (node)
                 RETURN {search_id} as id, null as citation, null as title,
                        null as chapter, null as status, {specialized_kind} as kind, score,
                        coalesce(node.actor_text, node.object_text) as text,
                        {inferred_authority} as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"),
                0.95,
            )],
            Some("taxrule") | Some("moneyamount") | Some("ratelimit") | Some("legalaction")
            | Some("legalactor") => vec![(
                "specialized_legal_fulltext",
                format!("MATCH (node)
                 RETURN {search_id} as id,
                        node.citation as citation, null as title, node.chapter as chapter, null as status,
                        {specialized_kind} as kind, score,
                        coalesce(node.text, node.normalized_text, node.actor_text, node.action_text, node.object_text, node.tax_type, node.rate_type, node.amount_type) as text,
                        {inferred_authority} as authority_family,
                        node.authority_type as authority_type,
                        node.corpus_id as corpus_id"),
                0.95,
            )],
            _ => vec![
                ("statute_fulltext", "MATCH (node) RETURN node.canonical_id as id, node.citation as citation, node.title as title, node.chapter as chapter, node.status as status, 'statute' as kind, score, coalesce(node.text, node.title) as text, coalesce(node.authority_family, 'ORS') as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id".to_string(), 1.05),
                ("provision_fulltext", "MATCH (node) RETURN node.provision_id as id, node.display_citation as citation, null as title, node.chapter as chapter, node.status as status, 'provision' as kind, score, node.text as text, coalesce(node.authority_family, 'ORS') as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id".to_string(), 1.15),
                ("definition_fulltext", format!("MATCH (node) RETURN {search_id} as id, node.term as citation, null as title, null as chapter, null as status, 'definition' as kind, score, coalesce(node.definition_text, node.term) as text, {inferred_authority} as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id"), 1.1),
                ("semantic_fulltext", format!("MATCH (node) RETURN {search_id} as id, node.citation as citation, null as title, node.chapter as chapter, null as status, {semantic_kind} as kind, score, node.text as text, {inferred_authority} as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id"), 1.0),
                ("specialized_legal_fulltext", format!("MATCH (node) RETURN {search_id} as id, node.citation as citation, null as title, node.chapter as chapter, null as status, {specialized_kind} as kind, score, coalesce(node.text, node.normalized_text, node.actor_text, node.action_text, node.object_text, node.tax_type, node.rate_type, node.amount_type) as text, {inferred_authority} as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id"), 0.95),
                ("history_fulltext", format!("MATCH (node) RETURN {search_id} as id, node.citation as citation, null as title, null as chapter, null as status, {history_kind} as kind, score, coalesce(node.text, node.raw_text) as text, {inferred_authority} as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id"), 0.9),
                ("chunk_fulltext", "MATCH (node) RETURN node.chunk_id as id, node.citation as citation, null as title, node.chapter as chapter, null as status, 'chunk' as kind, score, node.text as text, coalesce(node.authority_family, 'ORS') as authority_family, node.authority_type as authority_type, node.corpus_id as corpus_id".to_string(), 0.85),
            ],
        };

        let per_index_limit = if queries.len() == 1 {
            limit
        } else {
            limit.clamp(10, 40)
        };

        for (index_name, return_clause, source_weight) in queries {
            let cypher = format!(
                "CALL {{
                   CALL db.index.fulltext.queryNodes($index, $q) YIELD node, score
                   {}
                 }}
                 WITH id, citation, title, chapter, status, kind, score, text, authority_family, authority_type, corpus_id
                 WHERE ($chapter = '' OR chapter = $chapter)
                   AND ($status = '' OR status = $status)
                   AND ($current_only = false OR status IS NULL OR status = 'active')
                   AND ($authority_family = '' OR toUpper(coalesce(authority_family, 'ORS')) = $authority_family)
                 RETURN id, citation, title, chapter, status, kind, score, text, authority_family, authority_type, corpus_id
                 ORDER BY score DESC
                 LIMIT $limit",
                return_clause
            );

            let mut res = self
                .graph
                .execute(
                    query(&cypher)
                        .param("index", index_name)
                        .param("q", sanitized_query.clone())
                        .param("limit", per_index_limit as i64)
                        .param("chapter", filters.chapter.clone().unwrap_or_default())
                        .param("status", filters.status.clone().unwrap_or_default())
                        .param(
                            "authority_family",
                            filters.authority_family.clone().unwrap_or_default(),
                        )
                        .param("current_only", filters.current_only),
                )
                .await
                .map_err(ApiError::Neo4jConnection)?;

            let mut rank = 0usize;
            while let Some(row) = res.next().await.map_err(ApiError::Neo4jConnection)? {
                rank += 1;
                let score = fulltext_rank_score(rank, source_weight);
                let mut result = self.row_to_search_result(row, score)?;
                result.fulltext_score = Some(score);
                result.rank_source = Some("keyword".to_string());
                result.score_breakdown = Some(ScoreBreakdown {
                    exact: None,
                    keyword: Some(score),
                    vector: None,
                    rerank: None,
                    graph: None,
                    authority: None,
                    expansion: None,
                    penalties: None,
                });
                results.push(result);
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        Ok(results)
    }

    pub async fn search_vector_chunks(
        &self,
        index_name: &str,
        embedding: Vec<f32>,
        top_k: usize,
        min_score: f32,
        limit: usize,
        filters: &SearchRetrievalFilters,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let index_name = safe_vector_index_name(index_name)?;
        let top_k = top_k.max(1);
        let limit = limit.max(1);
        let mut search_where = vec![
            "node.citation IS NOT NULL".to_string(),
            "node.embedding_policy IN ['embed_primary', 'embed_special']".to_string(),
            "(node.answer_policy IS NULL OR node.answer_policy IN ['authoritative_support', 'answerable', 'preferred', 'supporting'])".to_string(),
        ];
        let chunk_type = filters.vector_chunk_type();
        if filters.chapter.is_some() {
            search_where.push("node.chapter = $chapter".to_string());
        }
        if filters.authority_family.is_some() {
            search_where
                .push("coalesce(node.authority_family, 'ORS') = $authority_family".to_string());
        }
        if chunk_type.is_some() {
            search_where.push("node.chunk_type = $chunk_type".to_string());
        }

        let cypher = format!(
            "MATCH (node:RetrievalChunk)
               SEARCH node IN (
                 VECTOR INDEX {index_name}
                 FOR $embedding
                 WHERE {search_where}
                 LIMIT {top_k}
               ) SCORE AS score
             WHERE score >= $min_score
             MATCH (node)-[:DERIVED_FROM]->(source)
             OPTIONAL MATCH (source:Provision)-[:PART_OF_VERSION]->(v:LegalTextVersion)-[:VERSION_OF]->(id:LegalTextIdentity)
             OPTIONAL MATCH (source:LegalTextVersion)-[:VERSION_OF]->(id2:LegalTextIdentity)
             WITH node, source, score, coalesce(id, id2) AS identity, v,
                  coalesce(source.chapter, identity.chapter, node.chapter) AS chapter,
                  coalesce(source.status, identity.status, 'active') AS status,
                  coalesce(source.authority_family, identity.authority_family, node.authority_family, 'ORS') AS authority_family,
                  coalesce(source.authority_type, identity.authority_type, node.authority_type) AS authority_type,
                  coalesce(source.corpus_id, identity.corpus_id, node.corpus_id) AS corpus_id
             WHERE ($chapter = '' OR chapter = $chapter)
               AND ($status = '' OR status = $status)
               AND ($authority_family = '' OR authority_family = $authority_family)
               AND ($current_only = false OR status IS NULL OR status = 'active')
             RETURN
               coalesce(source.provision_id, identity.canonical_id, node.chunk_id) as id,
               CASE
                 WHEN source:Provision AND authority_family = 'UTCR' THEN 'court_rule_provision'
                 WHEN source:Provision THEN 'provision'
                 WHEN source:LegalTextVersion AND authority_family = 'UTCR' THEN 'court_rule'
                 WHEN source:LegalTextVersion THEN 'statute'
                 ELSE 'chunk'
               END as kind,
               coalesce(source.display_citation, identity.citation, node.citation) as citation,
               identity.title as title,
               chapter,
               status,
               coalesce(source.text, node.text) as text,
               node.chunk_id as chunk_id,
               source.provision_id as provision_id,
               coalesce(v.version_id, source.version_id, node.parent_version_id, node.source_version_id) as version_id,
               authority_family,
               authority_type,
               corpus_id,
               score
             ORDER BY score DESC
             LIMIT $limit",
            index_name = index_name,
            search_where = search_where.join(" AND "),
            top_k = top_k,
        );

        let mut query_builder = query(&cypher)
            .param("embedding", embedding)
            .param("min_score", min_score as f64)
            .param("limit", limit as i64)
            .param("chapter", filters.chapter.clone().unwrap_or_default())
            .param("status", filters.status.clone().unwrap_or_default())
            .param(
                "authority_family",
                filters.authority_family.clone().unwrap_or_default(),
            )
            .param("current_only", filters.current_only);
        if let Some(chunk_type) = chunk_type {
            query_builder = query_builder.param("chunk_type", chunk_type);
        }

        let mut result = self
            .graph
            .execute(query_builder)
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut results = Vec::new();
        let mut rank = 0usize;
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            rank += 1;
            let score = row.get::<f64>("score").unwrap_or(0.0) as f32;
            let rank_score = vector_rank_score(rank, score);
            let mut search_result = self.row_to_search_result(row, rank_score)?;
            search_result.vector_score = Some(score);
            search_result.rank_source = Some("vector".to_string());
            let mut source = search_result.source.unwrap_or(SourceInfo {
                source_document_id: None,
                provision_id: None,
                version_id: None,
                chunk_id: None,
                source_note_id: None,
            });
            if source.chunk_id.is_none() {
                source.chunk_id = Some(search_result.id.clone());
            }
            search_result.source = Some(source);
            search_result.score_breakdown = Some(ScoreBreakdown {
                exact: None,
                keyword: None,
                vector: Some(score),
                rerank: None,
                graph: None,
                authority: None,
                expansion: None,
                penalties: None,
            });
            results.push(search_result);
        }

        Ok(results)
    }

    pub async fn expand_search_results(
        &self,
        results: &mut [SearchResultModel],
    ) -> ApiResult<usize> {
        if results.is_empty() {
            return Ok(0);
        }

        let buckets = search_node_id_buckets(results);
        let expansion_query = format!(
            "{}
                     WITH id, head(collect(DISTINCT n)) AS n
                     WHERE n IS NOT NULL
                     OPTIONAL MATCH (n:RetrievalChunk)-[:DERIVED_FROM]->(chunk_source)
                     WITH id, n, chunk_source, coalesce(n.source_provision_id, chunk_source.source_provision_id) AS support_provision_id
                     OPTIONAL MATCH (support:Provision {{provision_id: support_provision_id}})
                     WITH id, n, coalesce(chunk_source, support, n) AS source
                     OPTIONAL MATCH (source:Provision)-[:PART_OF_VERSION]->(v:LegalTextVersion)-[:VERSION_OF]->(identity:LegalTextIdentity)
                     OPTIONAL MATCH (source:LegalTextVersion)-[:VERSION_OF]->(identity2:LegalTextIdentity)
                     OPTIONAL MATCH (source:LegalTextIdentity)<-[:VERSION_OF]-(identity_version:LegalTextVersion)
                     WITH id, n, source, coalesce(v, identity_version, source) AS version,
                          coalesce(identity, identity2, source) AS identity
                     OPTIONAL MATCH (source)-[:EXPRESSES]->(sem)
                     OPTIONAL MATCH (version)<-[:PART_OF_VERSION]-(version_provision:Provision)
                     WHERE source:LegalTextIdentity OR source:LegalTextVersion
                     OPTIONAL MATCH (version_provision)-[:EXPRESSES]->(version_sem)
                     OPTIONAL MATCH (source)-[:DEFINES]->(def)
                     OPTIONAL MATCH (source)-[:MENTIONS_CITATION]->(cm:CitationMention)
                     OPTIONAL MATCH (cm)-[:RESOLVES_TO]->(target:LegalTextIdentity)
                     OPTIONAL MATCH (source)-[:HAS_SOURCE_NOTE]->(source_note)
                     OPTIONAL MATCH (version)-[:HAS_SOURCE_NOTE]->(version_note)
                     OPTIONAL MATCH (source)-[:HAS_TEMPORAL_EFFECT]->(source_te)
                     OPTIONAL MATCH (version)-[:HAS_TEMPORAL_EFFECT]->(version_te)
                     OPTIONAL MATCH (source)-[:CITES]->(out)
                     OPTIONAL MATCH (in)-[:CITES]->(source)
                     WITH id, n, source, version, identity,
                          collect(DISTINCT coalesce(sem.semantic_type, labels(sem)[0]))
                            + collect(DISTINCT coalesce(version_sem.semantic_type, labels(version_sem)[0])) AS semantic_types,
                          count(DISTINCT sem) + count(DISTINCT version_sem) AS semantic_count,
                          count(DISTINCT def) AS definition_count,
                          count(DISTINCT target) AS citation_target_count,
                          count(DISTINCT source_note) + count(DISTINCT version_note) AS source_note_count,
                          count(DISTINCT source_te) + count(DISTINCT version_te) AS temporal_effect_count,
                          count(DISTINCT out) AS outbound_count,
                          count(DISTINCT in) AS inbound_count
                     RETURN id,
                            identity.canonical_id AS canonical_id,
                            version.version_id AS version_id,
                            source.provision_id AS provision_id,
                            CASE WHEN source:RetrievalChunk THEN source.chunk_id ELSE n.chunk_id END AS chunk_id,
                            semantic_types,
                            semantic_count,
                            definition_count,
                            citation_target_count,
                            source_note_count,
                            temporal_effect_count,
                            outbound_count,
                            inbound_count",
            search_node_resolver_cypher()
        );
        let mut rows = self
            .graph
            .execute(with_search_node_bucket_params(
                query(&expansion_query),
                &buckets,
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut map: std::collections::HashMap<String, GraphInfo> =
            std::collections::HashMap::new();
        let mut semantic_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut source_map: std::collections::HashMap<String, SourceInfo> =
            std::collections::HashMap::new();
        let mut graph_scores: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();
        let mut expanded = 0;

        while let Some(row) = rows.next().await.map_err(ApiError::Neo4jConnection)? {
            let id = row.get::<String>("id").unwrap_or_default();
            let semantic_count = row.get::<i64>("semantic_count").unwrap_or(0).max(0) as u64;
            let definition_count = row.get::<i64>("definition_count").unwrap_or(0).max(0) as u64;
            let citation_target_count =
                row.get::<i64>("citation_target_count").unwrap_or(0).max(0) as u64;
            let source_note_count = row.get::<i64>("source_note_count").unwrap_or(0).max(0) as u64;
            let temporal_effect_count =
                row.get::<i64>("temporal_effect_count").unwrap_or(0).max(0) as u64;
            let outbound_count = row.get::<i64>("outbound_count").unwrap_or(0).max(0) as u64;
            let inbound_count = row.get::<i64>("inbound_count").unwrap_or(0).max(0) as u64;
            let connected = semantic_count
                + definition_count
                + citation_target_count
                + source_note_count
                + temporal_effect_count;

            map.insert(
                id.clone(),
                GraphInfo {
                    canonical_id: row.get("canonical_id").ok(),
                    version_id: row.get("version_id").ok(),
                    provision_id: row.get("provision_id").ok(),
                    connected_node_count: Some(connected),
                    citation_count: Some(outbound_count + citation_target_count),
                    cited_by_count: Some(inbound_count),
                },
            );

            semantic_map.insert(
                id.clone(),
                row.get::<Vec<String>>("semantic_types")
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|v| !v.is_empty())
                    .collect(),
            );

            source_map.insert(
                id.clone(),
                SourceInfo {
                    source_document_id: None,
                    provision_id: row.get("provision_id").ok(),
                    version_id: row.get("version_id").ok(),
                    chunk_id: row.get("chunk_id").ok(),
                    source_note_id: None,
                },
            );

            let graph_score = (semantic_count as f32 * 0.12)
                + (definition_count as f32 * 0.15)
                + (citation_target_count as f32 * 0.08)
                + (source_note_count as f32 * 0.08)
                + (temporal_effect_count as f32 * 0.1)
                + ((inbound_count + outbound_count) as f32).log10().max(0.0) * 0.2;
            graph_scores.insert(id, graph_score.min(1.5));
            expanded += 1;
        }

        for result in results {
            if let Some(graph) = map.get(&result.id) {
                result.graph = Some(graph.clone());
            }
            if let Some(source) = source_map.get(&result.id) {
                result.source = Some(source.clone());
            }
            if let Some(semantic_types) = semantic_map.get(&result.id) {
                for semantic_type in semantic_types {
                    if !result.semantic_types.contains(semantic_type) {
                        result.semantic_types.push(semantic_type.clone());
                    }
                }
            }
            if let Some(graph_score) = graph_scores.get(&result.id) {
                result.graph_score = Some(*graph_score);
                result.score += *graph_score;
                match &mut result.score_breakdown {
                    Some(breakdown) => breakdown.graph = Some(*graph_score),
                    None => {
                        result.score_breakdown = Some(ScoreBreakdown {
                            exact: None,
                            keyword: result.fulltext_score,
                            vector: result.vector_score,
                            rerank: result.rerank_score,
                            graph: Some(*graph_score),
                            authority: None,
                            expansion: None,
                            penalties: None,
                        });
                    }
                }
            }
        }

        Ok(expanded)
    }

    pub async fn batch_fetch_graph_metadata(
        &self,
        ids: &[String],
    ) -> ApiResult<std::collections::HashMap<String, GraphInfo>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let buckets = SearchNodeIdBuckets::from_generic_ids(ids.iter().cloned());
        let metadata_query = format!(
            "{}
                   WITH id, head(collect(DISTINCT n)) AS n
                   WHERE n IS NOT NULL
                   OPTIONAL MATCH (n)-[:CITES]->(out)
                   OPTIONAL MATCH (in)-[:CITES]->(n)
                   OPTIONAL MATCH (n)-[:HAS_SEMANTIC_NODE|DEFINED_BY]-(s)
                   RETURN id, count(DISTINCT out) as outbound, count(DISTINCT in) as inbound, count(DISTINCT s) as semantic",
            search_node_resolver_cypher()
        );
        let mut result = self
            .graph
            .execute(with_search_node_bucket_params(
                query(&metadata_query),
                &buckets,
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut map = std::collections::HashMap::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            let id = row.get::<String>("id").unwrap_or_default();
            map.insert(
                id,
                GraphInfo {
                    canonical_id: None,
                    version_id: None,
                    provision_id: None,
                    connected_node_count: Some(row.get::<i64>("semantic").unwrap_or(0) as u64),
                    citation_count: Some(row.get::<i64>("outbound").unwrap_or(0) as u64),
                    cited_by_count: Some(row.get::<i64>("inbound").unwrap_or(0) as u64),
                },
            );
        }
        Ok(map)
    }

    pub fn aggregate_facets(&self, results: &[SearchResultModel]) -> SearchFacets {
        let mut kinds = std::collections::HashMap::new();
        let mut chapters = std::collections::HashMap::new();
        let mut statuses = std::collections::HashMap::new();
        let mut semantic_types = std::collections::HashMap::new();
        let mut source_backed_true = 0;
        let mut source_backed_false = 0;

        for res in results {
            *kinds.entry(res.kind.clone()).or_insert(0) += 1;
            if let Some(ch) = &res.chapter {
                *chapters.entry(ch.clone()).or_insert(0) += 1;
            }
            if let Some(st) = &res.status {
                *statuses.entry(st.clone()).or_insert(0) += 1;
            }
            for st in &res.semantic_types {
                *semantic_types.entry(st.clone()).or_insert(0) += 1;
            }
            if res.source_backed {
                source_backed_true += 1;
            } else {
                source_backed_false += 1;
            }
        }

        SearchFacets {
            kinds,
            chapters,
            statuses,
            semantic_types,
            source_backed: SourceBackedFacet {
                r#true: source_backed_true,
                r#false: source_backed_false,
            },
            qc_warnings: std::collections::HashMap::new(),
        }
    }

    fn row_to_search_result(&self, row: neo4rs::Row, score: f32) -> ApiResult<SearchResultModel> {
        let kind = row
            .get::<String>("kind")
            .unwrap_or_else(|_| "unknown".to_string())
            .to_lowercase();
        let id = row.get::<String>("id").unwrap_or_default();
        let citation = row
            .get::<String>("citation")
            .ok()
            .or_else(|| citation_from_canonical_id(&id));
        let title: Option<String> = row.get("title").ok();
        let authority_family = row.get::<String>("authority_family").ok().or_else(|| {
            citation
                .as_deref()
                .and_then(infer_authority_family_from_citation)
        });
        let authority_type = row.get::<String>("authority_type").ok().or_else(|| {
            authority_type_for_family(authority_family.as_deref()).map(ToString::to_string)
        });
        let corpus_id = row
            .get::<String>("corpus_id")
            .ok()
            .or_else(|| corpus_id_for_family(authority_family.as_deref()).map(ToString::to_string));

        // Basic snippet generation
        let text: Option<String> = row.get("text").ok();
        let snippet = match text {
            Some(t) => {
                if t.chars().count() > 280 {
                    format!("{}...", t.chars().take(277).collect::<String>())
                } else {
                    t
                }
            }
            None => title.clone().unwrap_or_default(),
        };

        let href = match (authority_family.as_deref(), kind.as_str()) {
            (Some("UTCR"), "court_rule" | "legaltextidentity" | "utcrrule") => {
                format!("/rules/utcr/{}", citation.as_deref().unwrap_or(&id))
            }
            (Some("UTCR"), "court_rule_provision" | "provision" | "utcrprovision") => format!(
                "/rules/utcr/{}?provision={}",
                citation.as_deref().unwrap_or(&id),
                id
            ),
            (_, "statute" | "legaltextidentity") => {
                format!("/statutes/{}", citation.as_deref().unwrap_or(&id))
            }
            (_, "provision") => format!(
                "/statutes/{}?provision={}",
                citation.as_deref().unwrap_or(&id),
                id
            ),
            _ => format!("/search?q={}", id),
        };
        let mut semantic_types = semantic_types_for_search_kind(&kind);
        seed_history_semantic_types(&mut semantic_types, &id, &kind, &snippet);

        Ok(SearchResultModel {
            id,
            kind,
            authority_family,
            authority_type,
            corpus_id,
            citation: citation.clone(),
            title,
            chapter: row.get("chapter").ok(),
            status: row.get("status").ok(),
            snippet,
            score,
            vector_score: None,
            fulltext_score: None,
            graph_score: None,
            rerank_score: None,
            pre_rerank_score: None,
            rank_source: None,
            score_breakdown: None,
            semantic_types,
            source_backed: true,
            qc_warnings: vec![],
            href,
            source: Some(SourceInfo {
                source_document_id: row.get("source_document_id").ok(),
                provision_id: row.get("provision_id").ok(),
                version_id: row.get("version_id").ok(),
                chunk_id: row.get("chunk_id").ok(),
                source_note_id: row.get("source_note_id").ok(),
            }),
            graph: None,
        })
    }

    pub async fn list_statutes(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
        q: Option<&str>,
        chapter: Option<&str>,
        status: Option<&str>,
    ) -> ApiResult<StatuteIndexResponse> {
        let limit = limit.unwrap_or(250).clamp(1, 1000);
        let offset = offset.unwrap_or(0);
        let q = normalized_filter(q);
        let status = normalized_filter(status);
        let end = offset.saturating_add(limit);
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE ($chapter IS NULL OR i.chapter = $chapter)
                       AND ($status IS NULL OR toLower(coalesce(i.status, 'active')) = $status)
                       AND (
                         $q IS NULL
                         OR toLower(coalesce(i.citation, '')) CONTAINS $q
                         OR toLower(coalesce(i.canonical_id, '')) CONTAINS $q
                         OR toLower(coalesce(i.title, '')) CONTAINS $q
                       )
                     WITH i
                     ORDER BY coalesce(i.chapter, ''), coalesce(i.citation, '')
                     WITH collect({
                       canonical_id: i.canonical_id,
                       citation: i.citation,
                       title: i.title,
                       chapter: coalesce(i.chapter, ''),
                       status: coalesce(i.status, 'active'),
                       edition_year: coalesce(i.edition_year, 2025)
                     }) AS matched
                     RETURN size(matched) AS total, matched[$offset..$end] AS items",
                )
                .param("q", q)
                .param("chapter", chapter.map(|value| value.to_string()))
                .param("status", status)
                .param("offset", offset as i64)
                .param("end", end as i64),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound("No statutes found".to_string()))?;

        let items_json: Vec<serde_json::Value> = row.get("items").ok().unwrap_or_default();
        let items = items_json
            .into_iter()
            .map(|item| StatuteIndexItem {
                canonical_id: item["canonical_id"].as_str().unwrap_or("").to_string(),
                citation: item["citation"].as_str().unwrap_or("").to_string(),
                title: item["title"].as_str().map(|value| value.to_string()),
                chapter: item["chapter"].as_str().unwrap_or("").to_string(),
                status: item["status"].as_str().unwrap_or("active").to_string(),
                edition_year: item["edition_year"].as_i64().unwrap_or(2025) as i32,
            })
            .collect::<Vec<_>>();

        Ok(StatuteIndexResponse {
            total: row.get::<i64>("total").unwrap_or(0).max(0) as u64,
            items,
            limit,
            offset,
        })
    }

    pub async fn get_statute(&self, citation: &str) -> ApiResult<StatuteDetailResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     OPTIONAL MATCH (v)-[:DERIVED_FROM]->(s:SourceDocument)
                     CALL {
                       WITH v
                       OPTIONAL MATCH (v)-[:CONTAINS]->(p:Provision)
                       RETURN count(DISTINCT p) AS provision_count
                     }
                     CALL {
                       WITH v
                       OPTIONAL MATCH (v)-[:CONTAINS]->(:Provision)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->()
                       RETURN count(DISTINCT r) AS outbound_count
                     }
                     CALL {
                       WITH i, v
                       OPTIONAL MATCH (source:Provision)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->(target)
                       WHERE target = i
                          OR target = v
                          OR (target:Provision AND EXISTS { MATCH (target)-[:PART_OF_VERSION]->(v) })
                       RETURN count(DISTINCT r) AS inbound_count
                     }
                     CALL {
                       WITH v
                       OPTIONAL MATCH (v)-[:CONTAINS]->(sp:Provision)-[:EXPRESSES|DEFINES]->(sem)
                       RETURN
                         count(DISTINCT CASE WHEN sem:Obligation THEN sem END) AS obligations,
                         count(DISTINCT CASE WHEN sem:Exception THEN sem END) AS exceptions,
                         count(DISTINCT CASE WHEN sem:Deadline THEN sem END) AS deadlines,
                         count(DISTINCT CASE WHEN sem:Penalty THEN sem END) AS penalties,
                         count(DISTINCT CASE WHEN sem:Definition THEN sem END) AS definitions
                     }
                     CALL {
                       WITH v
                       OPTIONAL MATCH (v)-[:HAS_SOURCE_NOTE]->(version_note:SourceNote)
                       WITH v, collect(DISTINCT version_note.text) AS version_notes
                       OPTIONAL MATCH (v)-[:CONTAINS]->(:Provision)-[:HAS_SOURCE_NOTE]->(provision_note:SourceNote)
                       WITH version_notes + collect(DISTINCT provision_note.text) AS notes
                       RETURN [note IN notes
                               WHERE note IS NOT NULL AND note <> ''][0..25] AS source_notes
                     }
                     RETURN i.canonical_id as canonical_id, i.citation as citation, i.title as title,
                            i.chapter as chapter, coalesce(i.status, v.status, 'active') as status,
                            v.version_id as version_id, coalesce(v.effective_date, '') as effective_date,
                            v.end_date as end_date, coalesce(v.is_current, v.current, false) as is_current, v.text as text,
                            s.source_document_id as source_id, s.url as url, s.edition_year as edition_year,
                            provision_count, outbound_count, inbound_count,
                            obligations, exceptions, deadlines, penalties, definitions, source_notes"
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {}", citation)))?;

        Ok(StatuteDetailResponse {
            identity: StatuteIdentity {
                canonical_id: row.get("canonical_id").unwrap_or_default(),
                citation: row.get("citation").unwrap_or_default(),
                title: row.get("title").ok(),
                chapter: row.get("chapter").unwrap_or_default(),
                status: row.get("status").unwrap_or_default(),
            },
            current_version: StatuteVersion {
                version_id: row.get("version_id").unwrap_or_default(),
                effective_date: row.get("effective_date").unwrap_or_default(),
                end_date: row.get("end_date").ok(),
                is_current: row.get("is_current").unwrap_or(false),
                text: row.get("text").unwrap_or_default(),
            },
            chapter: row.get("chapter").unwrap_or_default(),
            title: row.get("title").ok(),
            status: row.get("status").unwrap_or_default(),
            source_document: SourceDocument {
                source_id: row.get("source_id").unwrap_or_default(),
                url: row.get("url").unwrap_or_default(),
                edition_year: row.get("edition_year").unwrap_or(2025),
            },
            provision_count: row
                .get("provision_count")
                .ok()
                .and_then(|v: i64| Some(v as u64))
                .unwrap_or(0),
            citation_counts: CitationCounts {
                outbound: row.get::<i64>("outbound_count").unwrap_or(0).max(0) as u64,
                inbound: row.get::<i64>("inbound_count").unwrap_or(0).max(0) as u64,
            },
            semantic_counts: SemanticCounts {
                obligations: row.get::<i64>("obligations").unwrap_or(0).max(0) as u64,
                exceptions: row.get::<i64>("exceptions").unwrap_or(0).max(0) as u64,
                deadlines: row.get::<i64>("deadlines").unwrap_or(0).max(0) as u64,
                penalties: row.get::<i64>("penalties").unwrap_or(0).max(0) as u64,
                definitions: row.get::<i64>("definitions").unwrap_or(0).max(0) as u64,
            },
            source_notes: row.get("source_notes").unwrap_or_default(),
        })
    }

    pub async fn get_statute_page(&self, citation: &str) -> ApiResult<StatutePageResponse> {
        let detail = self.get_statute(citation).await?;
        let provisions = self.get_provisions(citation).await?;
        let StatuteDetailResponse {
            identity,
            current_version,
            source_document,
            provision_count,
            citation_counts,
            semantic_counts,
            source_notes,
            ..
        } = detail;

        let qc_notes = source_notes
            .iter()
            .enumerate()
            .map(|(index, message)| QCNoteItem {
                note_id: format!("source-note:{index}"),
                level: "info".to_string(),
                category: "source".to_string(),
                message: message.clone(),
                related_id: Some(identity.canonical_id.clone()),
            })
            .collect::<Vec<_>>();
        let qc_status = if qc_notes.is_empty() {
            "pass".to_string()
        } else {
            "warning".to_string()
        };

        Ok(StatutePageResponse {
            identity,
            current_version,
            source_document,
            provision_count,
            citation_counts,
            semantic_counts,
            source_notes,
            provisions: provisions.provisions,
            qc: StatutePageQcSummary {
                status: qc_status,
                passed_checks: if qc_notes.is_empty() { 2 } else { 1 },
                total_checks: 2,
                notes: qc_notes,
            },
        })
    }

    pub async fn get_provisions(&self, citation: &str) -> ApiResult<ProvisionsResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     MATCH (v)-[:CONTAINS]->(p:Provision)
                     RETURN p.provision_id as provision_id, p.display_citation as display_citation,
                            p.local_path as local_path, p.depth as depth, p.text as text
                     ORDER BY p.order_index",
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut provisions = Vec::new();

        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            provisions.push(ProvisionNode {
                provision_id: row.get("provision_id").unwrap_or_default(),
                display_citation: row.get("display_citation").unwrap_or_default(),
                local_path: row.get("local_path").unwrap_or_default(),
                depth: row.get("depth").unwrap_or(0),
                text: row.get("text").unwrap_or_default(),
                children: vec![],
            });
        }

        Ok(ProvisionsResponse {
            citation: citation.to_string(),
            provisions: nest_provisions(provisions),
        })
    }

    pub async fn get_citations(&self, citation: &str) -> ApiResult<CitationsResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(source:Provision)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->(target)
                       RETURN collect(DISTINCT {
                         target_canonical_id: coalesce(target.canonical_id, target.version_id, target.provision_id, target.chapter_id),
                         target_citation: coalesce(target.citation, target.display_citation, target.chapter, target.version_id, target.provision_id, target.chapter_id),
                         context_snippet: coalesce(r.raw_text, r.normalized_citation, ''),
                         source_provision: coalesce(source.display_citation, source.provision_id),
                         resolved: true
                       }) AS outbound
                     }
                     CALL {
                       WITH i, v
                       MATCH (target)
                       WHERE target = i
                          OR target = v
                          OR (target:Provision AND EXISTS { MATCH (target)-[:PART_OF_VERSION]->(v) })
                       MATCH (source:Provision)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->(target)
                       OPTIONAL MATCH (source)-[:PART_OF_VERSION]->(:LegalTextVersion)-[:VERSION_OF]->(source_identity:LegalTextIdentity)
                       RETURN collect(DISTINCT {
                         target_canonical_id: source_identity.canonical_id,
                         target_citation: coalesce(source_identity.citation, source.display_citation, source.citation, source.provision_id),
                         context_snippet: coalesce(r.raw_text, r.normalized_citation, ''),
                         source_provision: coalesce(source.display_citation, source.provision_id),
                         resolved: true
                       }) AS inbound
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(source:Provision)-[:MENTIONS_CITATION]->(cm:CitationMention)
                       WHERE cm.resolver_status IS NULL
                          OR NOT cm.resolver_status IN ['resolved', 'ok']
                          OR NOT (cm)-[:RESOLVES_TO|RESOLVES_TO_VERSION|RESOLVES_TO_PROVISION|RESOLVES_TO_CHAPTER|RESOLVES_TO_EXTERNAL]->()
                       RETURN collect(DISTINCT {
                         target_canonical_id: null,
                         target_citation: coalesce(cm.raw_text, cm.normalized_citation, ''),
                         context_snippet: coalesce(cm.raw_text, cm.normalized_citation, ''),
                         source_provision: coalesce(source.display_citation, source.provision_id),
                         resolved: false
                       }) AS unresolved
                     }
                     RETURN i.citation as citation, outbound, inbound, unresolved"
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {}", citation)))?;

        let outbound = json_to_citations(row.get("outbound").ok().unwrap_or_default());
        let inbound = json_to_citations(row.get("inbound").ok().unwrap_or_default());
        let unresolved = json_to_citations(row.get("unresolved").ok().unwrap_or_default());

        Ok(CitationsResponse {
            citation: row.get("citation").unwrap_or_else(|_| citation.to_string()),
            outbound,
            inbound,
            unresolved,
        })
    }

    pub async fn get_semantics(&self, citation: &str) -> ApiResult<SemanticsResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(p:Provision)-[:EXPRESSES]->(o:Obligation)
                       RETURN collect(DISTINCT {
                         text: coalesce(o.text, o.action_text, o.normalized_text, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..200] AS obligations
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(p:Provision)-[:EXPRESSES]->(e:Exception)
                       RETURN collect(DISTINCT {
                         text: coalesce(e.text, e.trigger_phrase, e.normalized_text, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..200] AS exceptions
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(p:Provision)-[:EXPRESSES]->(d:Deadline)
                       RETURN collect(DISTINCT {
                         description: coalesce(d.text, d.action_required, ''),
                         duration: coalesce(d.duration, d.date_text, ''),
                         trigger: coalesce(d.trigger_event, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..200] AS deadlines
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(p:Provision)-[:EXPRESSES]->(pnl:Penalty)
                       RETURN collect(DISTINCT {
                         text: coalesce(pnl.text, pnl.penalty_type, pnl.target_conduct, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..200] AS penalties
                     }
                     CALL {
                       WITH v
                       MATCH (v)-[:CONTAINS]->(p:Provision)-[:DEFINES]->(d:Definition)
                       OPTIONAL MATCH (d)-[:HAS_SCOPE]->(scope)
                       RETURN collect(DISTINCT {
                         term: coalesce(d.term, d.normalized_term, ''),
                         text: coalesce(d.definition_text, d.text, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id),
                         scope: coalesce(scope.scope_citation, d.scope_citation, scope.scope_type, d.scope_type, '')
                       })[0..200] AS definitions
                     }
                     RETURN i.citation AS citation, obligations, exceptions, deadlines, penalties, definitions",
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {}", citation)))?;

        Ok(SemanticsResponse {
            citation: row.get("citation").unwrap_or_else(|_| citation.to_string()),
            obligations: json_to_semantic_items(row.get("obligations").ok().unwrap_or_default()),
            exceptions: json_to_semantic_items(row.get("exceptions").ok().unwrap_or_default()),
            deadlines: json_to_deadlines(row.get("deadlines").ok().unwrap_or_default()),
            penalties: json_to_semantic_items(row.get("penalties").ok().unwrap_or_default()),
            definitions: json_to_definitions(row.get("definitions").ok().unwrap_or_default()),
        })
    }

    pub async fn get_history(&self, citation: &str) -> ApiResult<HistoryResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     CALL {
                       WITH v
                       OPTIONAL MATCH (v)-[:HAS_SOURCE_NOTE]->(version_note:SourceNote)
                       WITH v, collect(DISTINCT version_note) AS version_notes
                       OPTIONAL MATCH (v)-[:CONTAINS]->(:Provision)-[:HAS_SOURCE_NOTE]->(provision_note:SourceNote)
                       WITH version_notes + collect(DISTINCT provision_note) AS notes
                       RETURN [note IN notes WHERE note IS NOT NULL | note.text][0..200] AS source_notes
                     }
                     CALL {
                       WITH i, v
                       OPTIONAL MATCH (am:Amendment)
                       WHERE (am)-[:AFFECTS]->(i) OR (v IS NOT NULL AND (am)-[:AFFECTS_VERSION]->(v))
                       RETURN collect(DISTINCT {
                         amendment_id: am.amendment_id,
                         description: coalesce(am.text, am.raw_text, am.amendment_type, ''),
                         effective_date: coalesce(am.effective_date, '')
                       })[0..200] AS amendments
                     }
                     CALL {
                       WITH i, v
                       OPTIONAL MATCH (i)-[:HAS_STATUS_EVENT]->(identity_event:StatusEvent)
                       WITH i, v, collect(DISTINCT identity_event) AS identity_events
                       OPTIONAL MATCH (v)-[:HAS_STATUS_EVENT]->(version_event:StatusEvent)
                       WITH i, v, identity_events + collect(DISTINCT version_event) AS status_events
                       OPTIONAL MATCH (v)-[:HAS_TEMPORAL_EFFECT]->(version_effect:TemporalEffect)
                       WITH v, status_events, collect(DISTINCT version_effect) AS version_effects
                       OPTIONAL MATCH (v)-[:CONTAINS]->(:Provision)-[:HAS_TEMPORAL_EFFECT]->(provision_effect:TemporalEffect)
                       WITH status_events + version_effects + collect(DISTINCT provision_effect) AS events
                       RETURN [event IN events WHERE event IS NOT NULL | {
                         event_id: coalesce(event.status_event_id, event.temporal_effect_id),
                         event_type: coalesce(event.status_type, event.effect_type, 'temporal_effect'),
                         date: coalesce(event.effective_date, event.operative_date, event.repeal_date, event.expiration_date, ''),
                         description: coalesce(event.status_text, event.trigger_text, event.text, event.session_law_ref, '')
                       }][0..200] AS status_events
                     }
                     CALL {
                       WITH i, v
                       OPTIONAL MATCH (v)-[:HAS_SOURCE_NOTE]->(sn:SourceNote)-[:MENTIONS_SESSION_LAW]->(source_law:SessionLaw)
                       WITH i, v, collect(DISTINCT source_law) AS source_laws
                       OPTIONAL MATCH (am:Amendment)
                       WHERE (am)-[:AFFECTS]->(i) OR (v IS NOT NULL AND (am)-[:AFFECTS_VERSION]->(v))
                       OPTIONAL MATCH (law_from_amendment:SessionLaw)-[:ENACTS]->(am)
                       WITH v, source_laws + collect(DISTINCT law_from_amendment) AS laws
                       OPTIONAL MATCH (v)-[:HAS_TEMPORAL_EFFECT]->(:TemporalEffect)-[:REFERENCES_SESSION_LAW]->(law_from_effect:SessionLaw)
                       WITH laws + collect(DISTINCT law_from_effect) AS session_laws
                       RETURN [law IN session_laws WHERE law IS NOT NULL | {
                         session_law_id: law.session_law_id,
                         citation: coalesce(law.citation, ''),
                         description: coalesce(law.text, law.raw_text, law.bill_number, '')
                       }][0..200] AS session_laws
                     }
                     RETURN i.citation AS citation, source_notes, amendments, session_laws, status_events",
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {}", citation)))?;

        Ok(HistoryResponse {
            citation: row.get("citation").unwrap_or_else(|_| citation.to_string()),
            source_notes: row.get("source_notes").unwrap_or_default(),
            amendments: json_to_amendments(row.get("amendments").ok().unwrap_or_default()),
            session_laws: json_to_session_laws(row.get("session_laws").ok().unwrap_or_default()),
            status_events: json_to_status_events(row.get("status_events").ok().unwrap_or_default()),
        })
    }

    pub async fn get_chunks(&self, citation: &str) -> ApiResult<StatuteChunksResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     CALL {
                       WITH i
                       OPTIONAL MATCH (i)-[:HAS_VERSION]->(candidate:LegalTextVersion)
                       WITH candidate
                       ORDER BY coalesce(candidate.current, candidate.is_current, false) DESC,
                                coalesce(candidate.edition_year, 0) DESC,
                                coalesce(candidate.effective_date, '') DESC
                       RETURN candidate AS v
                       LIMIT 1
                     }
                     OPTIONAL MATCH (v)<-[:DERIVED_FROM]-(version_chunk:RetrievalChunk)
                     OPTIONAL MATCH (v)-[:CONTAINS]->(p:Provision)<-[:DERIVED_FROM]-(provision_chunk:RetrievalChunk)
                     WITH i, v,
                       collect(DISTINCT {
                         chunk_id: version_chunk.chunk_id,
                         chunk_type: version_chunk.chunk_type,
                         source_kind: coalesce(version_chunk.source_kind, 'statute'),
                         source_id: coalesce(version_chunk.source_id, v.version_id, i.canonical_id),
                         text: version_chunk.text,
                         embedding_policy: version_chunk.embedding_policy,
                         answer_policy: version_chunk.answer_policy,
                         search_weight: version_chunk.search_weight,
                         embedded: version_chunk.embedded,
                         parser_confidence: version_chunk.parser_confidence
                       }) +
                       collect(DISTINCT {
                         chunk_id: provision_chunk.chunk_id,
                         chunk_type: provision_chunk.chunk_type,
                         source_kind: coalesce(provision_chunk.source_kind, 'provision'),
                         source_id: coalesce(provision_chunk.source_id, p.provision_id),
                         text: provision_chunk.text,
                         embedding_policy: provision_chunk.embedding_policy,
                         answer_policy: provision_chunk.answer_policy,
                         search_weight: provision_chunk.search_weight,
                         embedded: provision_chunk.embedded,
                         parser_confidence: provision_chunk.parser_confidence
                       }) AS raw_chunks
                     RETURN i.citation AS citation,
                            [chunk IN raw_chunks WHERE chunk.chunk_id IS NOT NULL][0..500] AS chunks",
                )
                .param("citation", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {}", citation)))?;

        Ok(StatuteChunksResponse {
            citation: row.get("citation").unwrap_or_else(|_| citation.to_string()),
            chunks: json_to_chunks(row.get("chunks").ok().unwrap_or_default(), citation),
        })
    }

    pub async fn get_provision_detail(
        &self,
        provision_id: &str,
    ) -> ApiResult<ProvisionDetailResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (p:Provision)
                     WHERE p.provision_id = $provision_id OR p.display_citation = $provision_id
                     MATCH (p)-[:PART_OF_VERSION]->(v:LegalTextVersion)-[:VERSION_OF]->(i:LegalTextIdentity)
                     OPTIONAL MATCH (p)<-[:DERIVED_FROM]-(chunk:RetrievalChunk)
                     CALL {
                       WITH p
                       OPTIONAL MATCH (p)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->(target)
                       RETURN collect(DISTINCT {
                         target_canonical_id: coalesce(target.canonical_id, target.version_id, target.provision_id, target.chapter_id),
                         target_citation: coalesce(target.citation, target.display_citation, target.chapter, target.version_id, target.provision_id, target.chapter_id),
                         context_snippet: coalesce(r.raw_text, r.normalized_citation, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id),
                         resolved: true
                       })[0..50] AS outbound_nodes
                     }
                     CALL {
                       WITH p
                       OPTIONAL MATCH (source:Provision)-[r:CITES|CITES_VERSION|CITES_PROVISION|CITES_CHAPTER|CITES_RANGE]->(p)
                       OPTIONAL MATCH (source)-[:PART_OF_VERSION]->(:LegalTextVersion)-[:VERSION_OF]->(source_identity:LegalTextIdentity)
                       RETURN collect(DISTINCT {
                         target_canonical_id: source_identity.canonical_id,
                         target_citation: coalesce(source_identity.citation, source.display_citation, source.citation, source.provision_id),
                         context_snippet: coalesce(r.raw_text, r.normalized_citation, ''),
                         source_provision: coalesce(source.display_citation, source.provision_id),
                         resolved: true
                       })[0..50] AS inbound_nodes
                     }
                     OPTIONAL MATCH (p)-[:CONTAINS]->(child:Provision)
                     OPTIONAL MATCH path = (p)-[:HAS_PARENT*1..8]->(ancestor:Provision)
                     WITH p, v, i, chunk, outbound_nodes, inbound_nodes, child,
                          [node IN reverse(nodes(path)) WHERE node <> p | {
                            provision_id: node.provision_id,
                            citation: coalesce(node.display_citation, node.provision_id)
                          }] AS ancestor_links
                     OPTIONAL MATCH (p)-[:HAS_PARENT]->(parent:Provision)
                     OPTIONAL MATCH (parent)-[:CONTAINS]->(sibling_from_parent:Provision)
                     OPTIONAL MATCH (v)-[:CONTAINS]->(sibling_top:Provision)
                     WITH p, v, i, parent,
                       collect(DISTINCT ancestor_links) AS ancestor_groups,
                       collect(DISTINCT {
                         chunk_id: chunk.chunk_id,
                         chunk_type: chunk.chunk_type,
                         source_kind: chunk.source_kind,
                         source_id: chunk.source_id,
                         text: chunk.text,
                         embedding_policy: chunk.embedding_policy,
                         answer_policy: chunk.answer_policy,
                         search_weight: chunk.search_weight,
                         embedded: chunk.embedded,
                         parser_confidence: chunk.parser_confidence
                       })[0..25] as chunks,
                       collect(DISTINCT {
                         provision_id: child.provision_id,
                         display_citation: child.display_citation,
                         provision_type: coalesce(child.provision_type, child.kind, 'section'),
                         parent_id: child.parent_id,
                         text: child.text,
                         qc_status: coalesce(child.qc_status, 'pass'),
                         status: coalesce(child.status, i.status, 'active')
                       })[0..50] as children,
                       outbound_nodes,
                       inbound_nodes,
                       collect(DISTINCT CASE
                         WHEN parent IS NOT NULL AND sibling_from_parent IS NOT NULL THEN {
                           provision_id: sibling_from_parent.provision_id,
                           citation: coalesce(sibling_from_parent.display_citation, sibling_from_parent.provision_id)
                         }
                         WHEN parent IS NULL AND sibling_top IS NOT NULL AND NOT (sibling_top)-[:HAS_PARENT]->() AND sibling_top <> p THEN {
                           provision_id: sibling_top.provision_id,
                           citation: coalesce(sibling_top.display_citation, sibling_top.provision_id)
                         }
                       END)[0..50] as siblings
                     CALL {
                       WITH p
                       OPTIONAL MATCH (p)-[:DEFINES]->(d:Definition)
                       OPTIONAL MATCH (d)-[:HAS_SCOPE]->(scope)
                       RETURN collect(DISTINCT {
                         term: coalesce(d.term, d.normalized_term, ''),
                         text: coalesce(d.definition_text, d.text, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id),
                         scope: coalesce(scope.scope_citation, d.scope_citation, scope.scope_type, d.scope_type, '')
                       })[0..100] AS definitions
                     }
                     CALL {
                       WITH p
                       OPTIONAL MATCH (p)-[:EXPRESSES]->(e:Exception)
                       RETURN collect(DISTINCT {
                         exception_id: e.exception_id,
                         text: coalesce(e.text, e.trigger_phrase, ''),
                         applies_to_provision: coalesce(e.target_provision_id, e.target_canonical_id, p.provision_id),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..100] AS exceptions
                     }
                     CALL {
                       WITH p
                       OPTIONAL MATCH (p)-[:EXPRESSES]->(d:Deadline)
                       RETURN collect(DISTINCT {
                         description: coalesce(d.text, d.action_required, ''),
                         duration: coalesce(d.duration, d.date_text, ''),
                         trigger: coalesce(d.trigger_event, ''),
                         source_provision: coalesce(p.display_citation, p.provision_id)
                       })[0..100] AS deadlines
                     }
                     CALL {
                       WITH p
                       OPTIONAL MATCH (p)-[:HAS_SOURCE_NOTE]->(sn:SourceNote)
                       RETURN collect(DISTINCT {
                         note_id: sn.source_note_id,
                         level: coalesce(sn.qc_severity, 'info'),
                         category: coalesce(sn.note_type, 'source'),
                         message: coalesce(sn.text, sn.normalized_text, ''),
                         related_id: coalesce(sn.provision_id, sn.version_id, sn.canonical_id)
                       })[0..100] AS qc_notes
                     }
                     RETURN i.canonical_id as canonical_id,
                            i.citation as statute_citation,
                            i.title as statute_title,
                            i.chapter as chapter,
                            coalesce(i.status, 'active') as statute_status,
                            coalesce(i.edition_year, 2025) as edition_year,
                            p.provision_id as provision_id,
                            p.display_citation as display_citation,
                            coalesce(p.provision_type, p.kind, 'section') as provision_type,
                            p.parent_id as parent_id,
                            p.text as text,
                            coalesce(p.signals, []) as signals,
                            coalesce(p.status, i.status, 'active') as status,
                            coalesce(p.qc_status, 'pass') as qc_status,
                            chunks,
                            outbound_nodes,
                            inbound_nodes,
                            children,
                            ancestor_groups,
                            siblings,
                            definitions,
                            exceptions,
                            deadlines,
                            qc_notes",
                )
                .param("provision_id", provision_id),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let row = result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .ok_or_else(|| ApiError::NotFound(format!("Provision not found: {}", provision_id)))?;

        let text: String = row.get("text").unwrap_or_default();
        let provision = ProvisionDetail {
            provision_id: row.get("provision_id").unwrap_or_default(),
            display_citation: row.get("display_citation").unwrap_or_default(),
            provision_type: row
                .get("provision_type")
                .unwrap_or_else(|_| "section".to_string()),
            parent_id: row.get("parent_id").ok(),
            text_preview: preview_text(&text, 220),
            text,
            signals: row.get("signals").unwrap_or_default(),
            cites_count: 0,
            cited_by_count: 0,
            chunk_count: row
                .get::<Vec<serde_json::Value>>("chunks")
                .ok()
                .map(|chunks| chunks.len() as u64)
                .unwrap_or(0),
            qc_status: row.get("qc_status").unwrap_or_else(|_| "pass".to_string()),
            status: row.get("status").unwrap_or_else(|_| "active".to_string()),
        };

        let chunks_json: Vec<serde_json::Value> = row.get("chunks").ok().unwrap_or_default();
        let chunks = chunks_json
            .into_iter()
            .filter(|chunk| !json_string(chunk, "chunk_id").is_empty())
            .map(|chunk| ProvisionChunk {
                chunk_id: json_string(&chunk, "chunk_id"),
                chunk_type: json_string_or(&chunk, "chunk_type", "contextual_provision"),
                source_kind: json_string_or(&chunk, "source_kind", "provision"),
                source_id: json_string_or(&chunk, "source_id", &provision.provision_id),
                text: json_string(&chunk, "text"),
                embedding_policy: json_string_or(&chunk, "embedding_policy", "primary"),
                answer_policy: json_string_or(&chunk, "answer_policy", "supporting"),
                search_weight: chunk["search_weight"].as_f64().unwrap_or(1.0),
                embedded: chunk["embedding"].is_array()
                    || chunk["embedded"].as_bool().unwrap_or(false),
                parser_confidence: chunk["parser_confidence"].as_f64().unwrap_or(1.0),
            })
            .collect::<Vec<_>>();

        let children_json: Vec<serde_json::Value> = row.get("children").ok().unwrap_or_default();
        let children = children_json
            .into_iter()
            .filter(|child| !json_string(child, "provision_id").is_empty())
            .map(|child| {
                let text = json_string(&child, "text");
                ProvisionDetail {
                    provision_id: json_string(&child, "provision_id"),
                    display_citation: json_string(&child, "display_citation"),
                    provision_type: json_string_or(&child, "provision_type", "section"),
                    parent_id: child["parent_id"].as_str().map(|value| value.to_string()),
                    text_preview: preview_text(&text, 180),
                    text,
                    signals: Vec::new(),
                    cites_count: 0,
                    cited_by_count: 0,
                    chunk_count: 0,
                    qc_status: json_string_or(&child, "qc_status", "pass"),
                    status: json_string_or(&child, "status", "active"),
                }
            })
            .collect::<Vec<_>>();

        let ancestors =
            json_to_provision_links(row.get("ancestor_groups").ok().unwrap_or_default());
        let siblings = json_to_provision_links(row.get("siblings").ok().unwrap_or_default());
        let outbound = json_to_citations(row.get("outbound_nodes").ok().unwrap_or_default());
        let inbound = json_to_citations(row.get("inbound_nodes").ok().unwrap_or_default());

        Ok(ProvisionDetailResponse {
            parent_statute: StatuteIndexItem {
                canonical_id: row.get("canonical_id").unwrap_or_default(),
                citation: row.get("statute_citation").unwrap_or_default(),
                title: row.get("statute_title").ok(),
                chapter: row.get("chapter").unwrap_or_default(),
                status: row
                    .get("statute_status")
                    .unwrap_or_else(|_| "active".to_string()),
                edition_year: row.get("edition_year").unwrap_or(2025),
            },
            provision,
            ancestors,
            children,
            siblings,
            chunks,
            outbound_citations: outbound,
            inbound_citations: inbound,
            definitions: json_to_definitions(row.get("definitions").ok().unwrap_or_default()),
            exceptions: json_to_provision_exceptions(
                row.get("exceptions").ok().unwrap_or_default(),
            ),
            deadlines: json_to_deadlines(row.get("deadlines").ok().unwrap_or_default()),
            qc_notes: json_to_qc_notes(row.get("qc_notes").ok().unwrap_or_default()),
        })
    }

    pub async fn get_neighborhood(
        &self,
        params: &GraphNeighborhoodRequest,
    ) -> ApiResult<GraphNeighborhoodResponse> {
        let lookup = params
            .id
            .as_ref()
            .or(params.citation.as_ref())
            .ok_or_else(|| ApiError::BadRequest("id or citation is required".to_string()))?;
        let depth = params.depth.clamp(1, 2);
        let node_limit = params.limit.clamp(1, 500);
        let edge_limit = (node_limit * 3).min(1500);
        let mut relationship_types =
            graph_relationship_types(&params.mode, params.relationship_types.as_deref());
        if params.include_similarity.unwrap_or(false)
            && !relationship_types
                .iter()
                .any(|rel_type| rel_type == "SIMILAR_TO")
        {
            relationship_types.push("SIMILAR_TO".to_string());
        }
        let node_types = graph_csv(params.node_types.as_deref());
        let include_chunks = params.include_chunks.unwrap_or(false);
        let path_limit = edge_limit;
        let row_limit = (node_limit + edge_limit + 20) as i64;

        let node_id_expr = "coalesce(node.canonical_id, node.version_id, node.provision_id, node.chunk_id, node.semantic_id, node.source_note_id, node.definition_id, node.defined_term_id, node.deadline_id, node.penalty_id, node.exception_id, node.remedy_id, node.required_notice_id, node.notice_id, node.form_text_id, node.actor_id, node.action_id, node.mention_id, node.citation_mention_id, node.external_citation_id, node.status_event_id, node.temporal_effect_id, node.lineage_event_id, node.session_law_id, node.amendment_id, node.chapter_id, elementId(node))";
        let source_id_expr = "coalesce(source.canonical_id, source.version_id, source.provision_id, source.chunk_id, source.semantic_id, source.source_note_id, source.definition_id, source.defined_term_id, source.deadline_id, source.penalty_id, source.exception_id, source.remedy_id, source.required_notice_id, source.notice_id, source.form_text_id, source.actor_id, source.action_id, source.mention_id, source.citation_mention_id, source.external_citation_id, source.status_event_id, source.temporal_effect_id, source.lineage_event_id, source.session_law_id, source.amendment_id, source.chapter_id, elementId(source))";
        let target_id_expr = source_id_expr
            .replace("source.", "target.")
            .replace("elementId(source)", "elementId(target)");

        let cypher = format!(
            "MATCH (center)
             WHERE ($id IS NOT NULL AND any(value IN [
                    center.canonical_id, center.version_id, center.provision_id, center.chunk_id,
                    center.semantic_id, center.source_note_id, center.definition_id, center.defined_term_id,
                    center.deadline_id, center.penalty_id, center.exception_id, center.remedy_id,
                    center.required_notice_id, center.notice_id, center.form_text_id, center.actor_id,
                    center.action_id, center.mention_id, center.citation_mention_id, center.external_citation_id,
                    center.status_event_id, center.temporal_effect_id, center.lineage_event_id,
                    center.session_law_id, center.amendment_id, center.chapter_id, elementId(center)
                  ] WHERE value = $id))
                OR ($citation IS NOT NULL AND (
                    toUpper(coalesce(center.citation, '')) = toUpper($citation)
                    OR toUpper(coalesce(center.display_citation, '')) = toUpper($citation)
                  ))
             WITH center
             LIMIT 1
             CALL {{
               WITH center
               OPTIONAL MATCH path = (center)-[*1..{depth}]-(neighbor)
               WHERE path IS NULL OR all(rel IN relationships(path)
                 WHERE type(rel) IN $relationship_types
                   AND ($similarity_threshold < 0 OR type(rel) <> 'SIMILAR_TO' OR coalesce(rel.similarity_score, rel.score, rel.weight, 0.0) >= $similarity_threshold))
               WITH center, [p IN collect(path)[0..$path_limit] WHERE p IS NOT NULL] AS paths
               WITH [center] + reduce(all_nodes = [], p IN paths | all_nodes + nodes(p)) AS graph_nodes
               UNWIND graph_nodes AS node
               WITH DISTINCT node
               WHERE ($include_chunks = true OR NOT 'RetrievalChunk' IN labels(node))
                 AND (size($node_types) = 0 OR any(label IN labels(node) WHERE label IN $node_types))
                 AND ($min_confidence < 0 OR coalesce(node.confidence, node.parser_confidence, 1.0) >= $min_confidence)
               RETURN 'node' AS record_kind,
                      {node_id_expr} AS node_id,
                      coalesce(node.citation, node.display_citation, node.term, node.title, node.label, node.chapter_id, node.semantic_type, labels(node)[0], {node_id_expr}) AS node_label,
                      labels(node)[0] AS node_type,
                      labels(node) AS node_labels,
                      coalesce(node.citation, node.display_citation) AS citation,
                      node.title AS title,
                      node.chapter AS chapter,
                      node.status AS status,
                      left(coalesce(node.text, node.normalized_text, node.definition_text, node.raw_text, node.description, node.status_text, node.trigger_text, ''), 260) AS text_snippet,
                      coalesce(node.confidence, node.parser_confidence) AS confidence,
                      coalesce(node.source_backed, node.sourceBacked) AS source_backed,
                      coalesce(node.qc_warnings, node.parser_warnings, []) AS qc_warnings,
                      null AS edge_id,
                      null AS source_id,
                      null AS target_id,
                      null AS edge_type,
                      null AS edge_confidence,
                      null AS edge_weight,
                      null AS edge_similarity_score
               UNION ALL
               WITH center
               OPTIONAL MATCH path = (center)-[*1..{depth}]-(neighbor)
               WHERE path IS NULL OR all(rel IN relationships(path)
                 WHERE type(rel) IN $relationship_types
                   AND ($similarity_threshold < 0 OR type(rel) <> 'SIMILAR_TO' OR coalesce(rel.similarity_score, rel.score, rel.weight, 0.0) >= $similarity_threshold))
               WITH [p IN collect(path)[0..$path_limit] WHERE p IS NOT NULL] AS paths
               UNWIND paths AS path
               UNWIND relationships(path) AS rel
               WITH DISTINCT rel, startNode(rel) AS source, endNode(rel) AS target
               WHERE ($include_chunks = true OR (NOT 'RetrievalChunk' IN labels(source) AND NOT 'RetrievalChunk' IN labels(target)))
                 AND ($min_confidence < 0 OR coalesce(rel.confidence, 1.0) >= $min_confidence)
                 AND ($similarity_threshold < 0 OR type(rel) <> 'SIMILAR_TO' OR coalesce(rel.similarity_score, rel.score, rel.weight, 0.0) >= $similarity_threshold)
               RETURN 'edge' AS record_kind,
                      null AS node_id,
                      null AS node_label,
                      null AS node_type,
                      [] AS node_labels,
                      null AS citation,
                      null AS title,
                      null AS chapter,
                      null AS status,
                      null AS text_snippet,
                      null AS confidence,
                      null AS source_backed,
                      [] AS qc_warnings,
                      elementId(rel) AS edge_id,
                      {source_id_expr} AS source_id,
                      {target_id_expr} AS target_id,
                      type(rel) AS edge_type,
                      rel.confidence AS edge_confidence,
                      coalesce(rel.weight, rel.score, rel.similarity_score) AS edge_weight,
                      coalesce(rel.similarity_score, rel.score, rel.weight) AS edge_similarity_score
             }}
             RETURN *
             LIMIT $row_limit",
        );

        let mut result = self
            .graph
            .execute(
                query(&cypher)
                    .param("id", params.id.as_deref().unwrap_or(lookup))
                    .param("citation", params.citation.as_deref().unwrap_or(lookup))
                    .param("relationship_types", relationship_types.clone())
                    .param("node_types", node_types)
                    .param("include_chunks", include_chunks)
                    .param("min_confidence", params.min_confidence.unwrap_or(-1.0))
                    .param(
                        "similarity_threshold",
                        params.similarity_threshold.unwrap_or(-1.0),
                    )
                    .param("path_limit", path_limit as i64)
                    .param("row_limit", row_limit),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut nodes_by_id: HashMap<String, GraphNode> = HashMap::new();
        let mut edges_by_id: HashMap<String, GraphEdge> = HashMap::new();

        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            let record_kind: String = row.get("record_kind").unwrap_or_default();
            if record_kind == "node" {
                let id: String = row.get("node_id").unwrap_or_default();
                if id.is_empty() || nodes_by_id.len() >= node_limit {
                    continue;
                }
                let node_type = row
                    .get::<String>("node_type")
                    .unwrap_or_else(|_| "Unknown".to_string());
                nodes_by_id.entry(id.clone()).or_insert_with(|| GraphNode {
                    href: graph_node_href(&id, &node_type),
                    id,
                    label: row.get("node_label").unwrap_or_else(|_| node_type.clone()),
                    node_type: node_type.clone(),
                    labels: row
                        .get("node_labels")
                        .unwrap_or_else(|_| vec![node_type.clone()]),
                    citation: row.get("citation").ok(),
                    title: row.get("title").ok(),
                    chapter: row.get("chapter").ok(),
                    status: row.get("status").ok(),
                    text_snippet: row.get("text_snippet").ok(),
                    size: None,
                    score: None,
                    similarity_score: None,
                    confidence: row.get("confidence").ok(),
                    source_backed: row.get("source_backed").ok(),
                    qc_warnings: row.get("qc_warnings").unwrap_or_default(),
                    metrics: None,
                });
            } else if record_kind == "edge" && edges_by_id.len() < edge_limit {
                let edge_id: String = row.get("edge_id").unwrap_or_default();
                let source: String = row.get("source_id").unwrap_or_default();
                let target: String = row.get("target_id").unwrap_or_default();
                let edge_type: String = row.get("edge_type").unwrap_or_default();
                if edge_id.is_empty()
                    || source.is_empty()
                    || target.is_empty()
                    || edge_type.is_empty()
                {
                    continue;
                }
                edges_by_id.entry(edge_id.clone()).or_insert_with(|| {
                    let kind = graph_edge_kind(&edge_type).to_string();
                    GraphEdge {
                        id: edge_id,
                        source,
                        target,
                        edge_type: edge_type.clone(),
                        label: Some(edge_type.clone()),
                        kind,
                        weight: row.get("edge_weight").ok(),
                        confidence: row.get("edge_confidence").ok(),
                        similarity_score: row.get("edge_similarity_score").ok(),
                        source_backed: Some(true),
                        style: Some(graph_edge_style(&edge_type)),
                    }
                });
            }
        }

        let connected_ids: HashSet<String> = edges_by_id
            .values()
            .flat_map(|edge| [edge.source.clone(), edge.target.clone()])
            .collect();
        let mut nodes: Vec<GraphNode> = nodes_by_id.into_values().collect();
        nodes.sort_by_key(|node| {
            (
                if node.id == *lookup {
                    0
                } else if connected_ids.contains(&node.id) {
                    1
                } else {
                    2
                },
                node.label.clone(),
            )
        });

        let mut edges: Vec<GraphEdge> = edges_by_id.into_values().collect();
        edges.retain(|edge| {
            nodes.iter().any(|node| node.id == edge.source)
                && nodes.iter().any(|node| node.id == edge.target)
        });
        edges.sort_by_key(|edge| {
            (
                edge.edge_type.clone(),
                edge.source.clone(),
                edge.target.clone(),
            )
        });

        let center = nodes
            .iter()
            .find(|node| {
                Some(node.id.as_str()) == params.id.as_deref()
                    || node.citation.as_deref() == params.citation.as_deref()
                    || node.id == *lookup
            })
            .cloned()
            .or_else(|| nodes.first().cloned());

        let mut warnings = Vec::new();
        let truncated = nodes.len() >= node_limit || edges.len() >= edge_limit;
        if truncated {
            warnings.push(format!(
                "Neighborhood truncated to {} nodes and {} edges.",
                node_limit, edge_limit
            ));
        }
        Ok(GraphNeighborhoodResponse {
            center,
            stats: GraphStats {
                node_count: nodes.len(),
                edge_count: edges.len(),
                truncated,
                warnings,
            },
            nodes,
            edges,
            layout: Some(GraphLayoutHint {
                name: match params.mode.as_str() {
                    "history" => "timeline".to_string(),
                    "citation" => "radial".to_string(),
                    _ => "force".to_string(),
                },
            }),
        })
    }

    pub async fn list_sources(
        &self,
        params: &SourceIndexRequest,
    ) -> ApiResult<SourceIndexResponse> {
        let limit = params.limit.unwrap_or(50).clamp(1, 200);
        let offset = params.offset.unwrap_or(0);
        let q = params.q.as_deref().unwrap_or("").trim().to_lowercase();
        let status = normalized_filter(params.status.as_deref());

        let rows = self
            .run_rows(
                query(
                    "MATCH (sd:SourceDocument)
                     WITH sd, coalesce(sd.source_document_id, sd.source_id, sd.id) AS source_id
                     WHERE ($q = ''
                         OR toLower(coalesce(sd.title, sd.chapter_title, sd.file_name, source_id, '')) CONTAINS $q
                         OR toLower(coalesce(sd.chapter, sd.source_kind, sd.authority_family, '')) CONTAINS $q)
                       AND ($edition_year < 0 OR coalesce(sd.edition_year, -1) = $edition_year)
                     OPTIONAL MATCH (p:Provision)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (c:RetrievalChunk)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (cm:CitationMention)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (v:LegalTextVersion)-[:DERIVED_FROM]->(sd)
                     RETURN source_id,
                            coalesce(sd.title, sd.chapter_title, sd.file_name, source_id) AS title,
                            coalesce(sd.jurisdiction, sd.jurisdiction_id, 'Oregon') AS jurisdiction,
                            coalesce(sd.scope, sd.chapter_title, sd.chapter, sd.authority_family, sd.source_kind, 'source') AS scope,
                            coalesce(sd.url, '') AS url,
                            toString(coalesce(sd.retrieved_at, sd.updated_at, sd.created_at, '')) AS retrieved_at,
                            coalesce(sd.raw_hash, '') AS raw_hash,
                            coalesce(sd.normalized_hash, '') AS normalized_hash,
                            coalesce(sd.edition_year, 0) AS edition_year,
                            coalesce(sd.parser_profile, sd.source_system, 'unknown') AS parser_profile,
                            coalesce(sd.parser_warnings, []) AS parser_warnings,
                            coalesce(sd.byte_size, sd.size_bytes, sd.content_length, 0) AS byte_size,
                            count(DISTINCT v) AS sections,
                            count(DISTINCT p) AS provisions,
                            count(DISTINCT c) AS chunks,
                            count(DISTINCT cm) AS citation_mentions
                     ORDER BY edition_year DESC, title ASC
                     LIMIT 1000",
                )
                .param("q", q)
                .param("edition_year", params.edition_year.unwrap_or(-1)),
            )
            .await?;

        let mut items: Vec<SourceIndexItem> = rows
            .iter()
            .map(source_item_from_row)
            .collect::<ApiResult<Vec<_>>>()?;

        if let Some(status) = status {
            items.retain(|item| item.ingestion_status == status);
        }

        let total = items.len() as u64;
        let paged = items
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok(SourceIndexResponse {
            items: paged,
            total,
            limit,
            offset,
        })
    }

    pub async fn get_source(&self, source_id: &str) -> ApiResult<SourceDetailResponse> {
        let rows = self
            .run_rows(
                query(
                    "MATCH (sd:SourceDocument)
                     WITH sd, coalesce(sd.source_document_id, sd.source_id, sd.id) AS source_id
                     WHERE source_id = $source_id
                     OPTIONAL MATCH (p:Provision)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (c:RetrievalChunk)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (cm:CitationMention)-[:DERIVED_FROM]->(sd)
                     OPTIONAL MATCH (v:LegalTextVersion)-[:DERIVED_FROM]->(sd)
                     RETURN source_id,
                            coalesce(sd.title, sd.chapter_title, sd.file_name, source_id) AS title,
                            coalesce(sd.jurisdiction, sd.jurisdiction_id, 'Oregon') AS jurisdiction,
                            coalesce(sd.scope, sd.chapter_title, sd.chapter, sd.authority_family, sd.source_kind, 'source') AS scope,
                            coalesce(sd.url, '') AS url,
                            toString(coalesce(sd.retrieved_at, sd.updated_at, sd.created_at, '')) AS retrieved_at,
                            coalesce(sd.raw_hash, '') AS raw_hash,
                            coalesce(sd.normalized_hash, '') AS normalized_hash,
                            coalesce(sd.edition_year, 0) AS edition_year,
                            coalesce(sd.parser_profile, sd.source_system, 'unknown') AS parser_profile,
                            coalesce(sd.parser_warnings, []) AS parser_warnings,
                            coalesce(sd.byte_size, sd.size_bytes, sd.content_length, 0) AS byte_size,
                            count(DISTINCT v) AS sections,
                            count(DISTINCT p) AS provisions,
                            count(DISTINCT c) AS chunks,
                            count(DISTINCT cm) AS citation_mentions",
                )
                .param("source_id", source_id),
            )
            .await?;

        let source = rows
            .first()
            .map(source_item_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::NotFound(format!("Source {source_id} not found")))?;

        let related = self
            .list_sources(&SourceIndexRequest {
                q: Some(source.scope.clone()),
                status: None,
                edition_year: Some(source.edition_year),
                limit: Some(7),
                offset: Some(0),
            })
            .await?
            .items
            .into_iter()
            .filter(|item| item.source_id != source.source_id)
            .take(6)
            .collect();

        Ok(SourceDetailResponse {
            source,
            related_sources: related,
        })
    }

    pub async fn get_graph_path(&self, params: &GraphPathRequest) -> ApiResult<GraphPathResponse> {
        let limit = params.limit.unwrap_or(3).clamp(1, 10);
        let relationship_types =
            graph_relationship_types(params.mode.as_deref().unwrap_or("legal"), None);
        let node_id_expr = "coalesce(node.canonical_id, node.version_id, node.provision_id, node.chunk_id, node.semantic_id, node.source_note_id, node.definition_id, node.defined_term_id, node.deadline_id, node.penalty_id, node.exception_id, node.remedy_id, node.required_notice_id, node.notice_id, node.form_text_id, node.actor_id, node.action_id, node.mention_id, node.citation_mention_id, node.external_citation_id, node.status_event_id, node.temporal_effect_id, node.lineage_event_id, node.session_law_id, node.amendment_id, node.chapter_id, elementId(node))";
        let source_id_expr = node_id_expr
            .replace("node.", "startNode(rel).")
            .replace("elementId(node)", "elementId(startNode(rel))");
        let target_id_expr = node_id_expr
            .replace("node.", "endNode(rel).")
            .replace("elementId(node)", "elementId(endNode(rel))");
        let from_lookup_expr = node_id_expr
            .replace("node.", "from.")
            .replace("elementId(node)", "elementId(from)");
        let to_lookup_expr = node_id_expr
            .replace("node.", "to.")
            .replace("elementId(node)", "elementId(to)");

        let cypher = format!(
            "MATCH (from), (to)
             WHERE ({from_lookup_expr} = $from OR toUpper(coalesce(from.citation, from.display_citation, '')) = toUpper($from))
               AND ({to_lookup_expr} = $to OR toUpper(coalesce(to.citation, to.display_citation, '')) = toUpper($to))
             WITH from, to
             LIMIT 1
             MATCH path = shortestPath((from)-[*..6]-(to))
             WHERE all(rel IN relationships(path) WHERE type(rel) IN $relationship_types)
             WITH path
             LIMIT $limit
             RETURN
               [node IN nodes(path) | {{
                 id: {node_id_expr},
                 label: coalesce(node.citation, node.display_citation, node.term, node.title, node.label, labels(node)[0], {node_id_expr}),
                 type: labels(node)[0],
                 labels: labels(node),
                 citation: coalesce(node.citation, node.display_citation),
                 title: node.title,
                 chapter: node.chapter,
                 status: node.status,
                 textSnippet: left(coalesce(node.text, node.normalized_text, node.definition_text, node.raw_text, node.description, ''), 220),
                 confidence: coalesce(node.confidence, node.parser_confidence),
                 sourceBacked: coalesce(node.source_backed, node.sourceBacked),
                 qcWarnings: coalesce(node.qc_warnings, node.parser_warnings, [])
               }}] AS nodes,
               [rel IN relationships(path) | {{
                 id: elementId(rel),
                 source: {source_id_expr},
                 target: {target_id_expr},
                 type: type(rel),
                 label: type(rel),
                 weight: coalesce(rel.weight, rel.score, rel.similarity_score),
                 confidence: rel.confidence
               }}] AS edges",
        );

        let rows = self
            .run_rows(
                query(&cypher)
                    .param("from", params.from.clone())
                    .param("to", params.to.clone())
                    .param("relationship_types", relationship_types)
                    .param("limit", limit as i64),
            )
            .await?;

        let mut nodes_by_id: HashMap<String, GraphNode> = HashMap::new();
        let mut edges_by_id: HashMap<String, GraphEdge> = HashMap::new();
        let mut paths = Vec::new();

        for row in rows {
            let node_values: Vec<serde_json::Value> = row.get("nodes").unwrap_or_default();
            let edge_values: Vec<serde_json::Value> = row.get("edges").unwrap_or_default();
            let mut node_ids = Vec::new();
            let mut edge_ids = Vec::new();

            for value in node_values {
                let node = graph_node_from_json(&value);
                if !node.id.is_empty() {
                    node_ids.push(node.id.clone());
                    nodes_by_id.entry(node.id.clone()).or_insert(node);
                }
            }
            for value in edge_values {
                let edge = graph_edge_from_json(&value);
                if !edge.id.is_empty() && !edge.source.is_empty() && !edge.target.is_empty() {
                    edge_ids.push(edge.id.clone());
                    edges_by_id.entry(edge.id.clone()).or_insert(edge);
                }
            }
            paths.push(GraphPath {
                length: edge_ids.len(),
                node_ids,
                edge_ids,
            });
        }

        let mut nodes: Vec<GraphNode> = nodes_by_id.into_values().collect();
        nodes.sort_by(|left, right| left.label.cmp(&right.label));
        let mut edges: Vec<GraphEdge> = edges_by_id.into_values().collect();
        edges.sort_by(|left, right| left.id.cmp(&right.id));
        let warnings = if paths.is_empty() {
            vec!["No path found within six graph hops for the selected nodes.".to_string()]
        } else {
            Vec::new()
        };

        Ok(GraphPathResponse {
            from: params.from.clone(),
            to: params.to.clone(),
            stats: GraphStats {
                node_count: nodes.len(),
                edge_count: edges.len(),
                truncated: false,
                warnings,
            },
            paths,
            nodes,
            edges,
        })
    }

    pub async fn get_qc_summary(&self) -> ApiResult<QCSummaryResponse> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (n)
                 RETURN labels(n)[0] as label, count(n) as count
                 ORDER BY count DESC",
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut node_counts = Vec::new();

        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            let label: Option<String> = row.get("label").ok();
            if let Some(label) = label {
                node_counts.push(NodeCount {
                    label,
                    count: row
                        .get("count")
                        .ok()
                        .and_then(|v: i64| Some(v as u64))
                        .unwrap_or(0),
                });
            }
        }

        let mut result = self
            .graph
            .execute(query(
                "MATCH ()-[r]->()
                 RETURN type(r) as rel_type, count(r) as count
                 ORDER BY count DESC",
            ))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut relationship_counts = Vec::new();

        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            if let Ok(rel_type) = row.get::<String>("rel_type") {
                relationship_counts.push(RelationshipCount {
                    rel_type,
                    count: row
                        .get("count")
                        .ok()
                        .and_then(|v: i64| Some(v as u64))
                        .unwrap_or(0),
                });
            }
        }

        let orphan_provisions = self
            .query_count(
                "MATCH (p:Provision)
                 WHERE NOT (p)-[:PART_OF_VERSION]->()
                 RETURN count(p) as count",
            )
            .await?;
        let orphan_chunks = self
            .query_count(
                "MATCH (c:RetrievalChunk)
                 WHERE NOT (c)-[:DERIVED_FROM]->()
                 RETURN count(c) as count",
            )
            .await?;
        let orphan_citations = self
            .query_count(
                "MATCH (cm:CitationMention)
                 WHERE NOT ()-[:MENTIONS_CITATION]->(cm)
                 RETURN count(cm) as count",
            )
            .await?;
        let duplicate_legal_text_identities = self
            .query_count(
                "MATCH (n:LegalTextIdentity)
                 WITH n.citation as citation, count(n) as cnt
                 WHERE cnt > 1
                 RETURN count(*) as count",
            )
            .await?;
        let duplicate_provisions = self
            .query_count(
                "MATCH (n:Provision)
                 WITH n.provision_id as pid, count(n) as cnt
                 WHERE cnt > 1
                 RETURN count(*) as count",
            )
            .await?;
        let duplicate_cites_relationships = self
            .query_count(
                "MATCH ()-[r:CITES]->()
                 WITH count(r) as total
                 MATCH ()-[r:CITES]->()
                 WITH total, count(DISTINCT {
                     s: startNode(r).provision_id,
                     e: coalesce(endNode(r).canonical_id, endNode(r).provision_id)
                 }) as unique
                 RETURN total - unique as count",
            )
            .await?;

        let total_chunks = self
            .query_count("MATCH (c:RetrievalChunk) RETURN count(c) as count")
            .await?;
        let embedded_chunks = self
            .query_count(
                "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL
                 RETURN count(c) as count",
            )
            .await?;
        let total_citations = self
            .query_count("MATCH (cm:CitationMention) RETURN count(cm) as count")
            .await?;
        let resolved_citations = self
            .query_count(
                "MATCH (cm:CitationMention)
                 WHERE (cm)-[:RESOLVES_TO]->() OR (cm)-[:RESOLVES_TO_VERSION]->() OR (cm)-[:RESOLVES_TO_CHAPTER]->()
                 RETURN count(DISTINCT cm) as count",
            )
            .await?;

        Ok(QCSummaryResponse {
            node_counts_by_label: node_counts,
            relationship_counts_by_type: relationship_counts,
            orphan_counts: OrphanCounts {
                provisions: orphan_provisions,
                chunks: orphan_chunks,
                citations: orphan_citations,
            },
            duplicate_counts: DuplicateCounts {
                legal_text_identities: duplicate_legal_text_identities,
                provisions: duplicate_provisions,
                cites_relationships: duplicate_cites_relationships,
            },
            embedding_readiness: EmbeddingReadiness {
                total_chunks,
                embedded_chunks,
                coverage: percentage(embedded_chunks, total_chunks),
            },
            cites_coverage: CitesCoverage {
                total_citations,
                resolved_citations,
                coverage: percentage(resolved_citations, total_citations),
            },
            last_qc_status: None,
        })
    }

    pub async fn run_qc(&self) -> ApiResult<QCRunResponse> {
        let started_at = unix_timestamp_string();
        let summary = self.get_qc_summary().await?;
        let completed_at = unix_timestamp_string();
        let status = if summary.duplicate_counts.legal_text_identities > 0
            || summary.duplicate_counts.provisions > 0
            || summary.duplicate_counts.cites_relationships > 0
        {
            "warning"
        } else {
            "succeeded"
        };

        Ok(QCRunResponse {
            run_id: format!("qc:run:{completed_at}"),
            status: status.to_string(),
            started_at,
            completed_at,
            summary,
            warnings: Vec::new(),
        })
    }

    pub async fn get_qc_report(&self, format: Option<&str>) -> ApiResult<QCReportResponse> {
        let format = format.unwrap_or("json").trim().to_ascii_lowercase();
        if format != "json" && format != "csv" {
            return Err(ApiError::BadRequest(
                "QC report format must be json or csv".to_string(),
            ));
        }
        let summary = self.get_qc_summary().await?;
        let generated_at = unix_timestamp_string();
        let content = if format == "csv" {
            qc_summary_csv(&summary)
        } else {
            serde_json::to_string_pretty(&summary)
                .map_err(|error| ApiError::Internal(error.to_string()))?
        };

        Ok(QCReportResponse {
            report_id: format!("qc:report:{generated_at}"),
            mime_type: if format == "csv" {
                "text/csv".to_string()
            } else {
                "application/json".to_string()
            },
            format,
            generated_at,
            summary,
            content,
        })
    }

    async fn query_count(&self, statement: &str) -> ApiResult<u64> {
        let mut result = self
            .graph
            .execute(query(statement))
            .await
            .map_err(ApiError::Neo4jConnection)?;

        Ok(result
            .next()
            .await
            .map_err(ApiError::Neo4jConnection)?
            .and_then(|row| row.get::<i64>("count").ok())
            .unwrap_or(0) as u64)
    }

    pub async fn expand_query_terms(
        &self,
        q: &str,
        filters: &SearchRetrievalFilters,
        limit: u32,
    ) -> ApiResult<Vec<QueryExpansionTerm>> {
        let q_norm = q.trim().to_ascii_lowercase();
        if q_norm.len() < 3 {
            return Ok(Vec::new());
        }

        let mut result = self.graph.execute(
            query("CALL {
                     MATCH (n:DefinedTerm)
                     WHERE coalesce(n.term, '') <> ''
                       AND (
                         toLower(n.term) CONTAINS $q
                         OR toLower(coalesce(n.normalized_term, '')) CONTAINS $q
                         OR $q CONTAINS toLower(n.term)
                       )
                     RETURN n.term AS term,
                            n.normalized_term AS normalized_term,
                            'defined_term' AS kind,
                            n.defined_term_id AS source_id,
                            null AS source_citation,
                            0.9 AS score
                     UNION
                     MATCH (n:Definition)
                     WHERE coalesce(n.term, '') <> ''
                       AND ($chapter = '' OR coalesce(n.scope_citation, n.source_provision_id, '') CONTAINS $chapter)
                       AND (
                         toLower(n.term) CONTAINS $q
                         OR toLower(coalesce(n.normalized_term, '')) CONTAINS $q
                         OR toLower(coalesce(n.definition_text, '')) CONTAINS $q
                         OR $q CONTAINS toLower(n.term)
                       )
                     RETURN n.term AS term,
                            n.normalized_term AS normalized_term,
                            'definition' AS kind,
                            n.definition_id AS source_id,
                            n.source_provision_id AS source_citation,
                            0.82 AS score
                     UNION
                     MATCH (n:LegalActor)
                     WITH n, coalesce(n.name, n.actor_text, n.normalized_actor, n.normalized_name, '') AS term
                     WHERE term <> '' AND (toLower(term) CONTAINS $q OR $q CONTAINS toLower(term))
                     RETURN term AS term,
                            coalesce(n.normalized_actor, n.normalized_name) AS normalized_term,
                            'legal_actor' AS kind,
                            n.actor_id AS source_id,
                            n.citation AS source_citation,
                            0.72 AS score
                     UNION
                     MATCH (n:LegalAction)
                     WITH n, coalesce(n.normalized_action, n.action_text, n.verb, n.object_text, '') AS term
                     WHERE term <> '' AND (toLower(term) CONTAINS $q OR $q CONTAINS toLower(term))
                     RETURN term AS term,
                            n.normalized_action AS normalized_term,
                            'legal_action' AS kind,
                            n.action_id AS source_id,
                            n.citation AS source_citation,
                            0.68 AS score
                   }
                   WITH term, normalized_term, kind, source_id, source_citation, max(score) AS score
                   RETURN term, normalized_term, kind, source_id, source_citation, score
                   ORDER BY score DESC, size(term) ASC
                   LIMIT $limit")
            .param("q", q_norm)
            .param("chapter", filters.chapter.clone().unwrap_or_default())
            .param("limit", limit.max(1) as i64)
        ).await.map_err(ApiError::Neo4jConnection)?;

        let mut terms = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            terms.push(QueryExpansionTerm {
                term: row.get("term").unwrap_or_default(),
                normalized_term: row.get("normalized_term").ok(),
                kind: row.get("kind").unwrap_or_default(),
                source_id: row.get("source_id").ok(),
                source_citation: row.get("source_citation").ok(),
                score: row.get::<f64>("score").unwrap_or(0.0) as f32,
            });
        }
        Ok(terms)
    }

    pub async fn suggest(&self, q: &str, limit: u32) -> ApiResult<Vec<SuggestResult>> {
        let q = q.trim();
        let explicit_utcr_re = regex::Regex::new(r"(?i)^UTCR\s+\d{1,3}(?:\.\d*)?").unwrap();
        let bare_citation_re = regex::Regex::new(r"^\d{1,3}[A-Za-z]?\.\d*").unwrap();
        let normalized_q = if explicit_utcr_re.is_match(q) {
            q.to_ascii_uppercase()
        } else if bare_citation_re.is_match(q) {
            format!("ORS {q}")
        } else {
            q.to_string()
        };

        let mut result = self.graph.execute(
            query("CALL {
                     MATCH (n:Provision)
	                     WHERE toUpper(n.display_citation) STARTS WITH toUpper($normalized_q)
	                        OR toUpper(n.display_citation) STARTS WITH toUpper($q)
	                     RETURN n.display_citation as label,
	                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule_provision' ELSE 'provision' END as kind,
	                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR'
	                              THEN '/rules/utcr/' + coalesce(n.canonical_id, n.display_citation) + '?provision=' + n.provision_id
	                              ELSE '/statutes/' + coalesce(n.canonical_id, n.display_citation) + '?provision=' + n.provision_id
	                            END as href,
	                            n.display_citation as citation,
	                            coalesce(n.canonical_id, n.provision_id) as canonical_id,
	                            'exact_provision' as match_type,
                            100.0 as score
                     UNION
                     MATCH (n:LegalTextIdentity)
                     WHERE toUpper(n.citation) STARTS WITH toUpper($normalized_q)
	                        OR toUpper(n.citation) STARTS WITH toUpper($q)
	                        OR toUpper(n.title) CONTAINS toUpper($q)
	                     RETURN n.citation as label,
	                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR' THEN 'court_rule' ELSE 'statute' END as kind,
	                            CASE WHEN coalesce(n.authority_family, 'ORS') = 'UTCR'
	                              THEN '/rules/utcr/' + n.canonical_id
	                              ELSE '/statutes/' + n.canonical_id
	                            END as href,
                            n.citation as citation,
                            n.canonical_id as canonical_id,
                            'exact_statute' as match_type,
                            CASE WHEN toUpper(n.citation) STARTS WITH toUpper($normalized_q) THEN 95.0 ELSE 65.0 END as score
                     UNION
                     MATCH (n:DefinedTerm)
                     WHERE toUpper(n.term) STARTS WITH toUpper($q)
                        OR toUpper(coalesce(n.normalized_term, '')) STARTS WITH toUpper($q)
                     RETURN n.term as label,
                            'definition' as kind,
                            '/search?q=' + n.term as href,
                            null as citation,
                            n.defined_term_id as canonical_id,
                            'none' as match_type,
                            45.0 as score
                     UNION
                     MATCH (n:SourceDocument)
                     WHERE toUpper(n.title) CONTAINS toUpper($q)
                     RETURN n.title as label,
                            'chapter' as kind,
                            '/statutes/' + n.source_document_id as href,
                            null as citation,
                            n.source_document_id as canonical_id,
                            'none' as match_type,
                            35.0 as score
                   }
                   WITH label, kind, href, citation, canonical_id, match_type, max(score) AS score
                   RETURN label, kind, href, citation, canonical_id, match_type, score
                   ORDER BY score DESC, label
                   LIMIT $limit")
            .param("q", q)
            .param("normalized_q", normalized_q)
            .param("limit", limit.max(1) as i64)
        ).await.map_err(ApiError::Neo4jConnection)?;

        let mut suggestions = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            let match_type = match row
                .get::<String>("match_type")
                .unwrap_or_else(|_| "none".to_string())
                .as_str()
            {
                "exact_provision" => DirectMatchType::ExactProvision,
                "exact_statute" => DirectMatchType::ExactStatute,
                "parent_statute" => DirectMatchType::ParentStatute,
                _ => DirectMatchType::None,
            };
            suggestions.push(SuggestResult {
                label: row.get("label").unwrap_or_default(),
                kind: row.get("kind").unwrap_or_default(),
                href: row.get("href").unwrap_or_default(),
                citation: row.get("citation").ok(),
                canonical_id: row.get("canonical_id").ok(),
                match_type,
                score: row.get::<f64>("score").unwrap_or(0.0) as f32,
            });
        }
        Ok(suggestions)
    }
}

fn infer_authority_family_from_citation(citation: &str) -> Option<String> {
    let upper = citation.trim().to_ascii_uppercase();
    if upper.starts_with("UTCR ") {
        Some("UTCR".to_string())
    } else if upper.starts_with("ORS ") {
        Some("ORS".to_string())
    } else {
        None
    }
}

fn authority_type_for_family(authority_family: Option<&str>) -> Option<&'static str> {
    match authority_family {
        Some("UTCR") => Some("court_rule"),
        Some("ORS") => Some("statute"),
        _ => None,
    }
}

fn corpus_id_for_family(authority_family: Option<&str>) -> Option<&'static str> {
    match authority_family {
        Some("UTCR") => Some("or:utcr"),
        Some("ORS") => Some("or:ors"),
        _ => None,
    }
}

fn source_item_from_row(row: &Row) -> ApiResult<SourceIndexItem> {
    let raw_hash: String = row.get("raw_hash").unwrap_or_default();
    let normalized_hash: String = row.get("normalized_hash").unwrap_or_default();
    let parser_warnings: Vec<String> = row.get("parser_warnings").unwrap_or_default();
    let ingestion_status = if raw_hash.is_empty() {
        "queued"
    } else if normalized_hash.is_empty() {
        "failed"
    } else {
        "ingested"
    };

    Ok(SourceIndexItem {
        source_id: row.get("source_id").unwrap_or_default(),
        title: row.get("title").unwrap_or_default(),
        jurisdiction: row.get("jurisdiction").unwrap_or_default(),
        scope: row.get("scope").unwrap_or_default(),
        url: row.get("url").unwrap_or_default(),
        retrieved_at: row.get("retrieved_at").unwrap_or_default(),
        raw_hash,
        normalized_hash,
        edition_year: row.get::<i64>("edition_year").unwrap_or(0) as i32,
        parser_profile: row.get("parser_profile").unwrap_or_default(),
        parser_warnings,
        byte_size: row.get::<i64>("byte_size").unwrap_or(0).max(0) as u64,
        ingestion_status: ingestion_status.to_string(),
        produced: SourceProducedCounts {
            sections: row.get::<i64>("sections").unwrap_or(0).max(0) as u64,
            provisions: row.get::<i64>("provisions").unwrap_or(0).max(0) as u64,
            chunks: row.get::<i64>("chunks").unwrap_or(0).max(0) as u64,
            citation_mentions: row.get::<i64>("citation_mentions").unwrap_or(0).max(0) as u64,
        },
    })
}

fn graph_node_from_json(value: &serde_json::Value) -> GraphNode {
    let id = json_string(value, "id");
    let node_type = json_string_or(value, "type", "Unknown");
    GraphNode {
        href: graph_node_href(&id, &node_type),
        id,
        label: json_string_or(value, "label", &node_type),
        node_type: node_type.clone(),
        labels: json_string_array(value, "labels")
            .into_iter()
            .chain(std::iter::once(node_type.clone()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
        citation: json_optional_string(value, "citation"),
        title: json_optional_string(value, "title"),
        chapter: json_optional_string(value, "chapter"),
        status: json_optional_string(value, "status"),
        text_snippet: json_optional_string(value, "textSnippet"),
        size: None,
        score: None,
        similarity_score: None,
        confidence: value["confidence"].as_f64(),
        source_backed: value["sourceBacked"].as_bool(),
        qc_warnings: json_string_array(value, "qcWarnings"),
        metrics: None,
    }
}

fn graph_edge_from_json(value: &serde_json::Value) -> GraphEdge {
    let edge_type = json_string_or(value, "type", "RELATED");
    GraphEdge {
        id: json_string(value, "id"),
        source: json_string(value, "source"),
        target: json_string(value, "target"),
        label: json_optional_string(value, "label").or_else(|| Some(edge_type.clone())),
        kind: graph_edge_kind(&edge_type).to_string(),
        weight: value["weight"].as_f64(),
        confidence: value["confidence"].as_f64(),
        similarity_score: None,
        source_backed: Some(true),
        style: Some(graph_edge_style(&edge_type)),
        edge_type,
    }
}

fn json_optional_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value[key]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn qc_summary_csv(summary: &QCSummaryResponse) -> String {
    let mut lines = vec!["section,key,value".to_string()];
    for item in &summary.node_counts_by_label {
        lines.push(format!(
            "node_count,{},{}",
            csv_cell(&item.label),
            item.count
        ));
    }
    for item in &summary.relationship_counts_by_type {
        lines.push(format!(
            "relationship_count,{},{}",
            csv_cell(&item.rel_type),
            item.count
        ));
    }
    lines.push(format!(
        "orphan,provisions,{}",
        summary.orphan_counts.provisions
    ));
    lines.push(format!("orphan,chunks,{}", summary.orphan_counts.chunks));
    lines.push(format!(
        "orphan,citations,{}",
        summary.orphan_counts.citations
    ));
    lines.push(format!(
        "duplicate,legal_text_identities,{}",
        summary.duplicate_counts.legal_text_identities
    ));
    lines.push(format!(
        "duplicate,provisions,{}",
        summary.duplicate_counts.provisions
    ));
    lines.push(format!(
        "duplicate,cites_relationships,{}",
        summary.duplicate_counts.cites_relationships
    ));
    lines.push(format!(
        "embedding,coverage,{:.2}",
        summary.embedding_readiness.coverage
    ));
    lines.push(format!(
        "citations,coverage,{:.2}",
        summary.cites_coverage.coverage
    ));
    lines.join("\n")
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn graph_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[derive(Debug, Default)]
struct SearchNodeIdBuckets {
    identity_ids: Vec<String>,
    provision_ids: Vec<String>,
    chunk_ids: Vec<String>,
    definition_ids: Vec<String>,
    semantic_ids: Vec<String>,
    history_ids: Vec<String>,
    specialized_ids: Vec<String>,
    chapter_ids: Vec<String>,
}

impl SearchNodeIdBuckets {
    fn from_generic_ids(ids: impl IntoIterator<Item = String>) -> Self {
        let mut buckets = Self::default();
        for id in ids {
            buckets.push_generic(id);
        }
        buckets.dedup();
        buckets
    }

    fn push_generic(&mut self, id: String) {
        self.identity_ids.push(id.clone());
        self.provision_ids.push(id.clone());
        self.chunk_ids.push(id.clone());
        self.definition_ids.push(id.clone());
        self.semantic_ids.push(id.clone());
        self.history_ids.push(id.clone());
        self.specialized_ids.push(id.clone());
        self.chapter_ids.push(id);
    }

    fn dedup(&mut self) {
        dedup_strings(&mut self.identity_ids);
        dedup_strings(&mut self.provision_ids);
        dedup_strings(&mut self.chunk_ids);
        dedup_strings(&mut self.definition_ids);
        dedup_strings(&mut self.semantic_ids);
        dedup_strings(&mut self.history_ids);
        dedup_strings(&mut self.specialized_ids);
        dedup_strings(&mut self.chapter_ids);
    }
}

fn search_node_id_buckets(results: &[SearchResultModel]) -> SearchNodeIdBuckets {
    let mut buckets = SearchNodeIdBuckets::default();
    for result in results {
        match result.kind.as_str() {
            "statute" | "court_rule" | "legaltextidentity" | "utcrrule" => {
                buckets.identity_ids.push(result.id.clone())
            }
            "provision" | "court_rule_provision" | "utcrprovision" => {
                buckets.provision_ids.push(result.id.clone())
            }
            "chunk" | "retrievalchunk" => buckets.chunk_ids.push(result.id.clone()),
            "definition" | "definedterm" => buckets.definition_ids.push(result.id.clone()),
            "semantic"
            | "legalsemanticnode"
            | "obligation"
            | "exception"
            | "deadline"
            | "penalty"
            | "remedy"
            | "requirednotice"
            | "notice"
            | "proceduralrequirement"
            | "formtext" => buckets.semantic_ids.push(result.id.clone()),
            "sourcenote" | "source_note" | "statusevent" | "status_event" | "temporaleffect"
            | "temporal_effect" | "sessionlaw" | "session_law" | "amendment" | "lineageevent"
            | "lineage_event" => buckets.history_ids.push(result.id.clone()),
            "taxrule" | "tax_rule" | "moneyamount" | "money_amount" | "ratelimit"
            | "rate_limit" | "legalactor" | "legal_actor" | "legalaction" | "legal_action" => {
                buckets.specialized_ids.push(result.id.clone())
            }
            "chapter" | "court_rule_chapter" | "chapterversion" => {
                buckets.chapter_ids.push(result.id.clone())
            }
            _ => buckets.push_generic(result.id.clone()),
        }
    }
    buckets.dedup();
    buckets
}

fn dedup_strings(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn with_search_node_bucket_params(
    q: neo4rs::Query,
    buckets: &SearchNodeIdBuckets,
) -> neo4rs::Query {
    q.param("identity_ids", buckets.identity_ids.clone())
        .param("provision_ids", buckets.provision_ids.clone())
        .param("chunk_ids", buckets.chunk_ids.clone())
        .param("definition_ids", buckets.definition_ids.clone())
        .param("semantic_ids", buckets.semantic_ids.clone())
        .param("history_ids", buckets.history_ids.clone())
        .param("specialized_ids", buckets.specialized_ids.clone())
        .param("chapter_ids", buckets.chapter_ids.clone())
}

fn search_node_resolver_cypher() -> &'static str {
    "CALL {
        UNWIND $identity_ids AS id
        MATCH (n:LegalTextIdentity {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $identity_ids AS id
        MATCH (n:LegalTextVersion {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $identity_ids AS id
        MATCH (n:LegalTextVersion {version_id: id})
        RETURN id, n
        UNION
        UNWIND $provision_ids AS id
        MATCH (n:Provision {provision_id: id})
        RETURN id, n
        UNION
        UNWIND $provision_ids AS id
        MATCH (n:Provision {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $chunk_ids AS id
        MATCH (n:RetrievalChunk {chunk_id: id})
        RETURN id, n
        UNION
        UNWIND $definition_ids AS id
        MATCH (n:Definition {definition_id: id})
        RETURN id, n
        UNION
        UNWIND $definition_ids AS id
        MATCH (n:Definition {source_provision_id: id})
        RETURN id, n
        UNION
        UNWIND $definition_ids AS id
        MATCH (n:DefinedTerm {defined_term_id: id})
        RETURN id, n
        UNION
        UNWIND $definition_ids AS id
        MATCH (n:DefinedTerm {term: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:LegalSemanticNode {semantic_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:LegalSemanticNode {source_provision_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:Obligation {obligation_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:Exception {exception_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:Deadline {deadline_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:Penalty {penalty_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:Remedy {remedy_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:RequiredNotice {required_notice_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:FormText {form_text_id: id})
        RETURN id, n
        UNION
        UNWIND $semantic_ids AS id
        MATCH (n:ProceduralRequirement {requirement_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:SourceNote {source_note_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:SourceNote {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:StatusEvent {status_event_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:StatusEvent {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:TemporalEffect {temporal_effect_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:TemporalEffect {canonical_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:TemporalEffect {source_provision_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:SessionLaw {session_law_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:Amendment {amendment_id: id})
        RETURN id, n
        UNION
        UNWIND $history_ids AS id
        MATCH (n:LineageEvent {lineage_event_id: id})
        RETURN id, n
        UNION
        UNWIND $specialized_ids AS id
        MATCH (n:TaxRule {tax_rule_id: id})
        RETURN id, n
        UNION
        UNWIND $specialized_ids AS id
        MATCH (n:MoneyAmount {money_amount_id: id})
        RETURN id, n
        UNION
        UNWIND $specialized_ids AS id
        MATCH (n:RateLimit {rate_limit_id: id})
        RETURN id, n
        UNION
        UNWIND $specialized_ids AS id
        MATCH (n:LegalActor {actor_id: id})
        RETURN id, n
        UNION
        UNWIND $specialized_ids AS id
        MATCH (n:LegalAction {action_id: id})
        RETURN id, n
        UNION
        UNWIND $chapter_ids AS id
        MATCH (n:ChapterVersion {chapter_id: id})
        RETURN id, n
    }"
}

const SEARCH_NODE_ID_PROPERTIES: &[&str] = &[
    "canonical_id",
    "version_id",
    "provision_id",
    "chunk_id",
    "semantic_id",
    "source_note_id",
    "definition_id",
    "defined_term_id",
    "deadline_id",
    "penalty_id",
    "obligation_id",
    "exception_id",
    "remedy_id",
    "required_notice_id",
    "notice_id",
    "form_text_id",
    "requirement_id",
    "tax_rule_id",
    "money_amount_id",
    "rate_limit_id",
    "actor_id",
    "action_id",
    "mention_id",
    "citation_mention_id",
    "external_citation_id",
    "status_event_id",
    "temporal_effect_id",
    "lineage_event_id",
    "session_law_id",
    "amendment_id",
    "chapter_id",
    "source_provision_id",
    "source_document_id",
    "term",
];

fn search_node_id_expr(alias: &str) -> String {
    let properties = SEARCH_NODE_ID_PROPERTIES
        .iter()
        .map(|property| format!("{alias}.{property}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("coalesce({properties}, elementId({alias}))")
}

fn citation_from_canonical_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("or:ors:") {
        return None;
    }

    let rest = &trimmed["or:ors:".len()..];
    let looks_like_statute = !rest.is_empty()
        && rest.chars().any(|ch| ch.is_ascii_digit())
        && rest
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '(' | ')' | '-'));

    looks_like_statute.then(|| format!("ORS {}", rest.to_ascii_uppercase()))
}

fn semantic_types_for_search_kind(kind: &str) -> Vec<String> {
    match kind.to_ascii_lowercase().as_str() {
        "definition" => vec!["Definition".to_string(), "DefinedTerm".to_string()],
        "definedterm" | "defined_term" => vec!["DefinedTerm".to_string()],
        "obligation" => vec!["Obligation".to_string()],
        "exception" => vec!["Exception".to_string()],
        "deadline" => vec!["Deadline".to_string()],
        "penalty" => vec!["Penalty".to_string()],
        "remedy" => vec!["Remedy".to_string()],
        "requirednotice" | "required_notice" | "notice" => vec!["RequiredNotice".to_string()],
        "proceduralrequirement" | "procedural_requirement" => {
            vec!["ProceduralRequirement".to_string()]
        }
        "sourcenote" | "source_note" => vec!["SourceNote".to_string()],
        "statusevent" | "status_event" => vec!["StatusEvent".to_string()],
        "temporaleffect" | "temporal_effect" => vec!["TemporalEffect".to_string()],
        "sessionlaw" | "session_law" => vec!["SessionLaw".to_string()],
        "amendment" => vec!["Amendment".to_string()],
        "lineageevent" | "lineage_event" => vec!["LineageEvent".to_string()],
        "taxrule" | "tax_rule" => vec!["TaxRule".to_string()],
        "moneyamount" | "money_amount" => vec!["MoneyAmount".to_string()],
        "ratelimit" | "rate_limit" => vec!["RateLimit".to_string()],
        "legalactor" | "legal_actor" => vec!["LegalActor".to_string()],
        "legalaction" | "legal_action" => vec!["LegalAction".to_string()],
        _ => Vec::new(),
    }
}

fn seed_history_semantic_types(
    semantic_types: &mut Vec<String>,
    id: &str,
    kind: &str,
    snippet: &str,
) {
    let kind = kind.to_ascii_lowercase();
    let snippet = snippet.to_ascii_lowercase();
    if id.starts_with("source_note:") || snippet.starts_with("note:") {
        push_semantic_type(semantic_types, "SourceNote");
    }
    if matches!(
        kind.as_str(),
        "sourcenote" | "source_note" | "sessionlaw" | "session_law" | "temporaleffect"
    ) && (snippet.contains("operative")
        || snippet.contains("effective")
        || snippet.contains("become operative")
        || snippet.contains("applies to"))
    {
        push_semantic_type(semantic_types, "TemporalEffect");
    }
}

fn push_semantic_type(semantic_types: &mut Vec<String>, semantic_type: &str) {
    if !semantic_types.iter().any(|value| value == semantic_type) {
        semantic_types.push(semantic_type.to_string());
    }
}

pub(crate) fn sanitize_fulltext_query(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut escaped = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if matches!(
            ch,
            '+' | '-'
                | '&'
                | '|'
                | '!'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '^'
                | '"'
                | '~'
                | '*'
                | '?'
                | ':'
                | '\\'
                | '/'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

pub(crate) fn fulltext_rank_score(rank: usize, source_weight: f32) -> f32 {
    if rank == 0 {
        return 0.0;
    }
    source_weight / (rank as f32).sqrt()
}

pub(crate) fn vector_rank_score(rank: usize, similarity: f32) -> f32 {
    if rank == 0 {
        return similarity.max(0.0);
    }
    similarity.max(0.0) + 0.75 / (rank as f32).sqrt()
}

fn safe_vector_index_name(index_name: &str) -> ApiResult<&str> {
    if index_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(index_name)
    } else {
        Err(ApiError::BadRequest(format!(
            "Invalid vector index name {index_name}"
        )))
    }
}

#[cfg(test)]
mod search_support_tests {
    use super::*;

    #[test]
    fn sanitizes_lucene_special_characters_without_touching_legal_words() {
        assert_eq!(
            sanitize_fulltext_query(r#"ORS 90.300(1) "deposit" + fee"#),
            r#"ORS 90.300\(1\) \"deposit\" \+ fee"#
        );
        assert_eq!(
            sanitize_fulltext_query("landlord notice"),
            "landlord notice"
        );
    }

    #[test]
    fn source_rank_scores_decay_by_rank() {
        let first = fulltext_rank_score(1, 1.0);
        let fourth = fulltext_rank_score(4, 1.0);
        assert!(first > fourth);
        assert!((fourth - 0.5).abs() < 0.001);

        let vector_first = vector_rank_score(1, 0.72);
        let vector_tenth = vector_rank_score(10, 0.72);
        assert!(vector_first > vector_tenth);
    }

    #[test]
    fn rejects_unsafe_dynamic_vector_index_names() {
        assert!(safe_vector_index_name("retrieval_chunk_embedding_1024").is_ok());
        assert!(safe_vector_index_name("retrieval`) MATCH (n) //").is_err());
    }

    #[test]
    fn search_id_helpers_cover_specialized_fulltext_nodes() {
        let expr = search_node_id_expr("node");
        for property in [
            "node.definition_id",
            "node.defined_term_id",
            "node.obligation_id",
            "node.requirement_id",
            "node.required_notice_id",
            "node.form_text_id",
            "node.source_provision_id",
        ] {
            assert!(
                expr.contains(property),
                "ID expression should include {property}"
            );
        }
        assert!(expr.contains("elementId(node)"));

        let resolver = search_node_resolver_cypher();
        for branch in [
            "MATCH (n:Definition {definition_id: id})",
            "MATCH (n:DefinedTerm {defined_term_id: id})",
            "MATCH (n:LegalSemanticNode {semantic_id: id})",
            "MATCH (n:Obligation {obligation_id: id})",
            "MATCH (n:ProceduralRequirement {requirement_id: id})",
            "MATCH (n:RequiredNotice {required_notice_id: id})",
            "MATCH (n:FormText {form_text_id: id})",
            "MATCH (n:TemporalEffect {canonical_id: id})",
        ] {
            assert!(
                resolver.contains(branch),
                "resolver should include {branch}"
            );
        }
        assert!(
            !resolver.contains("MATCH (n)\n"),
            "resolver should avoid label-free graph scans"
        );
    }

    #[test]
    fn derives_ors_citation_from_canonical_id() {
        assert_eq!(
            citation_from_canonical_id("or:ors:90.300"),
            Some("ORS 90.300".to_string())
        );
        assert_eq!(
            citation_from_canonical_id("OR:ORS:90.320(1)(a)"),
            Some("ORS 90.320(1)(A)".to_string())
        );
        assert_eq!(citation_from_canonical_id("or:utcr:2.010"), None);
        assert_eq!(citation_from_canonical_id("or:ors:"), None);
    }

    #[test]
    fn seeds_semantic_types_from_search_kind() {
        assert_eq!(
            semantic_types_for_search_kind("definition"),
            vec!["Definition".to_string(), "DefinedTerm".to_string()]
        );
        assert_eq!(
            semantic_types_for_search_kind("temporaleffect"),
            vec!["TemporalEffect".to_string()]
        );
        assert_eq!(
            semantic_types_for_search_kind("legal_actor"),
            vec!["LegalActor".to_string()]
        );

        let mut history_types = semantic_types_for_search_kind("sessionlaw");
        seed_history_semantic_types(
            &mut history_types,
            "source_note:abc",
            "sessionlaw",
            "Note: The amendments become operative January 1, 2027.",
        );
        assert!(history_types.contains(&"SessionLaw".to_string()));
        assert!(history_types.contains(&"SourceNote".to_string()));
        assert!(history_types.contains(&"TemporalEffect".to_string()));
    }

    #[test]
    fn graph_relationship_modes_cover_typed_citations_and_similarity() {
        let citation = graph_relationship_types("citation", None);
        for expected in [
            "CITES",
            "CITES_VERSION",
            "CITES_PROVISION",
            "CITES_CHAPTER",
            "CITES_RANGE",
            "RESOLVES_TO_CHAPTER",
            "RESOLVES_TO_EXTERNAL",
        ] {
            assert!(
                citation.iter().any(|value| value == expected),
                "citation mode should include {expected}"
            );
        }

        let hybrid = graph_relationship_types("hybrid", None);
        assert!(
            hybrid.iter().any(|value| value == "SIMILAR_TO"),
            "hybrid mode should include similarity edges"
        );

        let similarity = graph_relationship_types("embedding_similarity", None);
        assert_eq!(similarity, vec!["SIMILAR_TO".to_string()]);
    }
}

fn graph_relationship_types(mode: &str, override_value: Option<&str>) -> Vec<String> {
    let explicit = graph_csv(override_value);
    if !explicit.is_empty() {
        return explicit;
    }

    let values: &[&str] = match mode {
        "citation" => &[
            "CITES",
            "CITES_VERSION",
            "CITES_PROVISION",
            "CITES_CHAPTER",
            "CITES_RANGE",
            "MENTIONS_CITATION",
            "RESOLVES_TO",
            "RESOLVES_TO_VERSION",
            "RESOLVES_TO_PROVISION",
            "RESOLVES_TO_CHAPTER",
            "RESOLVES_TO_EXTERNAL",
            "RESOLVES_TO_RANGE_START",
            "RESOLVES_TO_RANGE_END",
            "CITES_EXTERNAL",
        ],
        "embedding_similarity" | "similarity" => &["SIMILAR_TO"],
        "semantic" => &[
            "EXPRESSES",
            "SUPPORTED_BY",
            "IMPOSED_ON",
            "REQUIRES_ACTION",
            "HAS_DEADLINE",
            "SUBJECT_TO",
            "VIOLATION_PENALIZED_BY",
            "EXCEPTION_TO",
            "REQUIRES_NOTICE",
            "DEFINES",
            "HAS_SCOPE",
        ],
        "history" => &[
            "HAS_STATUS_EVENT",
            "HAS_TEMPORAL_EFFECT",
            "HAS_LINEAGE_EVENT",
            "FORMERLY",
            "RENUMBERED_TO",
            "REPEALED_BY",
            "MENTIONS_SESSION_LAW",
            "ENACTS",
            "AFFECTS",
            "AFFECTS_VERSION",
            "HAS_SOURCE_NOTE",
        ],
        "hybrid" => &[
            "CITES",
            "CITES_VERSION",
            "CITES_PROVISION",
            "CITES_CHAPTER",
            "CITES_RANGE",
            "MENTIONS_CITATION",
            "RESOLVES_TO",
            "RESOLVES_TO_VERSION",
            "RESOLVES_TO_PROVISION",
            "RESOLVES_TO_CHAPTER",
            "RESOLVES_TO_EXTERNAL",
            "RESOLVES_TO_RANGE_START",
            "RESOLVES_TO_RANGE_END",
            "CITES_EXTERNAL",
            "SIMILAR_TO",
            "HAS_VERSION",
            "VERSION_OF",
            "CONTAINS",
            "HAS_PARENT",
            "NEXT",
            "PREVIOUS",
            "EXPRESSES",
            "SUPPORTED_BY",
            "DEFINES",
            "HAS_SCOPE",
            "HAS_DEADLINE",
            "EXCEPTION_TO",
            "REQUIRES_NOTICE",
            "HAS_SOURCE_NOTE",
            "HAS_TEMPORAL_EFFECT",
            "ENACTS",
            "AFFECTS",
        ],
        _ => &[
            "CITES",
            "CITES_VERSION",
            "CITES_PROVISION",
            "CITES_CHAPTER",
            "CITES_RANGE",
            "CITES_EXTERNAL",
            "HAS_VERSION",
            "VERSION_OF",
            "CONTAINS",
            "PART_OF_VERSION",
            "HAS_PARENT",
            "NEXT",
            "PREVIOUS",
            "EXPRESSES",
            "SUPPORTED_BY",
            "DEFINES",
            "HAS_SCOPE",
            "HAS_DEADLINE",
            "EXCEPTION_TO",
            "REQUIRES_NOTICE",
            "HAS_SOURCE_NOTE",
            "HAS_TEMPORAL_EFFECT",
            "ENACTS",
            "AFFECTS",
        ],
    };

    values.iter().map(|value| value.to_string()).collect()
}

fn graph_edge_kind(edge_type: &str) -> &'static str {
    match edge_type {
        "SIMILAR_TO" => "semantic_similarity",
        "DERIVED_FROM" | "SUPPORTED_BY" | "HAS_SOURCE_NOTE" => "provenance",
        "HAS_STATUS_EVENT"
        | "HAS_TEMPORAL_EFFECT"
        | "HAS_LINEAGE_EVENT"
        | "FORMERLY"
        | "RENUMBERED_TO"
        | "REPEALED_BY"
        | "MENTIONS_SESSION_LAW"
        | "ENACTS"
        | "AFFECTS"
        | "AFFECTS_VERSION" => "history",
        "PART_OF_CHUNK" | "HAS_CHUNK" | "CHUNK_OF" => "retrieval",
        _ => "legal",
    }
}

fn graph_edge_style(edge_type: &str) -> GraphEdgeStyle {
    let (dashed, width, color) = match edge_type {
        "SIMILAR_TO" => (true, 1.4, "#22d3ee"),
        "CITES"
        | "CITES_VERSION"
        | "CITES_PROVISION"
        | "CITES_CHAPTER"
        | "CITES_RANGE"
        | "CITES_EXTERNAL"
        | "MENTIONS_CITATION"
        | "RESOLVES_TO"
        | "RESOLVES_TO_VERSION"
        | "RESOLVES_TO_PROVISION"
        | "RESOLVES_TO_CHAPTER"
        | "RESOLVES_TO_EXTERNAL"
        | "RESOLVES_TO_RANGE_START"
        | "RESOLVES_TO_RANGE_END" => (false, 1.5, "#60a5fa"),
        "EXPRESSES" | "SUPPORTED_BY" | "IMPOSED_ON" | "REQUIRES_ACTION" => (false, 1.35, "#34d399"),
        "DEFINES" | "HAS_SCOPE" => (false, 1.25, "#a78bfa"),
        "HAS_TEMPORAL_EFFECT" | "HAS_STATUS_EVENT" | "AFFECTS" | "AFFECTS_VERSION" => {
            (false, 1.25, "#f59e0b")
        }
        "EXCEPTION_TO" | "REPEALED_BY" => (false, 1.25, "#ef4444"),
        _ => (false, 1.0, "#94a3b8"),
    };

    GraphEdgeStyle {
        dashed,
        width,
        color: color.to_string(),
    }
}

fn graph_node_href(id: &str, node_type: &str) -> Option<String> {
    match node_type {
        "LegalTextIdentity" => Some(format!("/statutes/{id}")),
        "Provision" => Some(format!("/provisions/{id}")),
        _ => None,
    }
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut preview = trimmed.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_string()
}

fn json_string_or(value: &serde_json::Value, key: &str, fallback: &str) -> String {
    value[key].as_str().unwrap_or(fallback).to_string()
}

fn json_to_citations(values: Vec<serde_json::Value>) -> Vec<Citation> {
    values
        .into_iter()
        .filter_map(|value| {
            let target_citation = json_string(&value, "target_citation");
            let source_provision = json_string(&value, "source_provision");
            if target_citation.is_empty() && source_provision.is_empty() {
                return None;
            }

            Some(Citation {
                target_canonical_id: value["target_canonical_id"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string()),
                target_citation,
                context_snippet: json_string(&value, "context_snippet"),
                source_provision,
                resolved: value["resolved"].as_bool().unwrap_or(true),
            })
        })
        .collect()
}

fn json_to_semantic_items(values: Vec<serde_json::Value>) -> Vec<SemanticItem> {
    values
        .into_iter()
        .filter_map(|value| {
            let text = json_string(&value, "text");
            if text.is_empty() {
                return None;
            }

            Some(SemanticItem {
                text,
                source_provision: json_string(&value, "source_provision"),
            })
        })
        .collect()
}

fn json_to_deadlines(values: Vec<serde_json::Value>) -> Vec<DeadlineItem> {
    values
        .into_iter()
        .filter_map(|value| {
            let description = json_string(&value, "description");
            if description.is_empty() {
                return None;
            }

            Some(DeadlineItem {
                description,
                duration: json_string(&value, "duration"),
                trigger: json_string(&value, "trigger"),
                source_provision: json_string(&value, "source_provision"),
            })
        })
        .collect()
}

fn json_to_definitions(values: Vec<serde_json::Value>) -> Vec<DefinitionItem> {
    values
        .into_iter()
        .filter_map(|value| {
            let term = json_string(&value, "term");
            let text = json_string(&value, "text");
            if term.is_empty() && text.is_empty() {
                return None;
            }

            Some(DefinitionItem {
                term,
                text,
                source_provision: json_string(&value, "source_provision"),
                scope: json_string(&value, "scope"),
            })
        })
        .collect()
}

fn json_to_provision_exceptions(values: Vec<serde_json::Value>) -> Vec<ProvisionException> {
    values
        .into_iter()
        .filter_map(|value| {
            let exception_id = json_string(&value, "exception_id");
            let text = json_string(&value, "text");
            if exception_id.is_empty() && text.is_empty() {
                return None;
            }

            Some(ProvisionException {
                exception_id,
                text,
                applies_to_provision: json_string(&value, "applies_to_provision"),
                source_provision: json_string(&value, "source_provision"),
            })
        })
        .collect()
}

fn json_to_qc_notes(values: Vec<serde_json::Value>) -> Vec<QCNoteItem> {
    values
        .into_iter()
        .filter_map(|value| {
            let note_id = json_string(&value, "note_id");
            let message = json_string(&value, "message");
            if note_id.is_empty() && message.is_empty() {
                return None;
            }

            Some(QCNoteItem {
                note_id,
                level: json_string_or(&value, "level", "info"),
                category: json_string_or(&value, "category", "source"),
                message,
                related_id: value["related_id"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string()),
            })
        })
        .collect()
}

fn json_to_chunks(values: Vec<serde_json::Value>, fallback_source_id: &str) -> Vec<ProvisionChunk> {
    values
        .into_iter()
        .filter(|chunk| !json_string(chunk, "chunk_id").is_empty())
        .map(|chunk| ProvisionChunk {
            chunk_id: json_string(&chunk, "chunk_id"),
            chunk_type: json_string_or(&chunk, "chunk_type", "contextual_provision"),
            source_kind: json_string_or(&chunk, "source_kind", "provision"),
            source_id: json_string_or(&chunk, "source_id", fallback_source_id),
            text: json_string(&chunk, "text"),
            embedding_policy: json_string_or(&chunk, "embedding_policy", "primary"),
            answer_policy: json_string_or(&chunk, "answer_policy", "supporting"),
            search_weight: chunk["search_weight"].as_f64().unwrap_or(1.0),
            embedded: chunk["embedding"].is_array() || chunk["embedded"].as_bool().unwrap_or(false),
            parser_confidence: chunk["parser_confidence"].as_f64().unwrap_or(1.0),
        })
        .collect()
}

fn json_to_provision_links(values: Vec<serde_json::Value>) -> Vec<ProvisionLink> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();

    for value in values {
        if let Some(items) = value.as_array() {
            for item in items {
                push_provision_link(item, &mut links, &mut seen);
            }
        } else {
            push_provision_link(&value, &mut links, &mut seen);
        }
    }

    links
}

fn push_provision_link(
    value: &serde_json::Value,
    links: &mut Vec<ProvisionLink>,
    seen: &mut HashSet<String>,
) {
    let provision_id = json_string(value, "provision_id");
    if provision_id.is_empty() || !seen.insert(provision_id.clone()) {
        return;
    }

    links.push(ProvisionLink {
        provision_id,
        citation: json_string(value, "citation"),
    });
}

fn json_to_amendments(values: Vec<serde_json::Value>) -> Vec<Amendment> {
    values
        .into_iter()
        .filter_map(|value| {
            let amendment_id = json_string(&value, "amendment_id");
            if amendment_id.is_empty() {
                return None;
            }

            Some(Amendment {
                amendment_id,
                description: json_string(&value, "description"),
                effective_date: json_string(&value, "effective_date"),
            })
        })
        .collect()
}

fn json_to_session_laws(values: Vec<serde_json::Value>) -> Vec<SessionLaw> {
    values
        .into_iter()
        .filter_map(|value| {
            let session_law_id = json_string(&value, "session_law_id");
            if session_law_id.is_empty() {
                return None;
            }

            Some(SessionLaw {
                session_law_id,
                citation: json_string(&value, "citation"),
                description: json_string(&value, "description"),
            })
        })
        .collect()
}

fn json_to_status_events(values: Vec<serde_json::Value>) -> Vec<StatusEvent> {
    values
        .into_iter()
        .filter_map(|value| {
            let event_id = json_string(&value, "event_id");
            if event_id.is_empty() {
                return None;
            }

            Some(StatusEvent {
                event_id,
                event_type: json_string_or(&value, "event_type", "status"),
                date: json_string(&value, "date"),
                description: json_string(&value, "description"),
            })
        })
        .collect()
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

fn normalized_filter(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty() && value != "all")
}

fn nest_provisions(mut provisions: Vec<ProvisionNode>) -> Vec<ProvisionNode> {
    provisions.sort_by_key(|provision| provision.local_path.len());

    let mut roots = Vec::new();
    for mut provision in provisions {
        provision.children = vec![];
        if provision.local_path.len() <= 1 {
            roots.push(provision);
            continue;
        }

        let parent_path = provision.local_path[..provision.local_path.len() - 1].to_vec();
        if let Some(parent) = find_provision_by_path_mut(&mut roots, &parent_path) {
            parent.children.push(provision);
        } else {
            roots.push(provision);
        }
    }

    roots
}

fn find_provision_by_path_mut<'a>(
    provisions: &'a mut [ProvisionNode],
    path: &[String],
) -> Option<&'a mut ProvisionNode> {
    for provision in provisions {
        if provision.local_path == path {
            return Some(provision);
        }
        if let Some(child) = find_provision_by_path_mut(&mut provision.children, path) {
            return Some(child);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn citation_mapping_filters_blank_optional_matches() {
        let citations = json_to_citations(vec![
            json!({
                "target_canonical_id": null,
                "target_citation": "",
                "context_snippet": "",
                "source_provision": "",
                "resolved": true
            }),
            json!({
                "target_canonical_id": "or:ors:90.300",
                "target_citation": "ORS 90.300",
                "context_snippet": "ORS 90.300",
                "source_provision": "ORS 90.100(1)",
                "resolved": true
            }),
        ]);

        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].target_citation, "ORS 90.300");
        assert!(citations[0].resolved);
    }

    #[test]
    fn unresolved_citation_mapping_preserves_raw_text() {
        let citations = json_to_citations(vec![json!({
            "target_canonical_id": null,
            "target_citation": "ORS 999.999",
            "context_snippet": "ORS 999.999",
            "source_provision": "ORS 90.100(2)",
            "resolved": false
        })]);

        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].target_canonical_id, None);
        assert!(!citations[0].resolved);
    }

    #[test]
    fn semantic_mapping_filters_empty_nodes() {
        let items = json_to_semantic_items(vec![
            json!({"text": "", "source_provision": ""}),
            json!({"text": "The tenant shall pay rent.", "source_provision": "ORS 90.100(3)"}),
        ]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_provision, "ORS 90.100(3)");
    }

    #[test]
    fn provision_link_mapping_flattens_ancestor_groups_and_dedupes() {
        let links = json_to_provision_links(vec![json!([
            {"provision_id": "p1", "citation": "ORS 1.001(1)"},
            {"provision_id": "p2", "citation": "ORS 1.001(2)"},
            {"provision_id": "p1", "citation": "ORS 1.001(1)"}
        ])]);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].provision_id, "p1");
        assert_eq!(links[1].provision_id, "p2");
    }

    #[test]
    fn provision_tree_nests_by_local_path() {
        let tree = nest_provisions(vec![
            provision_node("p1", "ORS 1.001(1)", &["1"]),
            provision_node("p1a", "ORS 1.001(1)(a)", &["1", "a"]),
            provision_node("p1ai", "ORS 1.001(1)(a)(A)", &["1", "a", "A"]),
            provision_node("p2", "ORS 1.001(2)", &["2"]),
        ]);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].provision_id, "p1");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].provision_id, "p1a");
        assert_eq!(tree[0].children[0].children[0].provision_id, "p1ai");
        assert_eq!(tree[1].provision_id, "p2");
    }

    #[test]
    fn normalized_filter_trims_all_and_lowercases() {
        assert_eq!(
            normalized_filter(Some("  Active ")),
            Some("active".to_string())
        );
        assert_eq!(normalized_filter(Some("all")), None);
        assert_eq!(normalized_filter(Some("  ")), None);
        assert_eq!(normalized_filter(None), None);
    }

    fn provision_node(id: &str, citation: &str, path: &[&str]) -> ProvisionNode {
        ProvisionNode {
            provision_id: id.to_string(),
            display_citation: citation.to_string(),
            local_path: path.iter().map(|value| value.to_string()).collect(),
            depth: path.len(),
            text: citation.to_string(),
            children: vec![],
        }
    }
}
