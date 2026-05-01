use crate::embedding_profiles::EmbeddingProfile;
use crate::embeddings::EmbeddingTargetSpec;
use crate::models::{
    Amendment, ChapterFrontMatter, ChapterHeading, ChapterTocEntry, CitationMention, CitesEdge,
    Deadline, DefinedTerm, Definition, DefinitionScope, EnrichedChunk, EnrichedCitation,
    EnrichedDefinition, Exception, FormText, HtmlParagraph, LegalAction, LegalActor,
    LegalSemanticNode, LegalTextIdentity, LegalTextVersion, LineageEvent, MoneyAmount, Obligation,
    ParserDiagnostic, Penalty, Provision, RateLimit, Remedy, RequiredNotice, ReservedRange,
    RetrievalChunk, SessionLaw, SourceDocument, SourceNote, StatusEvent, TaxRule, TemporalEffect,
    TimeInterval, TitleChapterEntry,
};
use anyhow::{Context, Result};
use neo4rs::{query, ConfigBuilder, Graph};
use regex::Regex;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

const DEFAULT_EMBEDDING_UPDATE_BATCH_SIZE: usize = 1000;

static CONCURRENT_TX_PATTERN: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"IN \d+ CONCURRENT TRANSACTIONS OF \d+ ROWS").unwrap()
});

fn normalize_cypher_statement(statement: &str) -> Option<String> {
    let normalized = statement
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
fn needs_embedding_for_metadata(
    has_embedding: bool,
    embedding_model: Option<&str>,
    embedding_dim: Option<i32>,
    embedding_input_hash: Option<&str>,
    current_embedding_input_hash: Option<&str>,
    target_model: &str,
    target_dimension: i32,
) -> bool {
    !has_embedding
        || embedding_model.map_or(true, |model| model != target_model)
        || embedding_dim.map_or(true, |dim| dim != target_dimension)
        || embedding_input_hash.is_none()
        || current_embedding_input_hash.is_none()
        || embedding_input_hash != current_embedding_input_hash
}

#[derive(Debug, Clone, Copy)]
pub struct SeedBatchConfig {
    pub node_batch_size: usize,
    pub edge_batch_size: usize,
    pub relationship_batch_size: usize,
}

impl SeedBatchConfig {
    pub fn new(
        node_batch_size: usize,
        edge_batch_size: usize,
        relationship_batch_size: usize,
    ) -> Self {
        Self {
            node_batch_size: node_batch_size.max(1),
            edge_batch_size: edge_batch_size.max(1),
            relationship_batch_size: relationship_batch_size.max(1),
        }
    }
}

pub struct Neo4jLoader {
    graph: Arc<Graph>,
}

impl Neo4jLoader {
    /// Helper function to load a Cypher query from a file.
    ///
    /// # Arguments
    /// * `query_name` - Name of the query file (without .cypher extension)
    ///
    /// # Returns
    /// The query string as a Result
    fn load_query(query_name: &str) -> Result<String> {
        let query_path = Path::new("cypher/queries").join(format!("{}.cypher", query_name));
        std::fs::read_to_string(&query_path)
            .with_context(|| format!("Failed to read query file: {}", query_path.display()))
    }

    /// Transforms Cypher query batch directives for Community Edition compatibility.
    ///
    /// Enterprise Edition supports `IN N CONCURRENT TRANSACTIONS OF X ROWS` syntax,
    /// but Community Edition only supports `IN TRANSACTIONS OF X ROWS`.
    /// This function normalizes all patterns to the Community Edition compatible format.
    /// It also replaces the :transaction placeholder with the appropriate batch syntax.
    fn with_transaction_batch(query_str: String, batch_size: usize) -> String {
        let batch_size_str = batch_size.max(1).to_string();

        // 1. Handle :transaction placeholder if present
        let mut result = query_str.replace(
            ":transaction",
            &format!("CALL {{ WITH row }} IN TRANSACTIONS OF {batch_size_str} ROWS"),
        );

        // 2. Normalize any existing concurrent transaction patterns
        result = CONCURRENT_TX_PATTERN
            .replace_all(
                &result,
                &format!("IN TRANSACTIONS OF {batch_size_str} ROWS"),
            )
            .to_string();

        // 3. Update any hardcoded batch sizes to the requested batch_size
        result
            .replace(
                "IN TRANSACTIONS OF 1000 ROWS",
                &format!("IN TRANSACTIONS OF {batch_size_str} ROWS"),
            )
            .replace(
                "IN TRANSACTIONS OF 5000 ROWS",
                &format!("IN TRANSACTIONS OF {batch_size_str} ROWS"),
            )
            .replace(
                "IN TRANSACTIONS OF 250 ROWS",
                &format!("IN TRANSACTIONS OF {batch_size_str} ROWS"),
            )
            .replace(
                "IN TRANSACTIONS OF 100 ROWS",
                &format!("IN TRANSACTIONS OF {batch_size_str} ROWS"),
            )
    }

    /// Deduplicates a vector of items by a key extractor function.
    /// Keeps the first occurrence of each unique key.
    ///
    /// # Arguments
    /// * `items` - Vector of items to deduplicate
    /// * `key_fn` - Function to extract the key from each item
    ///
    /// # Returns
    /// Deduplicated vector with only unique keys
    fn deduplicate_by_key<T, K: std::hash::Hash + Eq>(
        items: Vec<T>,
        key_fn: impl Fn(&T) -> K,
    ) -> Vec<T> {
        let mut seen = std::collections::HashSet::new();
        items
            .into_iter()
            .filter(|item| {
                let key = key_fn(item);
                seen.insert(key)
            })
            .collect()
    }

    /// Logs deduplication statistics for debugging.
    ///
    /// # Arguments
    /// * `entity_type` - Name of the entity type being loaded
    /// * `original_count` - Original number of items
    /// * `deduped_count` - Count after deduplication
    fn log_dedup_stats(entity_type: &str, original_count: usize, deduped_count: usize) {
        if original_count != deduped_count {
            let removed = original_count - deduped_count;
            info!(
                "Deduplicated {}: removed {} duplicates ({} -> {})",
                entity_type, removed, original_count, deduped_count
            );
        }
    }

    /// Creates a new Neo4jLoader instance with the given connection parameters.
    ///
    /// # Arguments
    /// * `uri` - Neo4j database URI (e.g., "bolt://localhost:7687")
    /// * `user` - Database username
    /// * `pass` - Database password
    ///
    /// # Returns
    /// A new Neo4jLoader instance or an error if connection fails
    pub async fn new(uri: &str, user: &str, pass: &str) -> Result<Self> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(pass)
            .build()?;
        let graph = Arc::new(Graph::connect(config).await?);
        Ok(Self { graph })
    }

    pub async fn run_query(&self, q: neo4rs::Query) -> Result<()> {
        self.graph.run(q).await?;
        Ok(())
    }

    async fn run_multi_statement_query(&self, query_str: &str) -> Result<()> {
        // Split by semicolon and execute each statement
        let statements: Vec<String> = query_str
            .split(';')
            .filter_map(normalize_cypher_statement)
            .collect();

        let total = statements.len();
        info!("Executing {} Cypher statement(s)", total);

        for (idx, stmt) in statements.iter().enumerate() {
            let start = std::time::Instant::now();
            let preview = if stmt.len() > 50 {
                format!("{}...", &stmt[..50])
            } else {
                stmt.to_string()
            };

            if let Err(e) = self.graph.run(query(stmt)).await {
                return Err(anyhow::anyhow!(
                    "Failed to execute statement {}/{}: {}\nStatement: {}",
                    idx + 1,
                    total,
                    e,
                    preview
                ));
            }

            let elapsed = start.elapsed().as_millis();
            if total > 1 {
                info!(
                    "Statement {}/{} completed in {}ms: {}",
                    idx + 1,
                    total,
                    elapsed,
                    preview
                );
            }
        }

        info!("All {} statement(s) executed successfully", total);
        Ok(())
    }
}

impl Neo4jLoader {
    ///
    /// This method is idempotent - it uses IF NOT EXISTS to avoid errors on re-runs.
    /// It loads constraints and indexes from cypher/indexes.cypher as the single source of truth.
    ///
    /// # Arguments
    /// * `dimension` - Embedding dimension for vector index (e.g., 1024)
    /// * `create_vector_index` - Whether to create the vector similarity index
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if constraint/index creation fails
    pub async fn create_constraints(
        &self,
        dimension: i32,
        create_vector_index: bool,
    ) -> Result<()> {
        // Load constraints and indexes from cypher/indexes.cypher
        let cypher_content = std::fs::read_to_string("cypher/indexes.cypher")
            .context("Failed to read cypher/indexes.cypher")?;

        // Split by semicolon and filter out empty lines or lines that are just comments
        let statements: Vec<&str> = cypher_content
            .split(';')
            .map(|s| s.trim())
            .filter(|s| {
                if s.is_empty() {
                    return false;
                }
                // Check if the statement has any actual Cypher content (not just comments)
                let lines: Vec<&str> = s.lines().map(|l| l.trim()).collect();
                lines
                    .iter()
                    .any(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("/*"))
            })
            .collect();

        // Execute all statements except the vector index (handled separately)
        for stmt in statements {
            // Skip the vector index statement - it has a placeholder and is conditional
            if stmt.contains("CREATE VECTOR INDEX") || stmt.contains("{DIMENSION}") {
                continue;
            }
            self.graph.run(query(stmt)).await?;
        }

        // Create vector index conditionally with the correct dimension
        if create_vector_index {
            // 1. Vector Property Type Constraints (Hardens data integrity)
            let chunk_constraint = format!(
                "CREATE CONSTRAINT retrieval_chunk_vector_type IF NOT EXISTS \
                 FOR (n:RetrievalChunk) \
                 REQUIRE n.embedding IS :: VECTOR<FLOAT32>({})",
                dimension
            );
            self.graph.run(query(&chunk_constraint)).await?;

            let provision_constraint = format!(
                "CREATE CONSTRAINT provision_vector_type IF NOT EXISTS \
                 FOR (p:Provision) \
                 REQUIRE p.embedding IS :: VECTOR<FLOAT32>({})",
                dimension
            );
            self.graph.run(query(&provision_constraint)).await?;

            // 2. Vector Indexes
            let chunk_idx = format!(
                "CREATE VECTOR INDEX retrieval_chunk_embedding_1024 IF NOT EXISTS \
                 FOR (n:RetrievalChunk) \
                 ON n.embedding \
                 WITH [n.citation, n.chunk_type, n.answer_policy, n.edition_year, n.authority_level, n.is_definition_candidate, n.is_exception_candidate] \
                 OPTIONS {{ indexConfig: {{ `vector.dimensions`: {}, `vector.similarity_function`: 'cosine' }} }}",
                dimension
            );
            self.graph.run(query(&chunk_idx)).await?;

            let provision_idx = format!(
                "CREATE VECTOR INDEX provision_embedding_1024 IF NOT EXISTS \
                 FOR (p:Provision) \
                 ON p.embedding \
                 OPTIONS {{ indexConfig: {{ `vector.dimensions`: {}, `vector.similarity_function`: 'cosine' }} }}",
                dimension
            );
            self.graph.run(query(&provision_idx)).await?;

            let version_idx = format!(
                "CREATE VECTOR INDEX legal_text_version_embedding_1024 IF NOT EXISTS \
                 FOR (v:LegalTextVersion) \
                 ON v.embedding \
                 OPTIONS {{ indexConfig: {{ `vector.dimensions`: {}, `vector.similarity_function`: 'cosine' }} }}",
                dimension
            );
            self.graph.run(query(&version_idx)).await?;
        }

        Ok(())
    }

    /// Clears all nodes and relationships from the database.
    ///
    /// # Warning
    /// This is destructive and cannot be undone. Use with caution.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if the operation fails
    pub async fn clear_database(&self) -> Result<()> {
        // Delete in batches to avoid memory issues
        let batch_size = 1000;
        loop {
            let mut result = self
                .graph
                .execute(query("MATCH (n) WITH n LIMIT $batchSize DETACH DELETE n RETURN count(n) as deleted").param("batchSize", batch_size))
                .await?;
            let row = result.next().await?;
            match row {
                Some(r) => {
                    let deleted: i64 = r.get("deleted").unwrap_or(0);
                    if deleted == 0 {
                        break;
                    }
                }
                None => break,
            }
        }
        Ok(())
    }

    /// Loads jurisdiction nodes into the graph (US and Oregon state).
    ///
    /// Creates the federal US jurisdiction and Oregon state jurisdiction with proper hierarchy.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_jurisdictions(&self) -> Result<()> {
        let q = Self::load_query("load_jurisdictions")?;
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Loads public body nodes into the graph (Oregon Legislative Assembly).
    ///
    /// Creates the public body node and establishes relationships with jurisdictions.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_public_bodies(&self) -> Result<()> {
        let q = Self::load_query("load_public_bodies")?;
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Loads the legal corpus node (Oregon Revised Statutes).
    ///
    /// Creates the ORS corpus node and establishes relationships with jurisdictions and public bodies.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_corpus(&self) -> Result<()> {
        let q = Self::load_query("load_corpus")?;
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Loads a specific corpus edition (e.g., ORS 2025).
    ///
    /// # Arguments
    /// * `edition_year` - The year of the edition to load (e.g., 2025)
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_corpus_editions(&self, edition_year: i32) -> Result<()> {
        let edition_id = format!("or:ors@{}", edition_year);
        let q = Self::load_query("load_corpus_editions")?;
        self.graph
            .run(
                query(&q)
                    .param("edition_id", edition_id)
                    .param("edition_year", edition_year as i64),
            )
            .await?;
        Ok(())
    }

    /// Creates chapter version nodes from loaded legal text versions.
    ///
    /// This method aggregates legal text versions by chapter and creates ChapterVersion nodes.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn load_chapter_versions(&self) -> Result<()> {
        let q = Self::load_query("load_chapter_versions")?;
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Loads source document nodes into the graph.
    ///
    /// # Arguments
    /// * `docs` - Vector of source documents to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_source_documents(
        &self,
        docs: Vec<SourceDocument>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_source_documents")?;
        self.run_rows_with_batch(&query_str, docs, batch_size).await
    }

    pub async fn load_html_paragraphs(
        &self,
        rows: Vec<HtmlParagraph>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_html_paragraphs")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_chapter_front_matter(
        &self,
        rows: Vec<ChapterFrontMatter>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_chapter_front_matter")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_title_chapter_entries(
        &self,
        rows: Vec<TitleChapterEntry>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_title_chapter_entries")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    /// Loads legal text identity nodes into the graph.
    ///
    /// # Arguments
    /// * `identities` - Vector of legal text identities to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_identities(
        &self,
        identities: Vec<LegalTextIdentity>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_identities")?;
        self.run_rows_with_batch(&query_str, identities, batch_size)
            .await
    }

    /// Loads legal text version nodes into the graph.
    ///
    /// # Arguments
    /// * `versions` - Vector of legal text versions to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_versions(
        &self,
        versions: Vec<LegalTextVersion>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_versions")?;
        self.run_rows_with_batch(&query_str, versions, batch_size)
            .await
    }

    /// Loads provision nodes into the graph.
    ///
    /// # Arguments
    /// * `provisions` - Vector of provisions to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_provisions(
        &self,
        provisions: Vec<Provision>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_provisions")?;
        self.run_rows_with_batch(&query_str, provisions, batch_size)
            .await
    }

    /// Loads citation mention nodes into the graph.
    ///
    /// # Arguments
    /// * `citations` - Vector of citation mentions to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_citation_mentions(
        &self,
        citations: Vec<CitationMention>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_citation_mentions")?;
        self.run_rows_with_batch(&query_str, citations, batch_size)
            .await
    }

    /// Loads chapter heading nodes into the graph.
    ///
    /// # Arguments
    /// * `headings` - Vector of chapter headings to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_chapter_headings(
        &self,
        headings: Vec<ChapterHeading>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_chapter_headings")?;
        self.run_rows_with_batch(&query_str, headings, batch_size)
            .await
    }

    /// Loads retrieval chunk nodes into the graph.
    ///
    /// # Arguments
    /// * `chunks` - Vector of retrieval chunks to load
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if loading fails
    pub async fn load_chunks(&self, chunks: Vec<RetrievalChunk>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_chunks")?;
        self.run_rows_with_batch(&query_str, chunks, batch_size)
            .await
    }

    pub async fn load_status_events(
        &self,
        status_events: Vec<StatusEvent>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_status_events")?;
        self.run_rows_with_batch(&query_str, status_events, batch_size)
            .await
    }

    pub async fn load_source_notes(
        &self,
        source_notes: Vec<SourceNote>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_source_notes")?;
        self.run_rows_with_batch(&query_str, source_notes, batch_size)
            .await
    }

    pub async fn load_chapter_toc_entries(
        &self,
        rows: Vec<ChapterTocEntry>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_chapter_toc_entries")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_reserved_ranges(
        &self,
        rows: Vec<ReservedRange>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_reserved_ranges")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_parser_diagnostics(
        &self,
        rows: Vec<ParserDiagnostic>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_parser_diagnostics")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_temporal_effects(
        &self,
        rows: Vec<TemporalEffect>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_temporal_effects")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_lineage_events(
        &self,
        rows: Vec<LineageEvent>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_lineage_events")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_amendments(
        &self,
        amendments: Vec<Amendment>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_amendments")?;
        self.run_rows_with_batch(&query_str, amendments, batch_size)
            .await
    }

    pub async fn load_session_laws(
        &self,
        session_laws: Vec<SessionLaw>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_session_laws")?;
        self.run_rows_with_batch(&query_str, session_laws, batch_size)
            .await
    }

    pub async fn load_time_intervals(
        &self,
        intervals: Vec<TimeInterval>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_time_intervals")?;
        self.run_rows_with_batch(&query_str, intervals, batch_size)
            .await
    }

    pub async fn load_defined_terms(
        &self,
        terms: Vec<DefinedTerm>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_defined_terms")?;
        self.run_rows_with_batch(&query_str, terms, batch_size)
            .await
    }

    pub async fn load_definitions(
        &self,
        definitions: Vec<Definition>,
        batch_size: usize,
    ) -> Result<()> {
        let original_count = definitions.len();
        // Deduplicate by definition_id to prevent duplicate nodes
        let definitions = Self::deduplicate_by_key(definitions, |d| d.definition_id.clone());
        Self::log_dedup_stats("Definition", original_count, definitions.len());

        let query_str = Self::load_query("load_definitions")?;
        self.run_rows_with_batch(&query_str, definitions, batch_size)
            .await
    }

    pub async fn load_definition_scopes(
        &self,
        scopes: Vec<DefinitionScope>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_definition_scopes")?;
        self.run_rows_with_batch(&query_str, scopes, batch_size)
            .await
    }

    pub async fn load_legal_semantic_nodes(
        &self,
        nodes: Vec<LegalSemanticNode>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_legal_semantic_nodes")?;
        self.run_rows_with_batch(&query_str, nodes, batch_size)
            .await
    }

    pub async fn load_obligations(
        &self,
        obligations: Vec<Obligation>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_obligations")?;
        self.run_rows_with_batch(&query_str, obligations, batch_size)
            .await
    }

    pub async fn load_exceptions(
        &self,
        exceptions: Vec<Exception>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_exceptions")?;
        self.run_rows_with_batch(&query_str, exceptions, batch_size)
            .await
    }

    pub async fn load_deadlines(&self, deadlines: Vec<Deadline>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_deadlines")?;
        self.run_rows_with_batch(&query_str, deadlines, batch_size)
            .await
    }

    pub async fn load_penalties(&self, penalties: Vec<Penalty>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_penalties")?;
        self.run_rows_with_batch(&query_str, penalties, batch_size)
            .await
    }

    pub async fn load_remedies(&self, remedies: Vec<Remedy>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_remedies")?;
        self.run_rows_with_batch(&query_str, remedies, batch_size)
            .await
    }

    pub async fn load_money_amounts(
        &self,
        rows: Vec<MoneyAmount>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_money_amounts")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_tax_rules(&self, rows: Vec<TaxRule>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_tax_rules")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_rate_limits(&self, rows: Vec<RateLimit>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_rate_limits")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_required_notices(
        &self,
        rows: Vec<RequiredNotice>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_required_notices")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_form_texts(&self, rows: Vec<FormText>, batch_size: usize) -> Result<()> {
        let query_str = Self::load_query("load_form_texts")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_external_legal_citations(
        &self,
        rows: Vec<EnrichedCitation>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_external_legal_citations")?;
        self.run_rows_with_batch(&query_str, rows, batch_size).await
    }

    pub async fn load_legal_actors(
        &self,
        actors: Vec<LegalActor>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_legal_actors")?;
        self.run_rows_with_batch(&query_str, actors, batch_size)
            .await
    }

    pub async fn load_legal_actions(
        &self,
        actions: Vec<LegalAction>,
        batch_size: usize,
    ) -> Result<()> {
        let query_str = Self::load_query("load_legal_actions")?;
        self.run_rows_with_batch(&query_str, actions, batch_size)
            .await
    }

    /// Creates structural relationships between corpus, chapters, and sections.
    ///
    /// This method creates edges linking corpus editions to chapters, chapters to sections,
    /// and chapters to headings.
    ///
    /// # Arguments
    /// * `edition_year` - The edition year for filtering relationships
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_structural_edges(
        &self,
        edition_year: i32,
        relationship_batch_size: usize,
    ) -> Result<()> {
        let edition_id = format!("or:ors@{}", edition_year);
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_structural_edges")?,
            relationship_batch_size,
        );
        self.graph
            .run(
                query(&q)
                    .param("edition_id", edition_id)
                    .param("edition_year", edition_year as i64),
            )
            .await?;

        let heading_sections_q = Self::with_transaction_batch(
            Self::load_query("materialize_heading_sections")?,
            relationship_batch_size,
        );
        self.graph.run(query(&heading_sections_q)).await?;
        Ok(())
    }

    /// Creates relationships between legal text identities and their versions.
    ///
    /// Creates HAS_VERSION and VERSION_OF edges.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_identity_version_edges(
        &self,
        relationship_batch_size: usize,
    ) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_identity_version_edges")?,
            relationship_batch_size,
        );
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Updates the `current` flag on `LegalTextVersion` nodes.
    /// Only the version with the maximum `edition_year` for a given `LegalTextIdentity` will have `current = true`.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if the operation fails
    pub async fn enforce_current_flags(&self) -> Result<()> {
        let q = "
            MATCH (lti:LegalTextIdentity)-[:HAS_VERSION]->(ltv:LegalTextVersion)
            WITH lti, max(ltv.edition_year) AS max_year
            MATCH (lti)-[:HAS_VERSION]->(ltv2:LegalTextVersion)
            SET ltv2.current = (ltv2.edition_year = max_year)
        ";
        self.graph.run(query(q)).await?;
        Ok(())
    }

    /// Creates relationships between legal text versions and their provisions.
    ///
    /// Creates CONTAINS and PART_OF_VERSION edges.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_version_provision_edges(
        &self,
        relationship_batch_size: usize,
    ) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_version_provision_edges")?,
            relationship_batch_size,
        );
        self.graph.run(query(&q)).await?;
        Ok(())
    }

    /// Creates hierarchical relationships between provisions.
    ///
    /// Creates HAS_PARENT, CONTAINS, NEXT, and PREVIOUS edges to establish
    /// the provision hierarchy and ordering.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_provision_hierarchy_edges(
        &self,
        relationship_batch_size: usize,
    ) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_provision_hierarchy_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    /// Creates relationships between chunks and their source provisions/versions.
    ///
    /// Links retrieval chunks to provisions (for contextual chunks) and versions
    /// (for full statute chunks).
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_chunk_edges(&self, relationship_batch_size: usize) -> Result<()> {
        // Use smaller batch size for chunk relationships to avoid memory pool errors
        let chunk_batch_size = relationship_batch_size.min(500);

        let provision_q = Self::with_transaction_batch(
            "
            CALL {
                MATCH (c:RetrievalChunk)
                WHERE c.chunk_type <> 'full_statute'
                MATCH (p:Provision {provision_id: coalesce(c.source_provision_id, c.source_id)})
                MERGE (c)-[:DERIVED_FROM]->(p)
                MERGE (p)-[:HAS_CHUNK]->(c)
            } IN TRANSACTIONS OF 1000 ROWS
        "
            .to_string(),
            chunk_batch_size,
        );
        self.graph.run(query(&provision_q)).await?;

        let version_q = Self::with_transaction_batch(
            "
            CALL {
                MATCH (c:RetrievalChunk {chunk_type: 'full_statute'})
                MATCH (ltv:LegalTextVersion {version_id: coalesce(c.source_version_id, c.parent_version_id, c.source_id)})
                MERGE (c)-[:DERIVED_FROM]->(ltv)
                MERGE (ltv)-[:HAS_STATUTE_CHUNK]->(c)
            } IN TRANSACTIONS OF 1000 ROWS
        ".to_string(),
            chunk_batch_size,
        );
        self.graph.run(query(&version_q)).await?;
        Ok(())
    }

    /// Creates relationships between source documents and derived entities.
    ///
    /// This consolidated method creates all source-related edges in a single operation:
    /// - Public body to source documents
    /// - Legal text versions to source documents
    /// - Provisions to source documents (via versions)
    /// - Citation mentions to source documents (via provisions)
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_source_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let consolidated_q = Self::with_transaction_batch(
            Self::load_query("materialize_source_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&consolidated_q).await?;
        Ok(())
    }

    /// Creates relationships from citation mentions to their resolved targets.
    ///
    /// This consolidated method creates all citation-related edges:
    /// - Mentions to source provisions
    /// - Mentions to resolved legal text identities
    /// - Mentions to resolved legal text versions
    /// - Mentions to resolved provisions
    /// - Mentions to resolved chapters
    /// - Mentions to range start/end identities
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if creation fails
    pub async fn materialize_citation_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let consolidated_q = Self::with_transaction_batch(
            Self::load_query("materialize_citation_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&consolidated_q).await?;

        let resolves_range_q = Self::with_transaction_batch(
            Self::load_query("materialize_citation_range_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&resolves_range_q).await?;
        Ok(())
    }

    pub async fn materialize_semantic_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_semantic_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    pub async fn materialize_definition_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_definition_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    pub async fn materialize_obligation_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_obligation_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    pub async fn materialize_history_edges(&self, relationship_batch_size: usize) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_history_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    pub async fn materialize_specialized_edges(
        &self,
        relationship_batch_size: usize,
    ) -> Result<()> {
        let q = Self::with_transaction_batch(
            Self::load_query("materialize_specialized_edges")?,
            relationship_batch_size,
        );
        self.run_multi_statement_query(&q).await?;
        Ok(())
    }

    /// Create citation edges efficiently using a single consolidated query.
    /// Uses CASE expressions to route edges to appropriate relationship types.
    /// ~40% fewer round-trips vs separate queries.
    pub async fn create_cites_edges(&self, edges: Vec<CitesEdge>, batch_size: usize) -> Result<()> {
        let consolidated_q = Self::load_query("create_cites_edges")?;
        self.run_rows_with_batch(&consolidated_q, edges, batch_size)
            .await?;

        let version_cites_q = "
            MATCH (ltv:LegalTextVersion)-[:CONTAINS]->(:Provision)-[r:CITES]->(target:LegalTextIdentity)
            MERGE (ltv)-[vr:CITES {via_citation_mention_id: r.via_citation_mention_id}]->(target)
            SET vr += properties(r)
            REMOVE vr.edge_id
        ";
        self.graph.run(query(version_cites_q)).await?;

        let identity_cites_q = "
            MATCH (lti:LegalTextIdentity)-[:HAS_VERSION]->(:LegalTextVersion)-[:CONTAINS]->(:Provision)-[r:CITES]->(target:LegalTextIdentity)
            MERGE (lti)-[ir:CITES {via_citation_mention_id: r.via_citation_mention_id}]->(target)
            SET ir += properties(r)
            REMOVE ir.edge_id
        ";
        self.graph.run(query(identity_cites_q)).await?;
        Ok(())
    }

    /// Get nodes that need embedding, with input text generated in Cypher.
    pub async fn get_embedding_targets(
        &self,
        label: &str,
        model: &str,
        dimension: i32,
        limit: usize,
        edition_year: i32,
        resume_existing: bool,
    ) -> Result<Vec<(String, String, String)>> {
        let needs_embedding_predicate = if resume_existing {
            "n.embedding IS NULL
              OR n.embedding_model IS NULL OR n.embedding_model <> $model
              OR n.embedding_dim IS NULL OR n.embedding_dim <> $dimension
              OR n.embedding_input_hash IS NULL
              OR n.embedded_input_hash IS NULL
              OR n.embedded_input_hash <> n.embedding_input_hash"
        } else {
            "n.embedding IS NULL"
        };
        let q = match label {
            "RetrievalChunk" => format!(
                "MATCH (n:RetrievalChunk)
                 WHERE n.embedding_policy IN ['embed_primary', 'embed_special']
                   AND (n.text IS NOT NULL AND n.text <> '')
                   AND coalesce(n.token_count, 0) <= 30000
                   AND ({needs_embedding_predicate})
                 RETURN n.chunk_id AS id, n.text AS text, n.embedding_input_hash AS hash
                 ORDER BY n.chunk_id LIMIT $limit"
            ),
            "Provision" => format!(
                "MATCH (n:Provision)
                 WHERE (n.text IS NOT NULL AND n.text <> '')
                   AND ({needs_embedding_predicate})
                 RETURN n.provision_id AS id,
                        'Oregon Revised Statutes. ' + $year + ' Edition.\\nCitation: ' + n.display_citation + 
                        '\\nProvision type: ' + n.provision_type + '.\\nStatus: active.\\nText:\\n' + n.text AS text,
                        n.embedding_input_hash AS hash
                 ORDER BY n.provision_id LIMIT $limit"
            ),
            "LegalTextVersion" => format!(
                "MATCH (n:LegalTextVersion)
                 WHERE (n.text IS NOT NULL AND n.text <> '')
                   AND ({needs_embedding_predicate})
                 RETURN n.version_id AS id,
                        'Oregon Revised Statutes. ' + $year + ' Edition.\\nCitation: ' + n.citation + 
                        '\\nTitle: ' + coalesce(n.title, '') + '\\nStatus: ' + n.status + 
                        '\\nText:\\n' + n.text AS text,
                        n.embedding_input_hash AS hash
                 ORDER BY n.version_id LIMIT $limit"
            ),
            _ => return Err(anyhow::anyhow!("Unsupported embedding target label: {}", label)),
        };

        let mut result = self
            .graph
            .execute(
                query(&q)
                    .param("model", model.to_string())
                    .param("dimension", dimension as i64)
                    .param("limit", limit as i64)
                    .param("year", edition_year as i64),
            )
            .await?;

        let mut nodes = Vec::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("id")?;
            let text: String = row.get("text")?;
            let hash: String = row.get("hash")?;
            nodes.push((id, text, hash));
        }
        Ok(nodes)
    }

    /// Bulk lookup to check which chunk IDs already have current embeddings.
    /// Returns a HashSet of chunk IDs that don't need re-embedding (hash matches).
    /// Useful for resume scenarios to avoid re-embedding unchanged content.
    pub async fn filter_chunks_needing_embedding(
        &self,
        chunk_hashes: Vec<(String, String)>, // (chunkId, currentHash)
    ) -> Result<Vec<(String, String)>> {
        // Returns (chunkId, currentHash) that need embedding
        let q = "
            UNWIND $candidates AS candidate
            MATCH (c:RetrievalChunk {chunk_id: candidate.chunkId})
            WHERE c.embedding IS NULL 
               OR c.embedded_input_hash IS NULL
               OR c.embedded_input_hash <> candidate.hash
            RETURN candidate.chunkId AS chunkId, candidate.hash AS hash
        ";

        // Process in batches to reduce memory overhead for large datasets
        const BATCH_SIZE: usize = 5000;
        let mut needs_embedding = Vec::new();

        for batch in chunk_hashes.chunks(BATCH_SIZE) {
            let candidates: Vec<neo4rs::BoltType> = batch
                .iter()
                .map(|(chunk_id, hash)| {
                    let json = serde_json::json!({"chunkId": chunk_id, "hash": hash});
                    neo4j_value_to_bolt(json)
                })
                .collect();

            let mut result = self
                .graph
                .execute(query(q).param("candidates", candidates))
                .await?;

            while let Some(row) = result.next().await? {
                let id: String = row.get("chunkId")?;
                let hash: String = row.get("hash")?;
                needs_embedding.push((id, hash));
            }
        }

        Ok(needs_embedding)
    }

    /// Get embedding statistics for monitoring progress.
    /// Returns (total_chunks, embedded_chunks, pending_chunks, outdated_chunks).
    pub async fn get_embedding_stats(
        &self,
        label: &str,
        model: &str,
        dimension: i32,
    ) -> Result<(i64, i64, i64, i64)> {
        let policy_filter = if label == "RetrievalChunk" {
            "AND n.embedding_policy IN ['embed_primary', 'embed_special']"
        } else {
            ""
        };

        let stats_q = format!(
            "
            MATCH (n:{label})
            WITH 
                count(*) AS total,
                count(CASE WHEN n.embedding IS NOT NULL THEN 1 END) AS embedded,
                count(CASE WHEN n.embedding IS NULL {policy_filter} THEN 1 END) AS pending,
                count(CASE WHEN n.embedding IS NOT NULL AND (
                    n.embedding_model IS NULL OR n.embedding_model <> $model
                    OR n.embedding_dim IS NULL OR n.embedding_dim <> $dimension
                    OR n.embedding_input_hash IS NULL
                    OR n.embedded_input_hash IS NULL
                    OR n.embedded_input_hash <> n.embedding_input_hash
                ) THEN 1 END) AS outdated
            RETURN total, embedded, pending, outdated
        "
        );

        let mut result = self
            .graph
            .execute(
                query(&stats_q)
                    .param("model", model.to_string())
                    .param("dimension", dimension as i64),
            )
            .await?;
        if let Some(row) = result.next().await? {
            let total: i64 = row.get("total")?;
            let embedded: i64 = row.get("embedded")?;
            let pending: i64 = row.get("pending")?;
            let outdated: i64 = row.get("outdated")?;
            Ok((total, embedded, pending, outdated))
        } else {
            Ok((0, 0, 0, 0))
        }
    }

    /// Streaming variant of vector search for memory-efficient large result sets.
    /// Returns a stream of (chunk_id, text, citation, score) tuples.
    /// Note: This returns neo4rs::Error, not anyhow::Error. Use map_err to convert if needed.
    pub async fn vector_search_stream(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<
        impl futures::Stream<
            Item = std::result::Result<(String, String, Option<String>, f64), neo4rs::Error>,
        >,
    > {
        let q = "
            MATCH (n:RetrievalChunk)
              SEARCH n IN (
                VECTOR INDEX retrieval_chunk_embedding_1024
                FOR $embedding
                LIMIT $limit
              ) SCORE AS similarityScore
            RETURN n.chunk_id AS chunkId,
                   n.text AS text,
                   n.citation AS citation,
                   similarityScore
            ORDER BY similarityScore DESC
        ";

        let result = self
            .graph
            .execute(
                query(q)
                    .param("embedding", embedding)
                    .param("limit", limit as i64),
            )
            .await?;

        use futures::stream::unfold;
        let stream = unfold(result, |mut result| async move {
            match result.next().await {
                Ok(Some(row)) => {
                    let id: String = row.get("chunkId").ok()?;
                    let text: String = row.get("text").ok()?;
                    let citation: Option<String> = row.get("citation").ok()?;
                    let score: f64 = row.get("similarityScore").ok()?;
                    Some((Ok((id, text, citation, score)), result))
                }
                Ok(None) => None,
                Err(e) => Some((Err(e), result)),
            }
        });

        Ok(stream)
    }

    /// Perform vector similarity search using Cypher 25 SEARCH clause with in-index filtering.
    ///
    /// Uses the modern SEARCH syntax which is the replacement for deprecated db.index.vector.queryNodes().
    /// Supports in-index filtering via WHERE inside SEARCH for better performance than post-filtering.
    ///
    /// # Arguments
    /// * `embedding` - Query vector for similarity search
    /// * `limit` - Number of results to return (top-k)
    /// * `answer_policy` - Optional filter for answerPolicy property (in-index filtering)
    /// * `over_fetch` - If true, fetches 2x results from index then limits after post-filtering
    ///
    /// # Returns
    /// Vec of (chunk_id, text, citation, similarity_score)
    ///
    /// # Example
    /// ```ignore
    /// let results = loader.vector_search(embedding, 10, Some("answerable"), true).await?;
    /// ```
    pub async fn vector_search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
        answer_policy: Option<&str>,
        over_fetch: bool,
    ) -> Result<Vec<(String, String, Option<String>, f64)>> {
        // Use over-fetching pattern: fetch more from index if post-filtering might reduce results
        let search_limit = if over_fetch { limit * 2 } else { limit };

        let mut q = "
            MATCH (n:RetrievalChunk)
              SEARCH n IN (
                VECTOR INDEX retrieval_chunk_embedding_1024
                FOR $embedding
                WHERE n.citation IS NOT NULL"
            .to_string();

        // In-index filtering via WHERE inside SEARCH clause
        if answer_policy.is_some() {
            q.push_str(" AND n.answer_policy = $answerPolicy");
        }

        // Add SCORE AS to get similarity scores (0.0 to 1.0, where 1.0 is most similar)
        q.push_str(&format!(
            "
                LIMIT {search_limit}
              ) SCORE AS similarityScore
            RETURN n.chunk_id AS chunkId,
                   n.text AS text,
                   n.citation AS citation,
                   similarityScore
            ORDER BY similarityScore DESC
            LIMIT {return_limit}",
            search_limit = search_limit,
            return_limit = limit
        ));

        let query_builder = query(&q).param("embedding", embedding);
        let query_builder = if let Some(policy) = answer_policy {
            query_builder.param("answerPolicy", policy)
        } else {
            query_builder
        };

        let mut result = self.graph.execute(query_builder).await?;
        let mut chunks = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("chunkId")?;
            let text: String = row.get("text")?;
            let citation: Option<String> = row.get("citation")?;
            let score: f64 = row.get("similarityScore")?;
            chunks.push((id, text, citation, score));
        }

        Ok(chunks)
    }

    /// Perform hybrid search (Vector + Full-text) using Reciprocal Rank Fusion (RRF).
    ///
    /// This is the "gold standard" for legal RAG, combining semantic similarity with keyword precision.
    ///
    /// # Arguments
    /// * `query_text` - Original text query for full-text search
    /// * `embedding` - Query vector for similarity search
    /// * `limit` - Number of results to return
    /// * `k` - RRF constant (default 60.0)
    pub async fn hybrid_search(
        &self,
        query_text: &str,
        embedding: Vec<f32>,
        limit: usize,
        k: f64,
    ) -> Result<Vec<(String, String, Option<String>, f64)>> {
        let q = Self::load_query("hybrid_search")?;

        let mut result = self
            .graph
            .execute(
                query(&q)
                    .param("embedding", embedding)
                    .param("query_text", query_text)
                    .param("k", k)
                    .param("limit", limit as i64),
            )
            .await?;

        let mut chunks = Vec::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("chunk_id")?;
            let text: String = row.get("text")?;
            let citation: Option<String> = row.get("citation")?;
            let score: f64 = row.get("rrf_score")?;
            chunks.push((id, text, citation, score));
        }

        Ok(chunks)
    }

    /// Perform a standalone full-text search.
    pub async fn fulltext_search(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, Option<String>, f64)>> {
        let q = "
            CALL db.index.fulltext.queryNodes('legalTextFulltext', $query_text) YIELD node AS n, score
            WHERE n:RetrievalChunk
            RETURN n.chunk_id AS chunk_id,
                   n.text AS text,
                   n.citation AS citation,
                   score
            ORDER BY score DESC
            LIMIT $limit
        ";

        let mut result = self
            .graph
            .execute(
                query(q)
                    .param("query_text", query_text)
                    .param("limit", limit as i64),
            )
            .await?;

        let mut chunks = Vec::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("chunk_id")?;
            let text: String = row.get("text")?;
            let citation: Option<String> = row.get("citation")?;
            let score: f64 = row.get("score")?;
            chunks.push((id, text, citation, score));
        }

        Ok(chunks)
    }

    /// Enrich a set of chunks with multi-hop context (citations, definitions, status) in a single batch query.
    pub async fn get_enriched_context(
        &self,
        chunks: Vec<(String, String, Option<String>, f64)>,
    ) -> Result<Vec<EnrichedChunk>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Map meta data for reconstruction
        let mut meta_map = std::collections::HashMap::new();
        let mut ids = Vec::new();
        for (id, text, citation, score) in chunks {
            ids.push(id.clone());
            meta_map.insert(id.clone(), (text, citation, score));
        }

        let q = "
            UNWIND $ids AS id
            MATCH (c:RetrievalChunk {chunk_id: id})
            OPTIONAL MATCH (c)-[:DERIVED_FROM]->(p:Provision)
            OPTIONAL MATCH (p)-[:PART_OF]->(ltv:LegalTextVersion)
            
            // Batch fetch citations for this provision
            CALL {
                WITH p
                OPTIONAL MATCH (p)-[:MENTIONS_CITATION]->(cm:CitationMention)
                OPTIONAL MATCH (cm)-[:RESOLVES_TO_PROVISION]->(target:Provision)
                WITH cm, target
                FILTER cm IS NOT NULL AND target IS NOT NULL
                RETURN collect({
                    citation: cm.normalized_citation,
                    target_citation: target.display_citation,
                    target_text: target.text
                })[..5] AS citations_list
            }
            
            // Batch fetch definitions for this provision
            CALL {
                WITH p
                OPTIONAL MATCH (p)-[:DEFINES]->(d:Definition)
                OPTIONAL MATCH (d)-[:DEFINES_TERM]->(dt:DefinedTerm)
                WITH d, dt
                FILTER d IS NOT NULL AND dt IS NOT NULL
                RETURN collect({
                    term: dt.term,
                    definition: d.definition_text
                }) AS definitions_list
            }
            
            RETURN c.chunk_id AS chunk_id,
                   c.breadcrumb AS breadcrumb,
                   ltv.edition_year AS edition_year,
                   ltv.status AS status,
                   citations_list,
                   definitions_list
        ";

        let mut result = self.graph.execute(query(q).param("ids", ids)).await?;
        let mut enriched = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("chunk_id")?;
            let breadcrumb: String = row.get("breadcrumb")?;
            let edition_year: Option<i32> = row.get("edition_year").ok();
            let status: Option<String> = row.get("status").ok();

            // Extract Citations from the list of maps
            let citations_raw: Vec<std::collections::BTreeMap<String, neo4rs::BoltType>> =
                row.get("citations_list").unwrap_or_default();
            let citations = citations_raw
                .into_iter()
                .map(|map| parse_enriched_citation(&map))
                .collect();

            // Extract Definitions from the list of maps
            let definitions_raw: Vec<std::collections::BTreeMap<String, neo4rs::BoltType>> =
                row.get("definitions_list").unwrap_or_default();
            let definitions = definitions_raw
                .into_iter()
                .map(|map| parse_enriched_definition(&map))
                .collect();

            if let Some((text, citation, score)) = meta_map.remove(&id) {
                enriched.push(EnrichedChunk {
                    chunk_id: id,
                    text,
                    citation,
                    breadcrumb,
                    score,
                    citations,
                    definitions,
                    status,
                    edition_year,
                });
            }
        }

        Ok(enriched)
    }

    /// Store embeddings as LIST<FLOAT> properties.
    pub async fn update_embeddings(&self, updates: Vec<EmbeddingUpdate>) -> Result<()> {
        self.update_node_embeddings("RetrievalChunk", "chunk_id", updates)
            .await
    }

    /// Generic method to update node embeddings.
    pub async fn update_node_embeddings(
        &self,
        label: &str,
        id_field: &str,
        updates: Vec<EmbeddingUpdate>,
    ) -> Result<()> {
        let q = format!(
            "
            FOR row IN $rows
            CALL (row) {{
                MATCH (n:{label} {{{id_field}: row.chunk_id}})
                SET n.embedding = vector(row.embedding, row.embedding_dim, FLOAT32),
                    n.embedding_model = row.embedding_model,
                    n.embedding_dim = row.embedding_dim,
                    n.embedding_input_hash = row.embedding_input_hash,
                    n.embedded_input_hash = row.embedding_input_hash,
                    n.embedded_at = datetime(),
                    n.embedding_input_type = 'document',
                    n.embedding_output_dtype = row.embedding_output_dtype,
                    n.embedding_profile = row.embedding_profile,
                    n.embedding_source_dimension = row.embedding_source_dimension
            }} IN 8 CONCURRENT TRANSACTIONS OF 100 ROWS
        "
        );
        self.run_rows(&q, updates).await
    }

    /// Warm up the vector index by running a dummy query.
    /// This loads the index into memory for faster subsequent queries.
    /// Community Edition compatible.
    pub async fn warmup_vector_index(&self) -> Result<()> {
        let warmup_q = "
            MATCH (n:RetrievalChunk)
              SEARCH n IN (
                VECTOR INDEX retrieval_chunk_embedding_1024
                FOR $embedding
                WHERE n.citation IS NOT NULL AND n.answer_policy = 'answerable'
                LIMIT 1
              ) SCORE AS similarityScore
            RETURN n.chunk_id, similarityScore
            LIMIT 1
        ";
        let dummy_embedding: Vec<f32> = vec![0.0; 1024];
        let _ = self
            .graph
            .execute(query(warmup_q).param("embedding", dummy_embedding))
            .await?;
        tracing::info!("Vector index warmed up");
        Ok(())
    }

    /// Verifies that the vector index exists and has the correct dimensions.
    ///
    /// # Arguments
    /// * `expected_dim` - The expected embedding dimension (e.g., 1024)
    ///
    /// # Returns
    /// Ok(()) if the index is valid, or an error if missing or mismatched
    pub async fn verify_vector_index(&self, expected_dim: i32) -> Result<()> {
        let q = "
            SHOW VECTOR INDEXES YIELD name, options
            WHERE name = 'retrieval_chunk_embedding_1024'
            RETURN options.indexConfig.`vector.dimensions` AS dim
        ";
        let mut result = self.graph.execute(query(q)).await?;
        if let Some(row) = result.next().await? {
            let dim: i64 = row.get("dim")?;
            if dim != expected_dim as i64 {
                anyhow::bail!(
                    "Vector index dimension mismatch: expected {}, found {}",
                    expected_dim,
                    dim
                );
            }
            info!(
                "✓ Vector index 'retrieval_chunk_embedding_1024' verified (dim: {})",
                dim
            );
        } else {
            anyhow::bail!(
                "Vector index 'retrieval_chunk_embedding_1024' not found. Run with --create-vector-index."
            );
        }
        Ok(())
    }

    pub async fn create_vector_index_for_profile(&self, profile: &EmbeddingProfile) -> Result<()> {
        let constraint_name = format!("{}_embedding_vector_type", profile.label.to_lowercase());
        let constraint = format!(
            "CREATE CONSTRAINT {constraint_name} IF NOT EXISTS \
             FOR (n:{label}) \
             REQUIRE n.{property} IS :: VECTOR<FLOAT32>({dimension})",
            constraint_name = constraint_name,
            label = profile.label,
            property = profile.neo4j_property,
            dimension = profile.output_dimension,
        );
        self.graph.run(query(&constraint)).await?;

        let index = format!(
            "CREATE VECTOR INDEX {index_name} IF NOT EXISTS \
             FOR (n:{label}) \
             ON n.{property} \
             OPTIONS {{ indexConfig: {{ `vector.dimensions`: {dimension}, `vector.similarity_function`: 'cosine' }} }}",
            index_name = profile.neo4j_index_name,
            label = profile.label,
            property = profile.neo4j_property,
            dimension = profile.output_dimension,
        );
        self.graph.run(query(&index)).await?;
        Ok(())
    }

    pub async fn fetch_embedding_candidates(
        &self,
        spec: &EmbeddingTargetSpec,
        edition_year: i32,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<EmbeddingCandidate>> {
        let q = format!(
            "
            MATCH (n:{label})
            OPTIONAL MATCH (n)-[:SUPPORTED_BY]->(p:Provision)
            WHERE {where_clause}
            RETURN n.{id_property} AS id,
                   {input_expr} AS input_text,
                   n.embedding IS NOT NULL AS has_embedding,
                   n.embedding_profile AS embedding_profile,
                   n.embedding_model AS embedding_model,
                   n.embedding_dim AS embedding_dim,
                   n.embedding_output_dtype AS embedding_output_dtype,
                   n.embedding_input_hash AS embedding_input_hash
            ORDER BY id
            SKIP $offset
            LIMIT $limit
            ",
            label = spec.label,
            id_property = spec.id_property,
            where_clause = spec.where_clause,
            input_expr = spec.input_expr,
        );

        let mut result = self
            .graph
            .execute(
                query(&q)
                    .param("edition_year", edition_year as i64)
                    .param("offset", offset as i64)
                    .param("limit", limit as i64),
            )
            .await?;
        let mut rows = Vec::new();
        while let Some(row) = result.next().await? {
            let embedding_dim = row.get::<i64>("embedding_dim").ok().map(|dim| dim as i32);
            rows.push(EmbeddingCandidate {
                id: row.get("id")?,
                input_text: row.get("input_text")?,
                has_embedding: row.get("has_embedding")?,
                embedding_profile: row.get("embedding_profile").ok(),
                embedding_model: row.get("embedding_model").ok(),
                embedding_dim,
                embedding_output_dtype: row.get("embedding_output_dtype").ok(),
                embedding_input_hash: row.get("embedding_input_hash").ok(),
            });
        }
        Ok(rows)
    }

    /// Vector search with multiple filter criteria.
    /// Supports filtering by chunkType, answerPolicy, citation presence, etc.
    /// Community Edition compatible - uses Cypher 25 SEARCH with WHERE clause.
    ///
    /// # Modern Cypher 25 SEARCH Example
    /// ```cypher
    /// MATCH (n:RetrievalChunk)
    /// SEARCH n IN (
    ///   VECTOR INDEX retrieval_chunk_embedding_1024
    ///   FOR $embedding
    ///   WHERE n.answer_policy = 'authoritative_support'
    ///     AND n.chunk_type IN ['contextual_provision', 'definition_block', 'exception_block']
    ///     AND n.edition_year = 2025
    ///   LIMIT 30
    /// ) SCORE AS similarityScore
    /// RETURN n.chunk_id, n.text, n.citation, n.chunk_type, n.answer_policy, similarityScore
    /// ORDER BY similarityScore DESC
    /// LIMIT 10;
    /// ```
    pub async fn vector_search_filtered(
        &self,
        embedding: Vec<f32>,
        chunk_type: Option<&str>,
        answer_policy: Option<&str>,
        edition_year: Option<i32>,
        authority_level: Option<i32>,
        require_citation: bool,
        limit: usize,
        over_fetch: bool,
    ) -> Result<Vec<(String, String, Option<String>, f64)>> {
        let search_limit = if over_fetch { limit * 2 } else { limit };
        let mut q = "
            MATCH (n:RetrievalChunk)
              SEARCH n IN (
                VECTOR INDEX retrieval_chunk_embedding_1024
                FOR $embedding
                WHERE n.citation IS NOT NULL"
            .to_string();

        if chunk_type.is_some() {
            q.push_str(" AND n.chunk_type = $chunkType");
        }
        if answer_policy.is_some() {
            q.push_str(" AND n.answer_policy = $answerPolicy");
        }
        if edition_year.is_some() {
            q.push_str(" AND n.edition_year = $editionYear");
        }
        if authority_level.is_some() {
            q.push_str(" AND n.authority_level = $authorityLevel");
        }
        if require_citation {
            q.push_str(" AND n.citation <> ''");
        }

        q.push_str(&format!(
            "
                    LIMIT {search_limit}
                  ) SCORE AS similarityScore
                RETURN n.chunk_id AS chunkId,
                       n.text AS text,
                       n.citation AS citation,
                       similarityScore
                ORDER BY similarityScore DESC
                LIMIT {return_limit}",
            search_limit = search_limit,
            return_limit = limit
        ));

        let mut query_builder = query(&q).param("embedding", embedding);
        if let Some(ct) = chunk_type {
            query_builder = query_builder.param("chunkType", ct);
        }
        if let Some(ap) = answer_policy {
            query_builder = query_builder.param("answerPolicy", ap);
        }
        if let Some(ey) = edition_year {
            query_builder = query_builder.param("editionYear", ey as i64);
        }
        if let Some(al) = authority_level {
            query_builder = query_builder.param("authorityLevel", al as i64);
        }

        let mut result = self.graph.execute(query_builder).await?;
        let mut chunks = Vec::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("chunkId")?;
            let text: String = row.get("text")?;
            let citation: Option<String> = row.get("citation")?;
            let score: f64 = row.get("similarityScore")?;
            chunks.push((id, text, citation, score));
        }

        Ok(chunks)
    }

    async fn run_rows<T: serde::Serialize>(&self, query_str: &str, rows: Vec<T>) -> Result<()> {
        self.run_rows_with_batch(query_str, rows, DEFAULT_EMBEDDING_UPDATE_BATCH_SIZE)
            .await
    }

    async fn run_rows_with_batch<T: serde::Serialize>(
        &self,
        query_str: &str,
        rows: Vec<T>,
        batch_size: usize,
    ) -> Result<()> {
        let total_rows = rows.len();
        let batch_count = (total_rows + batch_size - 1) / batch_size;

        for (batch_idx, batch) in rows.chunks(batch_size).enumerate() {
            let rows = batch
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| {
                    format!(
                        "Failed to serialize batch {}/{}",
                        batch_idx + 1,
                        batch_count
                    )
                })?
                .into_iter()
                .map(neo4j_value_to_bolt)
                .collect::<Vec<_>>();

            self.graph
                .run(query(query_str).param("rows", rows))
                .await
                .with_context(|| {
                    format!(
                        "Failed to execute batch {}/{} ({} rows)",
                        batch_idx + 1,
                        batch_count,
                        batch.len()
                    )
                })?;

            if batch_idx > 0 && (batch_idx + 1) % 10 == 0 {
                info!(
                    "Processed {}/{} batches ({} rows)",
                    batch_idx + 1,
                    batch_count,
                    (batch_idx + 1) * batch_size.min(batch.len())
                );
            }
        }

        info!(
            "Completed {} batches ({} total rows)",
            batch_count, total_rows
        );
        Ok(())
    }

    /// Check database connectivity and get basic stats.
    /// Useful for health checks before starting operations.
    pub async fn health_check(&self) -> Result<(bool, i64, String)> {
        let q = "
            CALL dbms.components() YIELD name, versions, edition
            RETURN edition, versions[0] AS version
        ";
        let mut result = self.graph.execute(query(q)).await?;
        if let Some(row) = result.next().await? {
            let edition: String = row.get("edition")?;
            let version: String = row.get("version")?;
            let is_community = edition.to_lowercase().contains("community");
            Ok((is_community, 0, version))
        } else {
            Ok((false, 0, "unknown".to_string()))
        }
    }

    /// Checks if the Neo4j version supports the Cypher 25 SEARCH clause.
    /// Requires Neo4j 2025.01+ or 5.x with certain capabilities.
    /// This implementation assumes 5.15+ or any 2025+ version.
    pub fn supports_search_clause(version: &str) -> bool {
        if version.starts_with("2025.") || version.starts_with("2026.") {
            return true;
        }
        if let Some(major_part) = version.split('.').next() {
            if let Ok(major) = major_part.parse::<i32>() {
                if major >= 5 {
                    // Technically 5.15+ supports it as an experimental feature,
                    // but we'll be conservative and assume search is safer on modern 5.x.
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct EmbeddingUpdate {
    pub chunk_id: String,
    pub embedding: Vec<f32>,
    pub embedding_model: String,
    pub embedding_dim: i32,
    pub embedding_input_hash: String,
    pub embedding_profile: Option<String>,
    pub embedding_output_dtype: Option<String>,
    pub embedding_source_dimension: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingCandidate {
    pub id: String,
    pub input_text: String,
    pub has_embedding: bool,
    pub embedding_profile: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<i32>,
    pub embedding_output_dtype: Option<String>,
    pub embedding_input_hash: Option<String>,
}

fn extract_string_from_map(
    map: &std::collections::BTreeMap<String, neo4rs::BoltType>,
    key: &str,
) -> String {
    map.get(key)
        .and_then(|v| match v {
            neo4rs::BoltType::String(s) => Some(s.value.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn parse_enriched_citation(
    map: &std::collections::BTreeMap<String, neo4rs::BoltType>,
) -> EnrichedCitation {
    EnrichedCitation {
        citation: extract_string_from_map(map, "citation"),
        target_citation: extract_string_from_map(map, "target_citation"),
        target_text: extract_string_from_map(map, "target_text"),
    }
}

pub(crate) fn parse_enriched_definition(
    map: &std::collections::BTreeMap<String, neo4rs::BoltType>,
) -> EnrichedDefinition {
    EnrichedDefinition {
        term: extract_string_from_map(map, "term"),
        definition: extract_string_from_map(map, "definition"),
    }
}

fn neo4j_value_to_bolt(val: serde_json::Value) -> neo4rs::BoltType {
    match val {
        serde_json::Value::String(s) => neo4rs::BoltType::String(neo4rs::BoltString { value: s }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                neo4rs::BoltType::Integer(neo4rs::BoltInteger { value: i })
            } else if let Some(f) = n.as_f64() {
                neo4rs::BoltType::Float(neo4rs::BoltFloat { value: f })
            } else {
                neo4rs::BoltType::Null(neo4rs::BoltNull)
            }
        }
        serde_json::Value::Bool(b) => neo4rs::BoltType::Boolean(neo4rs::BoltBoolean { value: b }),
        serde_json::Value::Array(a) => {
            let list: Vec<neo4rs::BoltType> = a.into_iter().map(neo4j_value_to_bolt).collect();
            neo4rs::BoltType::List(neo4rs::BoltList { value: list })
        }
        serde_json::Value::Object(o) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in o {
                map.insert(neo4rs::BoltString { value: k }, neo4j_value_to_bolt(v));
            }
            neo4rs::BoltType::Map(neo4rs::BoltMap { value: map })
        }
        serde_json::Value::Null => neo4rs::BoltType::Null(neo4rs::BoltNull),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_candidate_selection_matches_resume_rules() {
        assert!(needs_embedding_for_metadata(
            false,
            Some("voyage-4-large"),
            Some(1024),
            Some("hash"),
            Some("hash"),
            "voyage-4-large",
            1024,
        ));
        assert!(needs_embedding_for_metadata(
            true,
            Some("old-model"),
            Some(1024),
            Some("hash"),
            Some("hash"),
            "voyage-4-large",
            1024,
        ));
        assert!(needs_embedding_for_metadata(
            true,
            Some("voyage-4-large"),
            Some(256),
            Some("hash"),
            Some("hash"),
            "voyage-4-large",
            1024,
        ));
        assert!(needs_embedding_for_metadata(
            true,
            Some("voyage-4-large"),
            Some(1024),
            Some("old-hash"),
            Some("hash"),
            "voyage-4-large",
            1024,
        ));
        assert!(!needs_embedding_for_metadata(
            true,
            Some("voyage-4-large"),
            Some(1024),
            Some("hash"),
            Some("hash"),
            "voyage-4-large",
            1024,
        ));
    }

    #[test]
    fn supports_search_clause_works_for_2025_plus() {
        use super::Neo4jLoader;
        assert!(Neo4jLoader::supports_search_clause("2025.01"));
        assert!(Neo4jLoader::supports_search_clause("2026.04"));
        assert!(Neo4jLoader::supports_search_clause("5.15.0"));
        assert!(Neo4jLoader::supports_search_clause("6.0.0"));
        assert!(!Neo4jLoader::supports_search_clause("4.4.0"));
    }

    #[test]
    fn parse_enriched_citation_handles_bolt_types() {
        use super::parse_enriched_citation;
        use neo4rs::{BoltString, BoltType};
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        map.insert(
            "citation".to_string(),
            BoltType::String(BoltString {
                value: "ORS 1.001".to_string(),
            }),
        );
        map.insert(
            "target_citation".to_string(),
            BoltType::String(BoltString {
                value: "ORS 2.002".to_string(),
            }),
        );
        map.insert(
            "target_text".to_string(),
            BoltType::String(BoltString {
                value: "Target text".to_string(),
            }),
        );

        let enriched = parse_enriched_citation(&map);
        assert_eq!(enriched.citation, "ORS 1.001");
        assert_eq!(enriched.target_citation, "ORS 2.002");
        assert_eq!(enriched.target_text, "Target text");
    }

    #[test]
    fn parse_enriched_definition_handles_bolt_types() {
        use super::parse_enriched_definition;
        use neo4rs::{BoltString, BoltType};
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        map.insert(
            "term".to_string(),
            BoltType::String(BoltString {
                value: "term".to_string(),
            }),
        );
        map.insert(
            "definition".to_string(),
            BoltType::String(BoltString {
                value: "definition text".to_string(),
            }),
        );

        let enriched = parse_enriched_definition(&map);
        assert_eq!(enriched.term, "term");
        assert_eq!(enriched.definition, "definition text");
    }

    #[test]
    fn test_with_transaction_batch() {
        let query = "UNWIND $rows AS row :transaction".to_string();
        let result = Neo4jLoader::with_transaction_batch(query, 5000);
        assert!(result.contains("CALL { WITH row"));
        assert!(result.contains("} IN TRANSACTIONS OF 5000 ROWS"));
    }

    #[test]
    fn normalize_cypher_statement_strips_comments_without_dropping_statement() {
        let statement = r#"
            // Create source citation links

            CALL {
                MATCH (cm:CitationMention)
                MATCH (p:Provision {provision_id: cm.source_provision_id})
                MERGE (p)-[:MENTIONS_CITATION]->(cm)
            } IN TRANSACTIONS OF 5000 ROWS
        "#;

        let normalized = super::normalize_cypher_statement(statement).expect("statement");
        assert!(normalized.starts_with("CALL {"));
        assert!(normalized.contains("MENTIONS_CITATION"));
        assert!(!normalized.contains("//"));
    }
}
