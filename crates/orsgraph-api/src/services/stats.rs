use crate::error::ApiResult;
use crate::models::api::StatsResponse;
use crate::models::home::CorpusCounts;
use crate::services::neo4j::Neo4jService;
use neo4rs::query;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct StatsService {
    neo4j: Arc<Neo4jService>,
    cache: RwLock<Option<(Instant, CorpusCounts)>>,
    stats_cache: RwLock<Option<(Instant, StatsResponse)>>,
}

impl StatsService {
    pub fn new(neo4j: Arc<Neo4jService>) -> Self {
        Self {
            neo4j,
            cache: RwLock::new(None),
            stats_cache: RwLock::new(None),
        }
    }

    pub async fn get_node_count(&self) -> ApiResult<i64> {
        let counts = self.get_corpus_counts().await?;
        Ok(counts.neo4j_nodes)
    }

    pub async fn get_relationship_count(&self) -> ApiResult<i64> {
        let counts = self.get_corpus_counts().await?;
        Ok(counts.neo4j_relationships)
    }

    pub async fn get_label_counts(&self) -> ApiResult<HashMap<String, i64>> {
        // We can just query this directly, or add caching if needed.
        // For simplicity, we just query it directly here as it's not explicitly cached in the prompt's TTL list,
        // though it's part of stats.
        let rows = self
            .neo4j
            .run_rows(query(
                "MATCH (n) UNWIND labels(n) AS label RETURN label, count(*) AS count",
            ))
            .await?;

        let mut counts = HashMap::new();
        for row in rows {
            let label = row.get::<String>("label").unwrap_or_default();
            let count = row.get::<i64>("count").unwrap_or(0);
            counts.insert(label, count);
        }
        Ok(counts)
    }

    pub async fn get_relationship_type_counts(&self) -> ApiResult<HashMap<String, i64>> {
        let rows = self
            .neo4j
            .run_rows(query(
                "MATCH ()-[r]->() RETURN type(r) AS type, count(*) AS count",
            ))
            .await?;

        let mut counts = HashMap::new();
        for row in rows {
            let t = row.get::<String>("type").unwrap_or_default();
            let count = row.get::<i64>("count").unwrap_or(0);
            counts.insert(t, count);
        }
        Ok(counts)
    }

    pub async fn get_corpus_counts(&self) -> ApiResult<CorpusCounts> {
        {
            let cache = self.cache.read().await;
            if let Some((time, counts)) = &*cache {
                if time.elapsed() < Duration::from_secs(30) {
                    return Ok(counts.clone());
                }
            }
        }

        let rows = self
            .neo4j
            .run_rows(query(
                "
                MATCH (n) RETURN count(n) AS count, 'neo4jNodes' AS type
                UNION ALL
                MATCH ()-[r]->() RETURN count(r) AS count, 'neo4jRelationships' AS type
                UNION ALL
                MATCH (n:LegalTextIdentity) RETURN count(n) AS count, 'sections' AS type
                UNION ALL
                MATCH (n:LegalTextVersion) RETURN count(n) AS count, 'versions' AS type
                UNION ALL
                MATCH (n:Provision) RETURN count(n) AS count, 'provisions' AS type
                UNION ALL
                MATCH (n:RetrievalChunk) RETURN count(n) AS count, 'retrievalChunks' AS type
                UNION ALL
                MATCH (n:CitationMention) RETURN count(n) AS count, 'citationMentions' AS type
                UNION ALL
                MATCH ()-[r:CITES]->() RETURN count(r) AS count, 'citesEdges' AS type
                UNION ALL
                MATCH (n:LegalSemanticNode) RETURN count(n) AS count, 'semanticNodes' AS type
                UNION ALL
                MATCH (n:SourceNote) RETURN count(n) AS count, 'sourceNotes' AS type
                UNION ALL
                MATCH (n:Amendment) RETURN count(n) AS count, 'amendments' AS type
                UNION ALL
                MATCH (n:SessionLaw) RETURN count(n) AS count, 'sessionLaws' AS type
            ",
            ))
            .await?;

        let mut map = HashMap::new();
        for row in rows {
            let t = row.get::<String>("type").unwrap_or_default();
            let c = row.get::<i64>("count").unwrap_or(0);
            map.insert(t, c);
        }

        let counts = CorpusCounts {
            sections: *map.get("sections").unwrap_or(&0),
            versions: *map.get("versions").unwrap_or(&0),
            provisions: *map.get("provisions").unwrap_or(&0),
            retrieval_chunks: *map.get("retrievalChunks").unwrap_or(&0),
            citation_mentions: *map.get("citationMentions").unwrap_or(&0),
            cites_edges: *map.get("citesEdges").unwrap_or(&0),
            semantic_nodes: *map.get("semanticNodes").unwrap_or(&0),
            source_notes: *map.get("sourceNotes").unwrap_or(&0),
            amendments: *map.get("amendments").unwrap_or(&0),
            session_laws: *map.get("sessionLaws").unwrap_or(&0),
            neo4j_nodes: *map.get("neo4jNodes").unwrap_or(&0),
            neo4j_relationships: *map.get("neo4jRelationships").unwrap_or(&0),
        };

        {
            let mut cache = self.cache.write().await;
            *cache = Some((Instant::now(), counts.clone()));
        }

        Ok(counts)
    }

    pub async fn get_stats_response(&self) -> ApiResult<StatsResponse> {
        {
            let cache = self.stats_cache.read().await;
            if let Some((time, response)) = &*cache {
                if time.elapsed() < Duration::from_secs(30) {
                    // clone manually since StatsResponse doesn't implement Clone, or we could just implement Clone for it.
                    // Wait, models/api.rs StatsResponse doesn't have Clone. Let's return a new instance.
                    return Ok(StatsResponse {
                        nodes: response.nodes,
                        relationships: response.relationships,
                        chapters: response.chapters,
                        sections: response.sections,
                        provisions: response.provisions,
                        chunks: response.chunks,
                        citations: response.citations,
                        cites_edges: response.cites_edges,
                        semantic_nodes: response.semantic_nodes,
                        last_seeded_at: response.last_seeded_at.clone(),
                    });
                }
            }
        }

        let counts = self.get_corpus_counts().await?;

        // chapters wasn't in corpus counts query, let's query it or default it
        let rows = self
            .neo4j
            .run_rows(query("MATCH (n:SourceDocument) RETURN count(n) AS count"))
            .await?;
        let chapters = if let Some(row) = rows.into_iter().next() {
            row.get::<i64>("count").unwrap_or(0) as u64
        } else {
            0
        };

        let response = StatsResponse {
            nodes: counts.neo4j_nodes as u64,
            relationships: counts.neo4j_relationships as u64,
            chapters,
            sections: counts.sections as u64,
            provisions: counts.provisions as u64,
            chunks: counts.retrieval_chunks as u64,
            citations: counts.citation_mentions as u64,
            cites_edges: counts.cites_edges as u64,
            semantic_nodes: counts.semantic_nodes as u64,
            last_seeded_at: None,
        };

        {
            let mut cache = self.stats_cache.write().await;
            *cache = Some((
                Instant::now(),
                StatsResponse {
                    nodes: response.nodes,
                    relationships: response.relationships,
                    chapters: response.chapters,
                    sections: response.sections,
                    provisions: response.provisions,
                    chunks: response.chunks,
                    citations: response.citations,
                    cites_edges: response.cites_edges,
                    semantic_nodes: response.semantic_nodes,
                    last_seeded_at: response.last_seeded_at.clone(),
                },
            ));
        }

        Ok(response)
    }
}
