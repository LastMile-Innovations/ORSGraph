use crate::error::ApiResult;
use crate::models::search::SearchResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct RerankService {
    client: reqwest::Client,
    api_key: String,
    model: String,
    candidates: usize,
    top_k: usize,
    max_doc_tokens: usize,
}

#[derive(Serialize)]
struct VoyageRerankRequest {
    query: String,
    documents: Vec<String>,
    model: String,
    top_k: usize,
    truncation: bool,
}

#[derive(Deserialize)]
struct VoyageRerankResponse {
    data: Vec<VoyageRerankResult>,
    usage: VoyageUsage,
}

#[derive(Deserialize)]
struct VoyageRerankResult {
    index: usize,
    relevance_score: f32,
}

#[derive(Deserialize)]
struct VoyageUsage {
    total_tokens: usize,
}

pub struct RerankOutput {
    pub results: Vec<RerankedResult>,
    pub total_tokens: usize,
}

#[derive(Debug)]
pub struct RerankedResult {
    pub index: usize,
    pub score: f32,
}

impl RerankService {
    pub fn new(
        api_key: String,
        model: String,
        candidates: usize,
        top_k: usize,
        max_doc_tokens: usize,
        timeout_ms: u64,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model,
            candidates,
            top_k,
            max_doc_tokens,
        }
    }

    pub async fn rerank(
        &self,
        query: &str,
        candidates: &[SearchResult],
    ) -> ApiResult<RerankOutput> {
        let instruction = "Rank Oregon legal authorities by their usefulness for answering the user's legal research query. Prefer exact ORS citation matches, current law, source-backed provisions, definitions with correct scope, deadlines, penalties, remedies, source notes, and graph-connected legal meaning. Penalize unrelated text, stale/repealed law unless the query asks history, low-confidence extractions, and chunks without source support.";

        let full_query = format!("Instruction: {}\n\nUser query: {}", instruction, query);

        let documents: Vec<String> = candidates
            .iter()
            .map(|c| self.format_candidate_doc(c))
            .collect();

        let request = VoyageRerankRequest {
            query: full_query,
            documents,
            model: self.model.clone(),
            top_k: self.top_k,
            truncation: true,
        };

        let response = self
            .client
            .post("https://api.voyageai.com/v1/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(crate::error::ApiError::External(format!(
                "Voyage API error: {}",
                err_text
            )));
        }

        let voyage_res: VoyageRerankResponse = response.json().await?;

        Ok(RerankOutput {
            results: voyage_res
                .data
                .into_iter()
                .map(|r| RerankedResult {
                    index: r.index,
                    score: r.relevance_score,
                })
                .collect(),
            total_tokens: voyage_res.usage.total_tokens,
        })
    }

    fn format_candidate_doc(&self, c: &SearchResult) -> String {
        let mut doc = format!("Type: {}\n", c.kind);
        if let Some(cit) = &c.citation {
            doc.push_str(&format!("Citation: {}\n", cit));
        }
        if let Some(title) = &c.title {
            doc.push_str(&format!("Title: {}\n", title));
        }
        doc.push_str(&format!(
            "Status: {}\n",
            c.status.as_deref().unwrap_or("unknown")
        ));
        if !c.semantic_types.is_empty() {
            doc.push_str(&format!(
                "Semantic types: {}\n",
                c.semantic_types.join(", ")
            ));
        }
        if let Some(graph) = &c.graph {
            doc.push_str("Graph context:\n");
            if let Some(canonical_id) = &graph.canonical_id {
                doc.push_str(&format!("- Canonical authority: {}\n", canonical_id));
            }
            if let Some(provision_id) = &graph.provision_id {
                doc.push_str(&format!("- Source provision: {}\n", provision_id));
            }
            if let Some(count) = graph.connected_node_count {
                doc.push_str(&format!("- Connected legal nodes: {}\n", count));
            }
            if let Some(count) = graph.citation_count {
                doc.push_str(&format!("- Cites: {}\n", count));
            }
            if let Some(count) = graph.cited_by_count {
                doc.push_str(&format!("- Cited by: {}\n", count));
            }
        }
        doc.push_str(&format!("Text:\n{}\n", c.snippet));
        if c.source_backed {
            doc.push_str("Source: [source-backed]\n");
        }

        // Simple truncation if needed (Voyage handles truncation: true, but we can be polite)
        // 1 token is roughly 4 characters
        let char_limit = self.max_doc_tokens * 4;
        if doc.len() > char_limit {
            doc.truncate(char_limit);
        }

        doc
    }

    pub fn candidates_limit(&self) -> usize {
        self.candidates
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}
