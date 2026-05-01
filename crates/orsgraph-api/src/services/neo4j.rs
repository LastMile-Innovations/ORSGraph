use crate::error::{ApiError, ApiResult};
use crate::models::api::*;
use crate::models::search::{SearchResult as SearchResultModel, *};
use neo4rs::{query, Graph, Row};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
            "CREATE INDEX legal_identity_canonical IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.canonical_id)",
            "CREATE INDEX legal_version_id IF NOT EXISTS FOR (n:LegalTextVersion) ON (n.version_id)",
            "CREATE INDEX provision_display_citation IF NOT EXISTS FOR (n:Provision) ON (n.display_citation)",
            "CREATE INDEX provision_id IF NOT EXISTS FOR (n:Provision) ON (n.provision_id)",
            "CREATE FULLTEXT INDEX statute_fulltext IF NOT EXISTS FOR (n:LegalTextIdentity|LegalTextVersion) ON EACH [n.citation, n.title, n.text]",
            "CREATE FULLTEXT INDEX provision_fulltext IF NOT EXISTS FOR (n:Provision) ON EACH [n.display_citation, n.text, n.normalized_text]",
            "CREATE FULLTEXT INDEX definition_fulltext IF NOT EXISTS FOR (n:Definition|DefinedTerm) ON EACH [n.term, n.normalized_term, n.definition_text]",
            "CREATE FULLTEXT INDEX semantic_fulltext IF NOT EXISTS FOR (n:LegalSemanticNode|Obligation|Exception|Deadline|Penalty|Remedy|RequiredNotice|FormText) ON EACH [n.text, n.normalized_text, n.actor_text, n.action_text, n.object_text, n.trigger_event]",
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

    pub async fn search_exact(&self, citation: &str) -> ApiResult<Vec<SearchResultModel>> {
        let mut results = Vec::new();

        // Statute lookup
        let mut statute_res = self
            .graph
            .execute(
                query(
                    "MATCH (n:LegalTextIdentity) 
                   WHERE toUpper(n.citation) = toUpper($c) OR n.canonical_id = $c
                   RETURN n.canonical_id as id, n.citation as citation, n.title as title, 
                          n.chapter as chapter, n.status as status, labels(n)[0] as kind,
                          n.title as text",
                )
                .param("c", citation),
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
                   WHERE toUpper(n.display_citation) = toUpper($c)
                   RETURN n.provision_id as id, n.display_citation as citation, null as title, 
                          n.chapter as chapter, n.status as status, labels(n)[0] as kind,
                          n.text as text",
                )
                .param("c", citation),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        while let Some(row) = prov_res.next().await.map_err(ApiError::Neo4jConnection)? {
            results.push(self.row_to_search_result(row, 4.0)?);
        }

        // Chapter lookup
        let chapter_number = citation
            .strip_prefix("Chapter ")
            .or_else(|| citation.strip_prefix("chapter "));
        if let Some(chapter) = chapter_number {
            let mut chapter_res = self
                .graph
                .execute(
                    query(
                        "MATCH (n:ChapterVersion)
                         WHERE n.chapter = $chapter OR n.chapter_number = $chapter
                         RETURN n.chapter_id as id, 'Chapter ' + n.chapter as citation, n.title as title,
                                n.chapter as chapter, null as status, 'chapter' as kind,
                                coalesce(n.title, n.summary, n.chapter) as text
                         LIMIT 5",
                    )
                    .param("chapter", chapter),
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

    pub async fn search_fulltext(
        &self,
        q: &str,
        result_type: Option<&str>,
        limit: u32,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let mut results = Vec::new();
        let limit = limit as i64;

        let queries = match result_type {
            Some("statute") => vec![(
                "statute_fulltext", 
                "MATCH (node) 
                 OPTIONAL MATCH (node)-[:HAS_VERSION]->(v:LegalTextVersion)
                 WITH coalesce(node, v) as n, score
                 RETURN n.canonical_id as id, n.citation as citation, n.title as title, 
                        n.chapter as chapter, n.status as status, 'statute' as kind, score,
                        coalesce(n.text, n.title) as text"
            )],
            Some("provision") => vec![(
                "provision_fulltext",
                "MATCH (node)
                 RETURN node.provision_id as id, node.display_citation as citation, null as title,
                        node.chapter as chapter, node.status as status, 'provision' as kind, score,
                        node.text as text"
            )],
            Some("definition") => vec![(
                "definition_fulltext",
                "MATCH (node)
                 RETURN coalesce(node.term, node.definition_id) as id, node.term as citation, null as title,
                        null as chapter, null as status, 'definition' as kind, score,
                        coalesce(node.definition_text, node.term) as text"
            )],
            Some("semantic") | Some("obligation") | Some("deadline") | Some("penalty") | Some("notice") => vec![(
                "semantic_fulltext",
                "MATCH (node)
                 RETURN coalesce(node.semantic_id, node.provision_id) as id, node.citation as citation, null as title,
                        node.chapter as chapter, null as status, labels(node)[0] as kind, score,
                        node.text as text"
            )],
            Some("history") => vec![(
                "history_fulltext",
                "MATCH (node)
                 RETURN coalesce(node.source_note_id, node.version_id) as id, node.citation as citation, null as title,
                        null as chapter, null as status, labels(node)[0] as kind, score,
                        coalesce(node.text, node.raw_text) as text"
            )],
            Some("chunk") => vec![(
                "chunk_fulltext",
                "MATCH (node)
                 RETURN node.chunk_id as id, node.citation as citation, null as title,
                        null as chapter, null as status, 'chunk' as kind, score,
                        node.text as text"
            )],
            Some("actor") => vec![(
                "actor_action_fulltext",
                "MATCH (node)
                 RETURN coalesce(node.actor_id, node.action_id) as id, null as citation, null as title,
                        null as chapter, null as status, labels(node)[0] as kind, score,
                        coalesce(node.actor_text, node.object_text) as text"
            )],
            Some("taxrule") | Some("moneyamount") | Some("ratelimit") | Some("legalaction") | Some("legalactor") => vec![(
                "specialized_legal_fulltext",
                "MATCH (node)
                 RETURN coalesce(node.tax_rule_id, node.money_amount_id, node.rate_limit_id, node.action_id, node.actor_id) as id,
                        node.citation as citation, null as title, node.chapter as chapter, null as status,
                        labels(node)[0] as kind, score,
                        coalesce(node.text, node.normalized_text, node.actor_text, node.action_text, node.object_text, node.tax_type, node.rate_type, node.amount_type) as text"
            )],
            _ => vec![
                ("statute_fulltext", "MATCH (node) RETURN node.canonical_id as id, node.citation as citation, node.title as title, node.chapter as chapter, node.status as status, 'statute' as kind, score, coalesce(node.text, node.title) as text"),
                ("provision_fulltext", "MATCH (node) RETURN node.provision_id as id, node.display_citation as citation, null as title, node.chapter as chapter, node.status as status, 'provision' as kind, score, node.text as text"),
                ("definition_fulltext", "MATCH (node) RETURN coalesce(node.term, node.definition_id) as id, node.term as citation, null as title, null as chapter, null as status, 'definition' as kind, score, coalesce(node.definition_text, node.term) as text"),
                ("semantic_fulltext", "MATCH (node) RETURN coalesce(node.semantic_id, node.provision_id) as id, node.citation as citation, null as title, node.chapter as chapter, null as status, labels(node)[0] as kind, score, node.text as text"),
                ("specialized_legal_fulltext", "MATCH (node) RETURN coalesce(node.tax_rule_id, node.money_amount_id, node.rate_limit_id, node.action_id, node.actor_id) as id, node.citation as citation, null as title, node.chapter as chapter, null as status, labels(node)[0] as kind, score, coalesce(node.text, node.normalized_text, node.actor_text, node.action_text, node.object_text, node.tax_type, node.rate_type, node.amount_type) as text"),
                ("history_fulltext", "MATCH (node) RETURN coalesce(node.source_note_id, node.version_id) as id, node.citation as citation, null as title, null as chapter, null as status, labels(node)[0] as kind, score, coalesce(node.text, node.raw_text) as text"),
                ("chunk_fulltext", "MATCH (node) RETURN node.chunk_id as id, node.citation as citation, null as title, node.chapter as chapter, null as status, 'chunk' as kind, score, node.text as text"),
            ]
        };

        for (index_name, return_clause) in queries {
            let cypher = format!(
                "CALL db.index.fulltext.queryNodes($index, $q) YIELD node, score 
                 {} 
                 LIMIT $limit",
                return_clause
            );

            let mut res = self
                .graph
                .execute(
                    query(&cypher)
                        .param("index", index_name)
                        .param("q", q)
                        .param("limit", limit),
                )
                .await
                .map_err(ApiError::Neo4jConnection)?;

            while let Some(row) = res.next().await.map_err(ApiError::Neo4jConnection)? {
                let score = row.get::<f64>("score").unwrap_or(1.0) as f32;
                let mut result = self.row_to_search_result(row, score)?;
                result.fulltext_score = Some(score);
                result.score_breakdown = Some(ScoreBreakdown {
                    exact: None,
                    keyword: Some(score),
                    vector: None,
                    rerank: None,
                    graph: None,
                    authority: None,
                    penalties: None,
                });
                results.push(result);
            }
        }

        Ok(results)
    }

    pub async fn search_vector_chunks(
        &self,
        index_name: &str,
        embedding: Vec<f32>,
        top_k: usize,
        min_score: f32,
        limit: usize,
    ) -> ApiResult<Vec<SearchResultModel>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "CALL db.index.vector.queryNodes($index_name, $top_k, $embedding)
                     YIELD node, score
                     WHERE node:RetrievalChunk AND score >= $min_score
                     MATCH (node)-[:DERIVED_FROM]->(source)
                     OPTIONAL MATCH (source:Provision)-[:PART_OF_VERSION]->(v:LegalTextVersion)-[:VERSION_OF]->(id:LegalTextIdentity)
                     OPTIONAL MATCH (source:LegalTextVersion)-[:VERSION_OF]->(id2:LegalTextIdentity)
                     WITH node, source, score, coalesce(id, id2) AS identity, v
                     RETURN
                       coalesce(source.provision_id, identity.canonical_id, node.chunk_id) as id,
                       CASE
                         WHEN source:Provision THEN 'provision'
                         WHEN source:LegalTextVersion THEN 'statute'
                         ELSE 'chunk'
                       END as kind,
                       coalesce(source.display_citation, identity.citation, node.citation) as citation,
                       identity.title as title,
                       coalesce(source.chapter, identity.chapter, node.chapter) as chapter,
                       coalesce(source.status, identity.status, 'active') as status,
                       coalesce(source.text, node.text) as text,
                       node.chunk_id as chunk_id,
                       source.provision_id as provision_id,
                       coalesce(v.version_id, source.version_id, node.parent_version_id, node.source_version_id) as version_id,
                       score
                     ORDER BY score DESC
                     LIMIT $limit",
                )
                .param("index_name", index_name)
                .param("top_k", top_k as i64)
                .param("embedding", embedding)
                .param("min_score", min_score as f64)
                .param("limit", limit as i64),
            )
            .await
            .map_err(ApiError::Neo4jConnection)?;

        let mut results = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            let score = row.get::<f64>("score").unwrap_or(0.0) as f32;
            let mut search_result = self.row_to_search_result(row, score)?;
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

        let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
        let mut rows = self
            .graph
            .execute(
                query(
                    "UNWIND $ids as id
                     MATCH (n)
                     WHERE n.canonical_id = id
                        OR n.provision_id = id
                        OR n.version_id = id
                        OR n.semantic_id = id
                        OR n.definition_id = id
                        OR n.deadline_id = id
                        OR n.penalty_id = id
                        OR n.source_note_id = id
                        OR n.chunk_id = id
                     OPTIONAL MATCH (n:RetrievalChunk)-[:DERIVED_FROM]->(chunk_source)
                     WITH id, n, coalesce(chunk_source, n) AS source
                     OPTIONAL MATCH (source:Provision)-[:PART_OF_VERSION]->(v:LegalTextVersion)-[:VERSION_OF]->(identity:LegalTextIdentity)
                     OPTIONAL MATCH (source:LegalTextVersion)-[:VERSION_OF]->(identity2:LegalTextIdentity)
                     WITH id, n, source, coalesce(v, source) AS version, coalesce(identity, identity2) AS identity
                     OPTIONAL MATCH (source)-[:EXPRESSES]->(sem)
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
                          collect(DISTINCT coalesce(sem.semantic_type, labels(sem)[0])) AS semantic_types,
                          count(DISTINCT sem) AS semantic_count,
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
                )
                .param("ids", ids),
            )
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

        let mut result = self.graph.execute(
            query("UNWIND $ids as id
                   MATCH (n) WHERE n.canonical_id = id OR n.provision_id = id OR n.semantic_id = id OR n.chunk_id = id
                   OPTIONAL MATCH (n)-[:CITES]->(out)
                   OPTIONAL MATCH (in)-[:CITES]->(n)
                   OPTIONAL MATCH (n)-[:HAS_SEMANTIC_NODE|DEFINED_BY]-(s)
                   RETURN id, count(DISTINCT out) as outbound, count(DISTINCT in) as inbound, count(DISTINCT s) as semantic")
            .param("ids", ids.to_vec())
        ).await.map_err(ApiError::Neo4jConnection)?;

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
        let citation = row.get::<String>("citation").ok();
        let title: Option<String> = row.get("title").ok();

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

        let href = match kind.as_str() {
            "statute" | "legaltextidentity" => {
                format!("/statutes/{}", citation.as_deref().unwrap_or(&id))
            }
            "provision" => format!(
                "/statutes/{}?provision={}",
                citation.as_deref().unwrap_or(&id),
                id
            ),
            _ => format!("/search?q={}", id),
        };

        Ok(SearchResultModel {
            id,
            kind,
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
            semantic_types: vec![],
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
        chapter: Option<&str>,
    ) -> ApiResult<StatuteIndexResponse> {
        let limit = limit.unwrap_or(250).clamp(1, 1000);
        let offset = offset.unwrap_or(0);
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE $chapter IS NULL OR i.chapter = $chapter
                     WITH i
                     ORDER BY i.chapter, i.citation
                     SKIP $offset
                     LIMIT $limit
                     RETURN collect({
                       canonical_id: i.canonical_id,
                       citation: i.citation,
                       title: i.title,
                       chapter: i.chapter,
                       status: coalesce(i.status, 'active'),
                       edition_year: coalesce(i.edition_year, 2025)
                     }) as items,
                     count(*) as page_count",
                )
                .param("chapter", chapter.map(|value| value.to_string()))
                .param("offset", offset as i64)
                .param("limit", limit as i64),
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
            total: items.len() as u64,
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
                     OPTIONAL MATCH (i)-[:HAS_CURRENT_VERSION]->(v:LegalTextVersion)
                     OPTIONAL MATCH (v)-[:FROM_SOURCE]->(s:SourceDocument)
                     OPTIONAL MATCH (i)<-[:BELONGS_TO]-(p:Provision)
                     RETURN i.canonical_id as canonical_id, i.citation as citation, i.title as title,
                            i.chapter as chapter, i.status as status,
                            v.version_id as version_id, v.effective_date as effective_date,
                            v.end_date as end_date, v.is_current as is_current, v.text as text,
                            s.source_document_id as source_id, s.url as url, s.edition_year as edition_year,
                            count(p) as provision_count"
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
                outbound: 0,
                inbound: 0,
            },
            semantic_counts: SemanticCounts {
                obligations: 0,
                exceptions: 0,
                deadlines: 0,
                penalties: 0,
                definitions: 0,
            },
            source_notes: vec![],
        })
    }

    pub async fn get_provisions(&self, citation: &str) -> ApiResult<ProvisionsResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)<-[:BELONGS_TO]-(p:Provision)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
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
            provisions,
        })
    }

    pub async fn get_citations(&self, citation: &str) -> ApiResult<CitationsResponse> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (i:LegalTextIdentity)
                     WHERE i.citation = $citation OR i.canonical_id = $citation
                     OPTIONAL MATCH (i)-[:CITES]->(t:LegalTextIdentity)
                     OPTIONAL MATCH (s:LegalTextIdentity)-[:CITES]->(i)
                     RETURN i.citation as citation,
                            collect(DISTINCT {target_canonical_id: t.canonical_id, target_citation: t.citation,
                                             context_snippet: '', source_provision: i.citation, resolved: true}) as outbound,
                            collect(DISTINCT {target_canonical_id: s.canonical_id, target_citation: s.citation,
                                             context_snippet: '', source_provision: s.citation, resolved: true}) as inbound"
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

        let outbound_json: Vec<serde_json::Value> = row.get("outbound").ok().unwrap_or_default();
        let outbound: Vec<Citation> = outbound_json
            .into_iter()
            .map(|c: serde_json::Value| Citation {
                target_canonical_id: c["target_canonical_id"].as_str().map(|s| s.to_string()),
                target_citation: c["target_citation"].as_str().unwrap_or("").to_string(),
                context_snippet: c["context_snippet"].as_str().unwrap_or("").to_string(),
                source_provision: c["source_provision"].as_str().unwrap_or("").to_string(),
                resolved: c["resolved"].as_bool().unwrap_or(true),
            })
            .collect();

        let inbound_json: Vec<serde_json::Value> = row.get("inbound").ok().unwrap_or_default();
        let inbound: Vec<Citation> = inbound_json
            .into_iter()
            .map(|c: serde_json::Value| Citation {
                target_canonical_id: c["target_canonical_id"].as_str().map(|s| s.to_string()),
                target_citation: c["target_citation"].as_str().unwrap_or("").to_string(),
                context_snippet: c["context_snippet"].as_str().unwrap_or("").to_string(),
                source_provision: c["source_provision"].as_str().unwrap_or("").to_string(),
                resolved: c["resolved"].as_bool().unwrap_or(true),
            })
            .collect();

        Ok(CitationsResponse {
            citation: citation.to_string(),
            outbound,
            inbound,
            unresolved: vec![],
        })
    }

    pub async fn get_semantics(&self, citation: &str) -> ApiResult<SemanticsResponse> {
        Ok(SemanticsResponse {
            citation: citation.to_string(),
            obligations: vec![],
            exceptions: vec![],
            deadlines: vec![],
            penalties: vec![],
            definitions: vec![],
        })
    }

    pub async fn get_history(&self, citation: &str) -> ApiResult<HistoryResponse> {
        Ok(HistoryResponse {
            citation: citation.to_string(),
            source_notes: vec![],
            amendments: vec![],
            session_laws: vec![],
            status_events: vec![],
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
                     MATCH (p)-[:BELONGS_TO]->(i:LegalTextIdentity)
                     OPTIONAL MATCH (p)<-[:DERIVED_FROM]-(chunk:RetrievalChunk)
                     OPTIONAL MATCH (p)-[:CITES]->(target:LegalTextIdentity)
                     OPTIONAL MATCH (source:LegalTextIdentity)-[:CITES]->(p)
                     OPTIONAL MATCH (p)<-[:PARENT_OF]-(child:Provision)
                     WITH p, i,
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
                         canonical_id: target.canonical_id,
                         citation: target.citation
                       })[0..50] as outbound_nodes,
                       collect(DISTINCT {
                         canonical_id: source.canonical_id,
                         citation: source.citation
                       })[0..50] as inbound_nodes,
                       collect(DISTINCT {
                         provision_id: child.provision_id,
                         display_citation: child.display_citation,
                         provision_type: coalesce(child.provision_type, child.kind, 'section'),
                         parent_id: child.parent_id,
                         text: child.text,
                         qc_status: coalesce(child.qc_status, 'pass'),
                         status: coalesce(child.status, i.status, 'active')
                       })[0..50] as children
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
                            children",
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

        let outbound = nodes_to_citations(
            row.get("outbound_nodes").ok().unwrap_or_default(),
            &provision.display_citation,
        );
        let inbound = nodes_to_citations(
            row.get("inbound_nodes").ok().unwrap_or_default(),
            &provision.display_citation,
        );

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
            ancestors: Vec::new(),
            children,
            siblings: Vec::new(),
            chunks,
            outbound_citations: outbound,
            inbound_citations: inbound,
            definitions: Vec::new(),
            exceptions: Vec::new(),
            deadlines: Vec::new(),
            qc_notes: Vec::new(),
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
        let relationship_types =
            graph_relationship_types(&params.mode, params.relationship_types.as_deref());
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
               WHERE path IS NULL OR all(rel IN relationships(path) WHERE type(rel) IN $relationship_types)
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
                      null AS edge_weight
               UNION ALL
               WITH center
               OPTIONAL MATCH path = (center)-[*1..{depth}]-(neighbor)
               WHERE path IS NULL OR all(rel IN relationships(path) WHERE type(rel) IN $relationship_types)
               WITH [p IN collect(path)[0..$path_limit] WHERE p IS NOT NULL] AS paths
               UNWIND paths AS path
               UNWIND relationships(path) AS rel
               WITH DISTINCT rel, startNode(rel) AS source, endNode(rel) AS target
               WHERE ($include_chunks = true OR (NOT 'RetrievalChunk' IN labels(source) AND NOT 'RetrievalChunk' IN labels(target)))
                 AND ($min_confidence < 0 OR coalesce(rel.confidence, 1.0) >= $min_confidence)
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
                      coalesce(rel.weight, rel.score, rel.similarity_score) AS edge_weight
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
                        similarity_score: None,
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
        if params.include_similarity.unwrap_or(false) {
            warnings.push(format!(
                "Similarity edges are not included by /graph/neighborhood; requested threshold was {:.2}. Use /graph/hybrid when enabled.",
                params.similarity_threshold.unwrap_or(0.78)
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

    pub async fn suggest(&self, q: &str, limit: u32) -> ApiResult<Vec<SuggestResult>> {
        let mut result = self.graph.execute(
            query("MATCH (n:LegalTextIdentity) WHERE toUpper(n.citation) STARTS WITH toUpper($q) OR toUpper(n.title) CONTAINS toUpper($q)
                   RETURN n.citation as label, 'statute' as kind, '/statutes/' + n.citation as href
                   UNION
                   MATCH (n:DefinedTerm) WHERE toUpper(n.term) STARTS WITH toUpper($q)
                   RETURN n.term as label, 'definition' as kind, '/search?q=' + n.term as href
                   UNION
                   MATCH (n:SourceDocument) WHERE toUpper(n.title) CONTAINS toUpper($q)
                   RETURN n.title as label, 'chapter' as kind, '/statutes/' + n.source_document_id as href
                   LIMIT $limit")
            .param("q", q)
            .param("limit", limit as i64)
        ).await.map_err(ApiError::Neo4jConnection)?;

        let mut suggestions = Vec::new();
        while let Some(row) = result.next().await.map_err(ApiError::Neo4jConnection)? {
            suggestions.push(SuggestResult {
                label: row.get("label").unwrap_or_default(),
                kind: row.get("kind").unwrap_or_default(),
                href: row.get("href").unwrap_or_default(),
            });
        }
        Ok(suggestions)
    }
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

fn graph_relationship_types(mode: &str, override_value: Option<&str>) -> Vec<String> {
    let explicit = graph_csv(override_value);
    if !explicit.is_empty() {
        return explicit;
    }

    let values: &[&str] = match mode {
        "citation" => &[
            "CITES",
            "MENTIONS_CITATION",
            "RESOLVES_TO",
            "RESOLVES_TO_VERSION",
            "RESOLVES_TO_PROVISION",
            "CITES_EXTERNAL",
        ],
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
            "MENTIONS_CITATION",
            "RESOLVES_TO",
            "RESOLVES_TO_VERSION",
            "RESOLVES_TO_PROVISION",
            "CITES_EXTERNAL",
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
        | "MENTIONS_CITATION"
        | "RESOLVES_TO"
        | "RESOLVES_TO_VERSION"
        | "RESOLVES_TO_PROVISION"
        | "CITES_EXTERNAL" => (false, 1.5, "#60a5fa"),
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

fn nodes_to_citations(nodes: Vec<serde_json::Value>, source_provision: &str) -> Vec<Citation> {
    nodes
        .into_iter()
        .filter_map(|node| {
            let citation = json_string(&node, "citation");
            if citation.is_empty() {
                return None;
            }

            Some(Citation {
                target_canonical_id: node["canonical_id"].as_str().map(|value| value.to_string()),
                target_citation: citation,
                context_snippet: String::new(),
                source_provision: source_provision.to_string(),
                resolved: true,
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
