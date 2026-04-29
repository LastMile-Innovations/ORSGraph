use crate::neo4j_loader::Neo4jLoader;
use anyhow::Result;

pub struct RetrievalAugmentor {
    loader: Neo4jLoader,
}

impl RetrievalAugmentor {
    pub fn new(loader: Neo4jLoader) -> Self {
        Self { loader }
    }

    /// Orchestrates a full hybrid retrieval and multi-hop enrichment flow.
    /// Returns a Markdown-formatted context block for use in an LLM prompt.
    pub async fn retrieve_context(
        &self,
        query_text: &str,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<String> {
        // 1. Perform Hybrid Search (Vector + Full-text fused via RRF)
        let hybrid_results = self
            .loader
            .hybrid_search(query_text, embedding, limit, 60.0)
            .await?;

        // 2. Perform Multi-hop Enrichment (Citations, Definitions, Breadcrumbs)
        let enriched = self.loader.get_enriched_context(hybrid_results).await?;

        // 3. Format as Markdown for LLM Context
        Ok(self.format_context(&enriched))
    }

    /// Formats enriched chunks into a structured Markdown block.
    pub fn format_context(&self, enriched: &[crate::models::EnrichedChunk]) -> String {
        format_retrieval_context(enriched)
    }
}

pub fn format_retrieval_context(enriched: &[crate::models::EnrichedChunk]) -> String {
    let mut context = String::new();
    context.push_str("# LEGAL CONTEXT FOR QUERY\n");
    context.push_str("The following statutes and related definitions have been retrieved from the ORS Knowledge Graph based on semantic and keyword relevance.\n\n");

    for (i, chunk) in enriched.iter().enumerate() {
        context.push_str(&format!(
            "## [{}] {}\n",
            i + 1,
            chunk.citation.as_deref().unwrap_or("Uncited Statute")
        ));
        context.push_str(&format!("**Location:** {}\n", chunk.breadcrumb));

        if let Some(year) = chunk.edition_year {
            context.push_str(&format!("**Edition:** {} ORS\n", year));
        }

        if let Some(status) = &chunk.status {
            context.push_str(&format!("**Status:** {}\n", status));
        }

        context.push_str("\n### Primary Text\n");
        context.push_str(&format!("```text\n{}\n```\n", chunk.text));

        if !chunk.definitions.is_empty() {
            context.push_str("\n### Relevant Definitions\n");
            for def in &chunk.definitions {
                context.push_str(&format!("- **{}**: {}\n", def.term, def.definition));
            }
        }

        if !chunk.citations.is_empty() {
            context.push_str("\n### Referenced Statutes\n");
            for cite in &chunk.citations {
                context.push_str(&format!(
                    "- **{}** ({}): {}\n",
                    cite.citation, cite.target_citation, cite.target_text
                ));
            }
        }

        context.push_str("\n---\n\n");
    }

    if enriched.is_empty() {
        context.push_str("_No relevant legal context was found for this query._\n");
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnrichedChunk, EnrichedCitation, EnrichedDefinition};

    #[test]
    fn format_retrieval_context_handles_empty_results() {
        let context = format_retrieval_context(&[]);
        assert!(context.contains("No relevant legal context was found"));
    }

    #[test]
    fn format_retrieval_context_includes_primary_fields() {
        let enriched = vec![EnrichedChunk {
            chunk_id: "test:1".to_string(),
            text: "Primary text content".to_string(),
            citation: Some("ORS 1.001".to_string()),
            breadcrumb: "Oregon > ORS > 1.001".to_string(),
            score: 0.95,
            citations: vec![EnrichedCitation {
                citation: "ORS 1.001".to_string(),
                target_citation: "ORS 2.002".to_string(),
                target_text: "Target description".to_string(),
            }],
            definitions: vec![EnrichedDefinition {
                term: "TestTerm".to_string(),
                definition: "Test definition content".to_string(),
            }],
            status: Some("active".to_string()),
            edition_year: Some(2025),
        }];

        let context = format_retrieval_context(&enriched);
        assert!(context.contains("ORS 1.001"));
        assert!(context.contains("Primary text content"));
        assert!(context.contains("TestTerm"));
        assert!(context.contains("Target description"));
        assert!(context.contains("2025 ORS"));
        assert!(context.contains("**Status:** active"));
    }
}
