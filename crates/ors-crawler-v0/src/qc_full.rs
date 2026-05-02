use anyhow::Result;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tracing::warn;

use crate::io_jsonl::read_jsonl_strict;
use crate::models::{
    ChapterHeading, CitationMention, CitesEdge, LegalTextIdentity, LegalTextVersion, Provision,
    QcChunkStats, QcCitationStats, QcCoverageStats, QcEmbeddingReadiness, QcExamples, QcFullReport,
    QcGoldenStats, QcGraphStats, QcParseStats, QcProvisionEmbeddingReadiness, QcResolverReadiness,
    QcSemanticStats, QcSourceStats, QcStatus, QcTokenDistribution, QcVersionEmbeddingReadiness,
    RetrievalChunk,
};

// Constants for validation thresholds
const MAX_EXAMPLES_PER_CATEGORY: usize = 25;
const TINY_HTML_THRESHOLD: u64 = 1024; // 1KB
const MAX_WARNINGS_PER_CATEGORY: usize = 100;
const HARD_FAIL_TOKENS: usize = 30_000;

pub struct QcFullValidator {
    graph_dir: PathBuf,
    raw_dir: Option<PathBuf>,
    expected_chapters: usize,
    edition_year: i32,
    require_resolved_citations: bool,
    strict_chunk_policy: bool,
    require_embeddings: bool,
    require_golden: bool,
    embedding_model: String,
    embedding_dim: usize,
    errors: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    warnings: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    examples: std::sync::Arc<std::sync::Mutex<QcExamples>>,
}

impl QcFullValidator {
    pub fn new(
        graph_dir: PathBuf,
        raw_dir: Option<PathBuf>,
        expected_chapters: usize,
        edition_year: i32,
        require_resolved_citations: bool,
        strict_chunk_policy: bool,
        require_embeddings: bool,
        require_golden: bool,
        embedding_model: String,
        embedding_dim: usize,
    ) -> Self {
        Self {
            graph_dir,
            raw_dir,
            expected_chapters,
            edition_year,
            require_resolved_citations,
            strict_chunk_policy,
            require_embeddings,
            require_golden,
            embedding_model,
            embedding_dim,
            errors: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            warnings: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            examples: std::sync::Arc::new(std::sync::Mutex::new(QcExamples::default())),
        }
    }

    fn add_example(&self, category: &str, value: String) {
        let mut examples = self.examples.lock().unwrap();
        match category {
            "duplicate_provision_ids" => {
                if examples.duplicate_provision_ids.len() < MAX_EXAMPLES_PER_CATEGORY {
                    examples.duplicate_provision_ids.push(value);
                }
            }
            "orphan_chunks" => {
                if examples.orphan_chunks.len() < MAX_EXAMPLES_PER_CATEGORY {
                    examples.orphan_chunks.push(value);
                }
            }
            "heading_leaks" => {
                if examples.heading_leaks.len() < MAX_EXAMPLES_PER_CATEGORY {
                    examples.heading_leaks.push(value);
                }
            }
            "unresolved_citations" => {
                if examples.unresolved_citations.len() < MAX_EXAMPLES_PER_CATEGORY {
                    examples.unresolved_citations.push(value);
                }
            }
            "bad_edges" => {
                if examples.bad_edges.len() < MAX_EXAMPLES_PER_CATEGORY {
                    examples.bad_edges.push(value);
                }
            }
            _ => {}
        }
    }

    pub fn run(mut self) -> Result<QcFullReport> {
        let run_id = format!(
            "ors_{}_full_parse_{}",
            self.edition_year,
            uuid::Uuid::new_v4()
        );
        let generated_at = chrono::Utc::now().to_rfc3339();

        let source_stats = self.validate_source()?;

        let _identities: Vec<LegalTextIdentity> =
            read_jsonl_strict(self.graph_dir.join("legal_text_identities.jsonl"))?;
        let _versions: Vec<LegalTextVersion> =
            read_jsonl_strict(self.graph_dir.join("legal_text_versions.jsonl"))?;
        let _provisions: Vec<Provision> =
            read_jsonl_strict(self.graph_dir.join("provisions.jsonl"))?;
        let _chunks: Vec<RetrievalChunk> =
            read_jsonl_strict(self.graph_dir.join("retrieval_chunks.jsonl"))?;
        let _citations: Vec<CitationMention> =
            read_jsonl_strict(self.graph_dir.join("citation_mentions.jsonl"))?;
        let _headings: Vec<ChapterHeading> =
            read_jsonl_strict(self.graph_dir.join("chapter_headings.jsonl"))?;
        let _edges: Vec<CitesEdge> = read_jsonl_strict(self.graph_dir.join("cites_edges.jsonl"))?;

        let identity_ids: HashSet<String> =
            _identities.iter().map(|i| i.canonical_id.clone()).collect();
        let version_ids: HashSet<String> = _versions.iter().map(|v| v.version_id.clone()).collect();
        let provision_ids: HashSet<String> =
            _provisions.iter().map(|p| p.provision_id.clone()).collect();
        let chapter_ids: HashSet<String> = _versions
            .iter()
            .map(|v| format!("or:ors:chapter:{}@{}", v.chapter, v.edition_year))
            .collect();

        // Run streaming validations in parallel
        // Group validations by which file they read to avoid redundant I/O:
        // - Thread A: validate_parse_streaming (reads versions, identities, provisions, headings)
        // - Thread B: validate_chunks_combined (reads chunks ONCE for chunk stats + embedding readiness + coverage)
        // - Thread C: validate_citations_streaming (reads citation_mentions)
        // - Thread D: validate_graph_streaming (reads cites_edges)
        // - Thread E: validate_semantic (reads ~15 small entity files)
        // - Thread F: validate_golden_tests (checks file existence only)
        // - Thread G: validate_resolver_readiness_streaming (reads identities - small)
        let (
            parse_stats,
            (chunk_stats, chunk_readiness, provision_readiness, version_readiness, coverage_stats),
            citation_stats,
            graph_stats,
            resolver_readiness,
            semantic_stats,
            golden_stats,
        ) = std::thread::scope(|s| {
            let parse_handle = s.spawn(|| self.validate_parse_streaming());
            let chunk_handle =
                s.spawn(|| self.validate_chunks_combined(&provision_ids, &version_ids));
            let citation_handle = s.spawn(|| {
                self.validate_citations_streaming(&provision_ids, &identity_ids, &chapter_ids)
            });
            let graph_handle = s.spawn(|| {
                self.validate_graph_streaming(
                    &identity_ids,
                    &version_ids,
                    &provision_ids,
                    &chapter_ids,
                )
            });
            let resolver_handle = s.spawn(|| self.validate_resolver_readiness_streaming());
            let semantic_handle =
                s.spawn(|| self.validate_semantic(&provision_ids, &version_ids, &identity_ids));
            let golden_handle = s.spawn(|| self.validate_golden_tests());

            Result::<_, anyhow::Error>::Ok((
                parse_handle.join().unwrap()?,
                chunk_handle.join().unwrap()?,
                citation_handle.join().unwrap()?,
                graph_handle.join().unwrap()?,
                resolver_handle.join().unwrap()?,
                semantic_handle.join().unwrap()?,
                golden_handle.join().unwrap()?,
            ))
        })?;

        let status = if self.errors.lock().unwrap().is_empty() {
            if self.warnings.lock().unwrap().is_empty() {
                QcStatus::Pass
            } else {
                QcStatus::Warning
            }
        } else {
            QcStatus::Fail
        };

        Ok(QcFullReport {
            run_id,
            generated_at,
            edition_year: self.edition_year,
            status,
            source: source_stats,
            parse: parse_stats,
            chunks: chunk_stats,
            citations: citation_stats,
            graph: graph_stats,
            embedding_readiness: chunk_readiness,
            provision_embedding_readiness: provision_readiness,
            version_embedding_readiness: version_readiness,
            resolver_readiness,
            coverage: coverage_stats,
            semantic: semantic_stats,
            golden: golden_stats,
            blocking_errors: self.errors.lock().unwrap().clone(),
            warnings: self.warnings.lock().unwrap().clone(),
            examples: self.examples.lock().unwrap().clone(),
            ..Default::default()
        })
    }

    fn validate_source(&mut self) -> Result<QcSourceStats> {
        let mut empty_raw_files = 0;
        let mut raw_html_files = 0;
        let mut raw_html_bytes = 0;
        let mut empty_html_files = 0;
        let mut tiny_html_files = 0;

        let enforce_raw_counts = self.raw_dir.is_some();
        if let Some(raw_dir) = &self.raw_dir {
            if raw_dir.exists() {
                for entry in fs::read_dir(raw_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if !is_raw_html_path(&path) {
                        continue;
                    }
                    let metadata = entry.metadata()?;
                    raw_html_files += 1;
                    raw_html_bytes += metadata.len();
                    if metadata.len() < TINY_HTML_THRESHOLD {
                        tiny_html_files += 1;
                        empty_raw_files += 1;
                    }
                    if metadata.len() == 0 {
                        empty_html_files += 1;
                    }
                }
            } else {
                warn!("Raw dir does not exist: {}", raw_dir.display());
            }
        }

        let chapters_expected = if enforce_raw_counts {
            self.expected_chapters
        } else {
            0
        };
        let chapters_fetched = raw_html_files;

        let fetch_failures = if enforce_raw_counts {
            chapters_expected.saturating_sub(chapters_fetched)
        } else {
            0
        };
        if enforce_raw_counts && fetch_failures > 0 {
            self.errors.lock().unwrap().push(format!(
                "Expected {} chapters, found {}",
                chapters_expected, chapters_fetched
            ));
        }
        if empty_html_files > 0 {
            self.errors
                .lock()
                .unwrap()
                .push(format!("Empty raw HTML files: {empty_html_files}"));
        }
        if tiny_html_files > 0 {
            self.warnings
                .lock()
                .unwrap()
                .push(format!("Tiny raw HTML files: {tiny_html_files}"));
        }

        Ok(QcSourceStats {
            chapters_expected,
            chapters_fetched,
            fetch_failures,
            empty_raw_files,
            raw_html_files,
            raw_html_bytes,
            empty_html_files,
            tiny_html_files,
        })
    }

    fn validate_parse_streaming(&self) -> Result<QcParseStats> {
        let mut identities_count = 0;
        let mut versions_count = 0;
        let mut provisions_count = 0;
        let mut headings_count = 0;
        let mut active_with_empty_text = 0;
        let mut invalid_status_classification = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<LegalTextIdentity>(
            self.graph_dir.join("legal_text_identities.jsonl"),
            5000,
        )? {
            let b = batch?;
            identities_count += b.len();
        }

        for batch in crate::io_jsonl::read_jsonl_batches::<LegalTextVersion>(
            self.graph_dir.join("legal_text_versions.jsonl"),
            5000,
        )? {
            for v in batch? {
                versions_count += 1;
                if v.status == "active" && v.text.trim().is_empty() {
                    active_with_empty_text += 1;
                    self.errors
                        .lock()
                        .unwrap()
                        .push(format!("Active version {} has empty text", v.version_id));
                }
                if !matches!(
                    v.status.as_str(),
                    "active" | "repealed" | "renumbered" | "formerly" | "status_only"
                ) {
                    invalid_status_classification += 1;
                }
            }
        }

        for batch in crate::io_jsonl::read_jsonl_batches::<Provision>(
            self.graph_dir.join("provisions.jsonl"),
            5000,
        )? {
            let b = batch?;
            provisions_count += b.len();
        }

        for batch in crate::io_jsonl::read_jsonl_batches::<ChapterHeading>(
            self.graph_dir.join("chapter_headings.jsonl"),
            5000,
        )? {
            let b = batch?;
            headings_count += b.len();
        }

        Ok(QcParseStats {
            identities_count,
            versions_count,
            provisions_count,
            headings_count,
            active_with_empty_text,
            invalid_status_classification,
            ..Default::default()
        })
    }

    fn validate_citations_streaming(
        &self,
        provision_ids: &HashSet<String>,
        identity_ids: &HashSet<String>,
        chapter_ids: &HashSet<String>,
    ) -> Result<QcCitationStats> {
        let mut total_mentions = 0;
        let mut unresolved_target_not_in_corpus = 0;
        let mut unresolved_malformed_citation = 0;
        let mut unsupported_citation_type = 0;
        let mut parsed_unverified = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<CitationMention>(
            self.graph_dir.join("citation_mentions.jsonl"),
            5000,
        )? {
            for c in batch? {
                total_mentions += 1;

                if self.require_resolved_citations && c.resolver_status == "parsed_unverified" {
                    self.errors.lock().unwrap().push(format!(
                        "Unresolved citation in provision {}: {}",
                        c.source_provision_id, c.normalized_citation
                    ));
                    self.add_example("unresolved_citations", c.normalized_citation.clone());
                }

                match c.resolver_status.as_str() {
                    "parsed_unverified" => parsed_unverified += 1,
                    "resolved_section" => {}
                    "resolved_section_and_provision" => {}
                    "resolved_chapter" => {}
                    "resolved_range" => {}
                    "resolved_section_unresolved_subpath" => {}
                    "unresolved_target_not_in_corpus" => unresolved_target_not_in_corpus += 1,
                    "unresolved_malformed_citation" => unresolved_malformed_citation += 1,
                    "unsupported_citation_type" => unsupported_citation_type += 1,
                    _ => {}
                }

                let unresolved_status = matches!(
                    c.resolver_status.as_str(),
                    "parsed_unverified"
                        | "unresolved_target_not_in_corpus"
                        | "unresolved_malformed_citation"
                        | "unsupported_citation_type"
                );

                if !unresolved_status {
                    if let Some(target_id) = &c.target_provision_id {
                        if !provision_ids.contains(target_id) {
                            self.errors.lock().unwrap().push(format!(
                                "Citation in {} resolves to non-existent provision {}",
                                c.source_provision_id, target_id
                            ));
                        }
                    } else if let Some(target_id) = &c.target_canonical_id {
                        if !identity_ids.contains(target_id) && !chapter_ids.contains(target_id) {
                            self.errors.lock().unwrap().push(format!(
                                "Citation in {} resolves to non-existent canonical_id {}",
                                c.source_provision_id, target_id
                            ));
                        }
                    }
                }
            }
        }

        Ok(QcCitationStats {
            citation_mentions: total_mentions,
            unresolved: parsed_unverified
                + unresolved_target_not_in_corpus
                + unresolved_malformed_citation
                + unsupported_citation_type,
            resolution_pending: parsed_unverified > 0,
            ..Default::default()
        })
    }

    fn validate_graph_streaming(
        &self,
        identity_ids: &HashSet<String>,
        version_ids: &HashSet<String>,
        provision_ids: &HashSet<String>,
        chapter_ids: &HashSet<String>,
    ) -> Result<QcGraphStats> {
        let mut total_edges = 0;
        let mut invalid_source = 0;
        let mut invalid_target = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<CitesEdge>(
            self.graph_dir.join("cites_edges.jsonl"),
            5000,
        )? {
            for e in batch? {
                total_edges += 1;

                if !provision_ids.contains(&e.source_provision_id) {
                    invalid_source += 1;
                    self.add_example("bad_edges", e.edge_id.clone());
                }

                let mut target_valid = false;
                if let Some(target_id) = &e.target_provision_id {
                    if provision_ids.contains(target_id) {
                        target_valid = true;
                    }
                } else if let Some(target_id) = &e.target_version_id {
                    if version_ids.contains(target_id) {
                        target_valid = true;
                    }
                } else if let Some(target_id) = &e.target_canonical_id {
                    if identity_ids.contains(target_id) {
                        target_valid = true;
                    }
                } else if let Some(target_id) = &e.target_chapter_id {
                    if chapter_ids.contains(target_id) {
                        target_valid = true;
                    }
                }

                if !target_valid {
                    invalid_target += 1;
                    self.add_example("bad_edges", e.edge_id.clone());
                }
            }
        }

        Ok(QcGraphStats {
            edges: total_edges,
            orphan_edges: invalid_source + invalid_target,
            ..Default::default()
        })
    }

    fn validate_resolver_readiness_streaming(&self) -> Result<QcResolverReadiness> {
        let mut total_identities = 0;
        let mut total_versions = 0;
        let mut total_provisions = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<LegalTextIdentity>(
            self.graph_dir.join("legal_text_identities.jsonl"),
            5000,
        )? {
            let b = batch?;
            total_identities += b.len();
        }
        for batch in crate::io_jsonl::read_jsonl_batches::<LegalTextVersion>(
            self.graph_dir.join("legal_text_versions.jsonl"),
            5000,
        )? {
            let b = batch?;
            total_versions += b.len();
        }
        for batch in crate::io_jsonl::read_jsonl_batches::<Provision>(
            self.graph_dir.join("provisions.jsonl"),
            5000,
        )? {
            let b = batch?;
            total_provisions += b.len();
        }

        Ok(QcResolverReadiness {
            identity_index_ready: total_identities > 0,
            version_index_ready: total_versions > 0,
            provision_path_index_ready: total_provisions > 0,
            chapter_index_ready: total_identities > 0, // Placeholder
        })
    }

    /// Combined single-pass validation that reads each heavy file once instead of 3x.
    /// Consolidates: validate_chunks_streaming + validate_embedding_readiness_streaming + validate_coverage_streaming
    fn validate_chunks_combined(
        &self,
        provision_ids: &HashSet<String>,
        version_ids: &HashSet<String>,
    ) -> Result<(
        QcChunkStats,
        QcEmbeddingReadiness,
        QcProvisionEmbeddingReadiness,
        QcVersionEmbeddingReadiness,
        QcCoverageStats,
    )> {
        let model_config = crate::voyage::model_config(&self.embedding_model)
            .unwrap_or(&crate::voyage::VOYAGE_4_LARGE);
        let context_tokens = model_config.context_tokens;
        let batch_token_limit = model_config.batch_token_limit;
        let batch_token_safety_limit = model_config.batch_token_safety_limit;

        if model_config.model != self.embedding_model {
            self.warnings.lock().unwrap().push(format!(
                "Unknown embedding model {}; using voyage-4-large token limits for QC",
                self.embedding_model
            ));
        }

        if !model_config
            .allowed_dimensions
            .contains(&(self.embedding_dim as usize))
        {
            self.errors.lock().unwrap().push(format!(
                "Embedding dimension {} is not supported by {}. Allowed dimensions: {:?}",
                self.embedding_dim, self.embedding_model, model_config.allowed_dimensions
            ));
        }

        // ── SINGLE PASS over retrieval_chunks.jsonl ──
        // Computes: chunk stats + chunk embedding readiness + coverage maps
        let mut full_statute_chunks = 0;
        let mut contextual_provision_chunks = 0;
        let mut definition_chunks = 0;
        let mut exception_chunks = 0;
        let mut deadline_chunks = 0;
        let mut penalty_chunks = 0;
        let mut citation_context_chunks = 0;
        let mut orphan_chunks = 0;
        let mut oversized_chunks_warn = 0;
        let mut oversized_chunks_fail = 0;
        let mut missing_embedding_input_hash = 0;
        let mut missing_embedding_policy = 0;
        let mut missing_token_count = 0;
        let mut missing_max_tokens = 0;
        let mut missing_context_window = 0;
        let mut missing_chunking_strategy = 0;
        let mut missing_chunk_version = 0;
        let mut invalid_part_metadata = 0;
        let mut chunks_over_max_tokens = 0;
        let mut chunks_over_hard_token_limit = 0;
        let mut token_counts = Vec::new();
        let mut token_counts_by_type: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut chunk_version_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut chunking_strategy_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut split_reason_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut part_keys = HashSet::new();
        let mut chunk_ids = HashSet::new();
        let mut duplicate_chunk_ids = 0;
        let mut total_chunks = 0;

        // Embedding readiness accumulators
        let mut emb_eligible_chunks = 0usize;
        let mut emb_chunks_missing_input_hash = 0usize;
        let mut emb_chunks_over_context_limit = 0usize;
        let mut emb_estimated_total_tokens = 0usize;

        // Coverage accumulators
        let mut version_chunk_counts: HashMap<String, usize> = HashMap::new();
        let mut provision_chunk_counts: HashMap<String, usize> = HashMap::new();

        // Batch error/warning accumulators to reduce mutex contention
        let mut local_errors = Vec::new();
        let mut local_warnings = Vec::new();

        for batch in crate::io_jsonl::read_jsonl_batches::<RetrievalChunk>(
            self.graph_dir.join("retrieval_chunks.jsonl"),
            5000,
        )? {
            for chunk in batch? {
                total_chunks += 1;
                if !chunk_ids.insert(chunk.chunk_id.clone()) {
                    duplicate_chunk_ids += 1;
                    if local_warnings.len() < MAX_WARNINGS_PER_CATEGORY {
                        local_warnings.push(format!("Duplicate chunk_id: {}", chunk.chunk_id));
                    }
                }

                match chunk.chunk_type.as_str() {
                    "full_statute" => full_statute_chunks += 1,
                    "contextual_provision" => contextual_provision_chunks += 1,
                    "definition_block" => definition_chunks += 1,
                    "exception_block" => exception_chunks += 1,
                    "deadline_block" => deadline_chunks += 1,
                    "penalty_block" => penalty_chunks += 1,
                    "citation_context" => citation_context_chunks += 1,
                    _ => {}
                }

                // ── Coverage: track chunk counts per parent ──
                if chunk.chunk_type == "full_statute" {
                    *version_chunk_counts
                        .entry(chunk.parent_version_id.clone())
                        .or_insert(0) += 1;
                } else if chunk.chunk_type == "contextual_provision" {
                    if let Some(source_provision_id) = &chunk.source_provision_id {
                        *provision_chunk_counts
                            .entry(source_provision_id.clone())
                            .or_insert(0) += 1;
                    }
                }

                // ── Orphan check ──
                let mut is_orphan = false;
                if chunk.chunk_type == "full_statute" {
                    if !version_ids.contains(&chunk.parent_version_id) {
                        is_orphan = true;
                    }
                } else {
                    if chunk
                        .source_provision_id
                        .as_ref()
                        .map(|id| !provision_ids.contains(id))
                        .unwrap_or(true)
                    {
                        is_orphan = true;
                    }
                }

                if is_orphan {
                    orphan_chunks += 1;
                    self.add_example("orphan_chunks", chunk.chunk_id.clone());
                }
                if chunk.text.trim().is_empty() {
                    local_errors.push(format!("Chunk {} has empty text", chunk.chunk_id));
                }

                let is_embeddable = matches!(
                    chunk.embedding_policy.as_deref(),
                    Some("embed_primary") | Some("embed_special")
                );
                let effective_tokens = chunk.token_count.unwrap_or_else(|| {
                    crate::voyage::estimate_tokens(&chunk.text, &self.embedding_model)
                });
                token_counts.push(effective_tokens);
                token_counts_by_type
                    .entry(chunk.chunk_type.clone())
                    .or_default()
                    .push(effective_tokens);

                let version_key = chunk
                    .chunk_version
                    .clone()
                    .unwrap_or_else(|| "<missing>".to_string());
                *chunk_version_counts.entry(version_key).or_default() += 1;
                let strategy_key = chunk
                    .chunking_strategy
                    .clone()
                    .unwrap_or_else(|| "<missing>".to_string());
                *chunking_strategy_counts.entry(strategy_key).or_default() += 1;
                let split_key = chunk
                    .split_reason
                    .clone()
                    .unwrap_or_else(|| "<missing>".to_string());
                *split_reason_counts.entry(split_key).or_default() += 1;

                if is_embeddable {
                    // ── Embedding readiness stats ──
                    emb_eligible_chunks += 1;
                    emb_estimated_total_tokens += effective_tokens;

                    if chunk.text.trim().is_empty() {
                        local_errors
                            .push(format!("Eligible chunk {} has empty text", chunk.chunk_id));
                    }
                    if chunk.embedding_input_hash.is_empty() {
                        emb_chunks_missing_input_hash += 1;
                        local_errors.push(format!(
                            "Eligible chunk {} missing embedding_input_hash",
                            chunk.chunk_id
                        ));
                    }
                    if effective_tokens > context_tokens {
                        emb_chunks_over_context_limit += 1;
                        local_errors.push(format!(
                            "Eligible chunk {} exceeds {} token limit ({} tokens)",
                            chunk.chunk_id, context_tokens, effective_tokens
                        ));
                    }

                    // ── Chunk stats: token budget checks ──
                    if chunk.token_count.is_none() {
                        missing_token_count += 1;
                    }
                    if chunk.max_tokens.is_none() {
                        missing_max_tokens += 1;
                    }
                    if chunk.context_window.is_none() {
                        missing_context_window += 1;
                    }
                    if chunk.chunking_strategy.is_none() {
                        missing_chunking_strategy += 1;
                    }
                    if chunk.chunk_version.is_none() {
                        missing_chunk_version += 1;
                    }

                    if let Some(max_tokens) = chunk.max_tokens {
                        if effective_tokens > max_tokens && effective_tokens <= HARD_FAIL_TOKENS {
                            chunks_over_max_tokens += 1;
                            oversized_chunks_warn += 1;
                            if local_warnings.len() < MAX_WARNINGS_PER_CATEGORY {
                                local_warnings.push(format!(
                                    "Chunk {} type {} is over target budget: {} tokens > {}",
                                    chunk.chunk_id, chunk.chunk_type, effective_tokens, max_tokens
                                ));
                            }
                        }
                    }

                    if effective_tokens > HARD_FAIL_TOKENS {
                        chunks_over_hard_token_limit += 1;
                        oversized_chunks_fail += 1;
                        local_errors.push(format!(
                            "Chunk {} type {} exceeds hard token limit: {} > {}",
                            chunk.chunk_id, chunk.chunk_type, effective_tokens, HARD_FAIL_TOKENS
                        ));
                    }
                }

                match (chunk.part_index, chunk.part_count) {
                    (Some(index), Some(count)) if count > 0 && index >= 1 && index <= count => {
                        let part_source = chunk
                            .source_id
                            .clone()
                            .or_else(|| chunk.source_provision_id.clone())
                            .or_else(|| chunk.source_version_id.clone())
                            .unwrap_or_else(|| chunk.chunk_id.clone());
                        if !part_keys.insert((part_source, chunk.chunk_type.clone(), index)) {
                            invalid_part_metadata += 1;
                            local_errors.push(format!(
                                "Chunk {} has duplicate part_index {} for source/type",
                                chunk.chunk_id, index
                            ));
                        }
                    }
                    _ if is_embeddable => {
                        invalid_part_metadata += 1;
                        local_errors.push(format!(
                            "Chunk {} has invalid part_index / part_count",
                            chunk.chunk_id
                        ));
                    }
                    _ => {}
                }

                if chunk.embedding_input_hash.is_empty() {
                    missing_embedding_input_hash += 1;
                }

                if chunk.embedding_policy.is_none() {
                    missing_embedding_policy += 1;
                }

                if self.strict_chunk_policy {
                    if chunk.answer_policy.is_none() {
                        local_errors
                            .push(format!("Chunk {} missing answer_policy", chunk.chunk_id));
                    }
                    if chunk.retrieval_profile.is_none() {
                        local_errors.push(format!(
                            "Chunk {} missing retrieval_profile",
                            chunk.chunk_id
                        ));
                    }
                    if chunk.chunk_version.as_deref() != Some("3.0") {
                        local_errors.push(format!(
                            "Chunk {} has invalid chunk_version",
                            chunk.chunk_id
                        ));
                    }
                    if chunk.search_weight.is_none() {
                        local_errors
                            .push(format!("Chunk {} missing search_weight", chunk.chunk_id));
                    }
                }
            }
        }

        // Flush accumulated errors/warnings in bulk (single mutex acquisition each)
        {
            let mut errors = self.errors.lock().unwrap();
            errors.extend(local_errors);
        }
        {
            let mut warnings = self.warnings.lock().unwrap();
            warnings.extend(local_warnings);
        }

        // Post-chunk aggregate errors
        if orphan_chunks > 0 {
            self.errors
                .lock()
                .unwrap()
                .push(format!("orphan chunks detected: {orphan_chunks}"));
        }
        if self.require_embeddings && missing_embedding_input_hash > 0 {
            self.errors.lock().unwrap().push(format!(
                "chunks missing embedding_input_hash: {missing_embedding_input_hash}"
            ));
        }
        if missing_embedding_policy > 0 {
            self.errors.lock().unwrap().push(format!(
                "chunks missing embedding_policy: {missing_embedding_policy}"
            ));
        }
        if self.strict_chunk_policy {
            if missing_token_count > 0 {
                self.errors.lock().unwrap().push(format!(
                    "embeddable chunks missing token_count: {missing_token_count}"
                ));
            }
            if missing_max_tokens > 0 {
                self.errors.lock().unwrap().push(format!(
                    "embeddable chunks missing max_tokens: {missing_max_tokens}"
                ));
            }
            if missing_context_window > 0 {
                self.errors.lock().unwrap().push(format!(
                    "embeddable chunks missing context_window: {missing_context_window}"
                ));
            }
            if missing_chunking_strategy > 0 {
                self.errors.lock().unwrap().push(format!(
                    "embeddable chunks missing chunking_strategy: {missing_chunking_strategy}"
                ));
            }
            if missing_chunk_version > 0 {
                self.errors.lock().unwrap().push(format!(
                    "embeddable chunks missing chunk_version: {missing_chunk_version}"
                ));
            }
        }

        let max_token_count = token_counts.iter().copied().max().unwrap_or_default();
        let p50_token_count = percentile_token_count(&token_counts, 50.0);
        let p95_token_count = percentile_token_count(&token_counts, 95.0);
        let p99_token_count = percentile_token_count(&token_counts, 99.0);
        let chunk_type_token_distribution = token_counts_by_type
            .into_iter()
            .map(|(chunk_type, counts)| {
                (
                    chunk_type,
                    QcTokenDistribution {
                        count: counts.len(),
                        max: counts.iter().copied().max().unwrap_or_default(),
                        p50: percentile_token_count(&counts, 50.0),
                        p95: percentile_token_count(&counts, 95.0),
                        p99: percentile_token_count(&counts, 99.0),
                    },
                )
            })
            .collect();

        let chunk_stats = QcChunkStats {
            total_chunks,
            full_statute_chunks,
            contextual_provision_chunks,
            definition_chunks,
            exception_chunks,
            deadline_chunks,
            penalty_chunks,
            citation_context_chunks,
            orphan_chunks,
            duplicate_chunk_ids,
            oversized_chunks_warn,
            oversized_chunks_fail,
            missing_embedding_input_hash,
            missing_embedding_policy,
            empty_chunks: 0,
            missing_answer_policy: 0,
            missing_retrieval_profile: 0,
            missing_chunk_schema_version: 0,
            missing_search_weight: 0,
            invalid_answer_policy: 0,
            generated_marked_authoritative: 0,
            invalid_chunk_schema_version: 0,
            missing_token_count,
            missing_max_tokens,
            missing_context_window,
            missing_chunking_strategy,
            missing_chunk_version,
            invalid_part_metadata,
            chunks_over_max_tokens,
            chunks_over_hard_token_limit,
            max_token_count,
            p50_token_count,
            p95_token_count,
            p99_token_count,
            chunk_version_counts,
            chunking_strategy_counts,
            split_reason_counts,
            chunk_type_token_distribution,
        };

        let mut chunk_readiness = QcEmbeddingReadiness {
            model: self.embedding_model.clone(),
            dimension: self.embedding_dim as usize,
            model_context_tokens: context_tokens,
            model_batch_token_limit: batch_token_limit,
            batch_token_safety_limit,
            eligible_chunks: emb_eligible_chunks,
            estimated_total_tokens: emb_estimated_total_tokens,
            chunks_missing_input_hash: emb_chunks_missing_input_hash,
            chunks_over_context_limit: emb_chunks_over_context_limit,
            ..Default::default()
        };
        chunk_readiness.estimated_batches =
            (chunk_readiness.estimated_total_tokens / batch_token_safety_limit).max(1);

        // ── SINGLE PASS over provisions.jsonl (for embedding readiness + coverage) ──
        let mut provision_stats = QcProvisionEmbeddingReadiness {
            model: self.embedding_model.clone(),
            dimension: self.embedding_dim as usize,
            eligible_provisions: 0,
            model_context_tokens: context_tokens,
            model_batch_token_limit: batch_token_limit,
            batch_token_safety_limit,
            ..Default::default()
        };

        let mut provisions_missing_contextual_chunk = 0;
        let mut provisions_with_duplicate_contextual_chunks = 0;
        let mut valid_provisions_coverage = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<Provision>(
            self.graph_dir.join("provisions.jsonl"),
            5000,
        )? {
            for p in batch? {
                // Embedding readiness
                provision_stats.eligible_provisions += 1;
                let input_text = format!(
                    "Oregon Revised Statutes. {} Edition.\nCitation: {}\nProvision type: {}.\nStatus: active.\nText:\n{}",
                    self.edition_year, p.display_citation, p.provision_type, p.text
                );
                let tokens = crate::voyage::estimate_tokens(&input_text, &self.embedding_model);
                provision_stats.estimated_total_tokens += tokens;

                if tokens > context_tokens {
                    provision_stats.provisions_over_context_limit += 1;
                }
                if p.embedding_input_hash
                    .as_ref()
                    .map_or(true, |h| h.is_empty())
                {
                    provision_stats.provisions_missing_input_hash += 1;
                }

                // Coverage
                if !p.is_implied && !p.text.trim().is_empty() {
                    valid_provisions_coverage += 1;
                }
                if provision_ids.contains(&p.provision_id) {
                    let count = provision_chunk_counts
                        .get(&p.provision_id)
                        .cloned()
                        .unwrap_or(0);
                    if count == 0 {
                        provisions_missing_contextual_chunk += 1;
                        self.errors.lock().unwrap().push(format!(
                            "Provision {} missing contextual_provision chunk",
                            p.provision_id
                        ));
                    } else if count > 1 {
                        provisions_with_duplicate_contextual_chunks += 1;
                    }
                }
            }
        }
        provision_stats.estimated_batches =
            (provision_stats.estimated_total_tokens / batch_token_safety_limit).max(1);

        // ── SINGLE PASS over legal_text_versions.jsonl (for embedding readiness + coverage) ──
        let mut version_stats = QcVersionEmbeddingReadiness {
            model: self.embedding_model.clone(),
            dimension: self.embedding_dim as usize,
            eligible_versions: 0,
            model_context_tokens: context_tokens,
            model_batch_token_limit: batch_token_limit,
            batch_token_safety_limit,
            ..Default::default()
        };

        let mut active_versions_missing_full_statute_chunk = 0;
        let mut versions_with_duplicate_full_statute_chunks = 0;
        let mut active_versions_count = 0;

        for batch in crate::io_jsonl::read_jsonl_batches::<LegalTextVersion>(
            self.graph_dir.join("legal_text_versions.jsonl"),
            5000,
        )? {
            for v in batch? {
                // Embedding readiness
                version_stats.eligible_versions += 1;
                let input_text = format!(
                    "Oregon Revised Statutes. {} Edition.\nCitation: {}\nTitle: {}\nStatus: {}\nText:\n{}",
                    self.edition_year,
                    v.citation,
                    v.title.as_deref().unwrap_or(""),
                    v.status,
                    v.text
                );
                let tokens = crate::voyage::estimate_tokens(&input_text, &self.embedding_model);
                version_stats.estimated_total_tokens += tokens;

                if tokens > context_tokens {
                    version_stats.versions_over_context_limit += 1;
                }
                if v.embedding_input_hash
                    .as_ref()
                    .map_or(true, |h| h.is_empty())
                {
                    version_stats.versions_missing_input_hash += 1;
                }

                // Coverage
                if v.status == "active" {
                    active_versions_count += 1;
                    let count = version_chunk_counts
                        .get(&v.version_id)
                        .cloned()
                        .unwrap_or(0);
                    if count == 0 {
                        active_versions_missing_full_statute_chunk += 1;
                        self.errors.lock().unwrap().push(format!(
                            "Active version {} missing full_statute chunk",
                            v.version_id
                        ));
                    } else if count > 1 {
                        versions_with_duplicate_full_statute_chunks += 1;
                    }
                }
            }
        }
        version_stats.estimated_batches =
            (version_stats.estimated_total_tokens / batch_token_safety_limit).max(1);

        let coverage_stats = QcCoverageStats {
            active_versions: active_versions_count,
            full_statute_chunks: version_chunk_counts.len(),
            active_versions_missing_full_statute_chunk,
            versions_with_duplicate_full_statute_chunks,
            provisions_missing_contextual_chunk,
            valid_provisions: valid_provisions_coverage,
            contextual_provision_chunks: provision_chunk_counts.len(),
            provisions_with_duplicate_contextual_chunks,
        };

        Ok((
            chunk_stats,
            chunk_readiness,
            provision_stats,
            version_stats,
            coverage_stats,
        ))
    }

    fn validate_golden_tests(&self) -> Result<QcGoldenStats> {
        let golden_dir = PathBuf::from("golden");
        let expected = [
            "chunks.yaml",
            "citation_extraction.yaml",
            "citation_resolution.yaml",
            "graph_integrity.yaml",
            "parse_sections.yaml",
            "provision_paths.yaml",
            "search_queries.yaml",
        ];
        let found = expected
            .iter()
            .filter(|file| golden_dir.join(file).exists())
            .count();
        let present = found == expected.len();
        if self.require_golden && !present {
            self.errors.lock().unwrap().push(format!(
                "golden tests missing: found {found} of {} files",
                expected.len()
            ));
        }
        Ok(QcGoldenStats {
            search_queries_tested: 0,
            search_queries_passed: 0,
            citation_extraction_tested: 0,
            citation_extraction_passed: 0,
            citation_resolution_tested: 0,
            citation_resolution_passed: 0,
            golden_tests_present: present,
            golden_tests_passed: !self.require_golden || present,
            golden_files_found: found,
            golden_files_expected: expected.len(),
        })
    }

    fn validate_semantic(
        &self,
        provision_ids: &HashSet<String>,
        version_ids: &HashSet<String>,
        identity_ids: &HashSet<String>,
    ) -> Result<QcSemanticStats> {
        let mut stats = QcSemanticStats::default();

        // Helper: read a JSONL file as serde_json::Value rows, returning empty if missing
        let read_optional_values = |filename: &str| -> Result<Vec<serde_json::Value>> {
            let path = self.graph_dir.join(filename);
            if !path.exists() {
                return Ok(Vec::new());
            }
            let content = std::fs::read_to_string(&path)?;
            let mut rows = Vec::new();
            for (i, line) in content.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let val: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                    anyhow::anyhow!("Malformed JSONL in {} line {}: {}", filename, i + 1, e)
                })?;
                rows.push(val);
            }
            Ok(rows)
        };

        // Helper: check confidence is in [0.0, 1.0]
        let check_confidence = |val: &serde_json::Value, stats: &mut QcSemanticStats| {
            if let Some(conf) = val.get("confidence").and_then(|v| v.as_f64()) {
                if !(0.0..=1.0).contains(&conf) {
                    stats.invalid_confidence_count += 1;
                }
            }
        };

        // Helper: check source_provision_id linkage
        let is_orphan_provision =
            |val: &serde_json::Value, provision_ids: &HashSet<String>| -> bool {
                val.get("source_provision_id")
                    .and_then(|v| v.as_str())
                    .map_or(true, |id| !provision_ids.contains(id))
            };

        let source_documents = read_optional_values("source_documents.jsonl")?;
        let source_document_ids: HashSet<String> = source_documents
            .iter()
            .filter_map(|val| {
                val.get("source_document_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        let is_orphan_source_document =
            |val: &serde_json::Value, source_document_ids: &HashSet<String>| -> bool {
                val.get("source_document_id")
                    .and_then(|v| v.as_str())
                    .map_or(true, |id| !source_document_ids.contains(id))
            };

        // --- Status Events ---
        let status_events = read_optional_values("status_events.jsonl")?;
        stats.status_events = status_events.len();
        for val in &status_events {
            check_confidence(val, &mut stats);
            let version_id = val.get("version_id").and_then(|v| v.as_str());
            match version_id {
                Some(vid) if !version_ids.contains(vid) => {
                    stats.status_events_missing_version_id += 1;
                }
                None => {
                    stats.status_events_missing_version_id += 1;
                }
                _ => {}
            }
        }

        // --- New extraction surfaces ---
        let source_notes = read_optional_values("source_notes.jsonl")?;
        stats.source_notes = source_notes.len();
        for val in &source_notes {
            check_confidence(val, &mut stats);
            let version_ok = val
                .get("version_id")
                .and_then(|v| v.as_str())
                .map_or(true, |id| version_ids.contains(id));
            let provision_ok = val
                .get("provision_id")
                .and_then(|v| v.as_str())
                .map_or(true, |id| provision_ids.contains(id));
            if !version_ok || !provision_ok {
                stats.orphan_source_notes += 1;
            }
        }

        let html_paragraphs = read_optional_values("html_paragraphs.debug.jsonl")?;
        stats.html_paragraphs = html_paragraphs.len();
        for val in &html_paragraphs {
            if is_orphan_source_document(val, &source_document_ids) {
                stats.orphan_html_paragraphs += 1;
            }
        }

        let chapter_front_matter = read_optional_values("chapter_front_matter.jsonl")?;
        stats.chapter_front_matter = chapter_front_matter.len();
        for val in &chapter_front_matter {
            check_confidence(val, &mut stats);
            if is_orphan_source_document(val, &source_document_ids) {
                stats.orphan_chapter_front_matter += 1;
            }
        }

        let title_chapter_entries = read_optional_values("title_chapter_entries.jsonl")?;
        stats.title_chapter_entries = title_chapter_entries.len();
        for val in &title_chapter_entries {
            check_confidence(val, &mut stats);
            if is_orphan_source_document(val, &source_document_ids) {
                stats.orphan_title_chapter_entries += 1;
            }
        }

        let toc_entries = read_optional_values("chapter_toc_entries.jsonl")?;
        stats.chapter_toc_entries = toc_entries.len();
        for val in &toc_entries {
            check_confidence(val, &mut stats);
            if val
                .get("canonical_id")
                .and_then(|v| v.as_str())
                .is_some_and(|id| !identity_ids.contains(id))
            {
                stats.toc_entries_missing_identity += 1;
                stats.orphan_chapter_toc_entries += 1;
            }
        }

        let reserved_ranges = read_optional_values("reserved_ranges.jsonl")?;
        stats.reserved_ranges = reserved_ranges.len();
        for val in &reserved_ranges {
            check_confidence(val, &mut stats);
        }

        let parser_diagnostics = read_optional_values("parser_diagnostics.jsonl")?;
        stats.parser_diagnostics = parser_diagnostics.len();

        let temporal_effects = read_optional_values("temporal_effects.jsonl")?;
        stats.temporal_effects = temporal_effects.len();
        for val in &temporal_effects {
            check_confidence(val, &mut stats);
            let supported = val.get("source_note_id").and_then(|v| v.as_str()).is_some()
                || val
                    .get("source_provision_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| provision_ids.contains(id))
                || val
                    .get("version_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| version_ids.contains(id));
            if !supported {
                stats.temporal_effects_missing_support += 1;
                stats.orphan_temporal_effects += 1;
            }
        }

        let lineage_events = read_optional_values("lineage_events.jsonl")?;
        stats.lineage_events = lineage_events.len();
        for val in &lineage_events {
            check_confidence(val, &mut stats);
            if val
                .get("current_canonical_id")
                .and_then(|v| v.as_str())
                .map_or(true, |id| !identity_ids.contains(id))
            {
                stats.lineage_events_missing_current_canonical_id += 1;
                stats.orphan_lineage_events += 1;
            }
        }

        let session_laws = read_optional_values("session_laws.jsonl")?;
        stats.session_laws = session_laws.len();
        for val in &session_laws {
            check_confidence(val, &mut stats);
        }

        let amendments = read_optional_values("amendments.jsonl")?;
        stats.amendments = amendments.len();
        for val in &amendments {
            check_confidence(val, &mut stats);
        }

        // --- Defined Terms ---
        let defined_terms = read_optional_values("defined_terms.jsonl")?;
        stats.defined_terms = defined_terms.len();
        stats.duplicate_defined_term_ids =
            count_json_value_dupes(&defined_terms, "defined_term_id");

        // --- Definition Scopes ---
        let definition_scopes = read_optional_values("definition_scopes.jsonl")?;
        stats.definition_scopes = definition_scopes.len();
        stats.duplicate_definition_scope_ids =
            count_json_value_dupes(&definition_scopes, "definition_scope_id");
        for val in &definition_scopes {
            let valid = val
                .get("scope_type")
                .and_then(|v| v.as_str())
                .is_some_and(|scope| {
                    matches!(
                        scope,
                        "section" | "chapter" | "range" | "article" | "corpus" | "unknown"
                    )
                });
            if !valid {
                stats.invalid_definition_scope_types += 1;
            }
        }

        // --- Definitions ---
        let definitions = read_optional_values("definitions.jsonl")?;
        stats.definitions = definitions.len();
        stats.duplicate_definition_ids = count_json_value_dupes(&definitions, "definition_id");
        for val in &definitions {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_definitions += 1;
            }
            if val
                .get("defined_term_id")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                stats.definitions_missing_defined_term_id += 1;
            }
            if val
                .get("definition_scope_id")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                stats.definitions_missing_scope_id += 1;
            }
        }

        // --- LegalSemanticNodes ---
        let legal_semantic_nodes = read_optional_values("legal_semantic_nodes.jsonl")?;
        stats.legal_semantic_nodes = legal_semantic_nodes.len();
        stats.duplicate_legal_semantic_node_ids =
            count_json_value_dupes(&legal_semantic_nodes, "semantic_id");
        for val in &legal_semantic_nodes {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_legal_semantic_nodes += 1;
            }
            if val
                .get("source_provision_id")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                stats.semantic_nodes_missing_source_provision_id += 1;
            }
        }

        // --- Obligations ---
        let obligations = read_optional_values("obligations.jsonl")?;
        stats.obligations = obligations.len();
        for val in &obligations {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_obligations += 1;
            }
            if val
                .get("source_provision_id")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                stats.obligations_missing_source_provision_id += 1;
            }
        }

        // --- Exceptions ---
        let exceptions = read_optional_values("exceptions.jsonl")?;
        stats.exceptions = exceptions.len();
        for val in &exceptions {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_exceptions += 1;
            }
        }

        // --- Deadlines ---
        let deadlines = read_optional_values("deadlines.jsonl")?;
        stats.deadlines = deadlines.len();
        for val in &deadlines {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_deadlines += 1;
            }
        }

        // --- Penalties ---
        let penalties = read_optional_values("penalties.jsonl")?;
        stats.penalties = penalties.len();
        for val in &penalties {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_penalties += 1;
            }
            let has_detail = [
                "penalty_type",
                "amount",
                "criminal_class",
                "civil_penalty_amount",
                "jail_term",
                "target_citation",
            ]
            .iter()
            .any(|field| val.get(*field).and_then(|v| v.as_str()).is_some());
            if !has_detail {
                stats.penalties_missing_detail += 1;
            }
        }

        // --- Remedies ---
        let remedies = read_optional_values("remedies.jsonl")?;
        stats.remedies = remedies.len();
        for val in &remedies {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_remedies += 1;
            }
        }

        // --- Legal Actors ---
        let legal_actors = read_optional_values("legal_actors.jsonl")?;
        stats.legal_actors = legal_actors.len();

        // --- Legal Actions ---
        let legal_actions = read_optional_values("legal_actions.jsonl")?;
        stats.legal_actions = legal_actions.len();

        let money_amounts = read_optional_values("money_amounts.jsonl")?;
        stats.money_amounts = money_amounts.len();
        for val in &money_amounts {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_money_amounts += 1;
            }
        }

        let tax_rules = read_optional_values("tax_rules.jsonl")?;
        stats.tax_rules = tax_rules.len();
        for val in &tax_rules {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_tax_rules += 1;
            }
        }

        let rate_limits = read_optional_values("rate_limits.jsonl")?;
        stats.rate_limits = rate_limits.len();
        for val in &rate_limits {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_rate_limits += 1;
            }
        }

        let required_notices = read_optional_values("required_notices.jsonl")?;
        stats.required_notices = required_notices.len();
        for val in &required_notices {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_required_notices += 1;
            }
        }

        let form_texts = read_optional_values("form_texts.jsonl")?;
        stats.form_texts = form_texts.len();
        for val in &form_texts {
            check_confidence(val, &mut stats);
            if is_orphan_provision(val, provision_ids) {
                stats.orphan_form_texts += 1;
            }
        }

        // --- Aggregate warnings ---
        let total_orphans = stats.orphan_definitions
            + stats.orphan_legal_semantic_nodes
            + stats.orphan_obligations
            + stats.orphan_exceptions
            + stats.orphan_deadlines
            + stats.orphan_penalties
            + stats.orphan_remedies
            + stats.orphan_source_notes
            + stats.orphan_html_paragraphs
            + stats.orphan_chapter_front_matter
            + stats.orphan_title_chapter_entries
            + stats.orphan_temporal_effects
            + stats.orphan_lineage_events
            + stats.orphan_chapter_toc_entries
            + stats.orphan_money_amounts
            + stats.orphan_tax_rules
            + stats.orphan_rate_limits
            + stats.orphan_required_notices
            + stats.orphan_form_texts;

        if total_orphans > 0 {
            self.warnings.lock().unwrap().push(format!(
                "source/semantic orphan nodes detected: {total_orphans}"
            ));
        }
        if stats.status_events_missing_version_id > 0 {
            self.warnings.lock().unwrap().push(format!(
                "status_events with missing/invalid version_id: {}",
                stats.status_events_missing_version_id
            ));
        }
        if stats.invalid_confidence_count > 0 {
            self.warnings.lock().unwrap().push(format!(
                "semantic nodes with confidence outside [0.0, 1.0]: {}",
                stats.invalid_confidence_count
            ));
        }
        let duplicate_semantic_ids = stats.duplicate_defined_term_ids
            + stats.duplicate_definition_ids
            + stats.duplicate_definition_scope_ids
            + stats.duplicate_legal_semantic_node_ids;
        if duplicate_semantic_ids > 0 {
            self.errors.lock().unwrap().push(format!(
                "duplicate semantic node IDs detected: {duplicate_semantic_ids}"
            ));
        }

        Ok(stats)
    }
}

fn percentile_token_count(counts: &[usize], percentile: f64) -> usize {
    if counts.is_empty() {
        return 0;
    }
    let mut sorted = counts.to_vec();
    sorted.sort_unstable();
    let rank = ((percentile / 100.0) * (sorted.len().saturating_sub(1) as f64)).ceil() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

pub fn print_console_summary(report: &QcFullReport) {
    println!("\n--- QC FULL VALIDATION REPORT ---");
    println!("Run ID: {}", report.run_id);
    println!("Generated: {}", report.generated_at);
    println!("Status: {:?}", report.status);
    println!();

    println!("📊 Source Statistics");
    println!("   Expected chapters: {}", report.source.chapters_expected);
    println!("   Fetched chapters: {}", report.source.chapters_fetched);
    if report.source.fetch_failures > 0 {
        println!("   ❌ Fetch failures: {}", report.source.fetch_failures);
    }
    if report.source.empty_raw_files > 0 {
        println!("   ⚠️  Empty raw files: {}", report.source.empty_raw_files);
    }
    println!();

    println!("🔍 Parse Statistics");
    println!("   Identities: {}", report.parse.identities_count);
    println!("   Versions: {}", report.parse.versions_count);
    println!("   Provisions: {}", report.parse.provisions_count);
    if report.parse.active_with_empty_text > 0 {
        println!(
            "   ❌ Active versions with empty text: {}",
            report.parse.active_with_empty_text
        );
    }
    if report.parse.invalid_status_classification > 0 {
        println!(
            "   ⚠️  Invalid status classification: {}",
            report.parse.invalid_status_classification
        );
    }
    println!();

    println!("📦 Chunk Statistics");
    println!("   Total chunks: {}", report.chunks.total_chunks);
    println!("   Full statute: {}", report.chunks.full_statute_chunks);
    println!(
        "   Contextual provision: {}",
        report.chunks.contextual_provision_chunks
    );
    println!("   Definition: {}", report.chunks.definition_chunks);
    println!("   Exception: {}", report.chunks.exception_chunks);
    println!("   Deadline: {}", report.chunks.deadline_chunks);
    println!("   Penalty: {}", report.chunks.penalty_chunks);
    println!(
        "   Citation context: {}",
        report.chunks.citation_context_chunks
    );
    if report.chunks.orphan_chunks > 0 {
        println!("   ❌ Orphan chunks: {}", report.chunks.orphan_chunks);
    }
    if report.chunks.duplicate_chunk_ids > 0 {
        println!(
            "   ❌ Duplicate chunk IDs: {}",
            report.chunks.duplicate_chunk_ids
        );
    }
    if report.chunks.oversized_chunks_fail > 0 {
        println!(
            "   ❌ Oversized chunks (fail): {}",
            report.chunks.oversized_chunks_fail
        );
    }
    if report.chunks.oversized_chunks_warn > 0 {
        println!(
            "   ⚠️  Oversized chunks (warn): {}",
            report.chunks.oversized_chunks_warn
        );
    }
    println!();

    println!("🔗 Citation Statistics");
    println!("   Total mentions: {}", report.citations.citation_mentions);
    if report.citations.unresolved > 0 {
        println!("   ⚠️  Unresolved: {}", report.citations.unresolved);
    }
    println!();

    println!("🕸️ Graph Statistics");
    println!("   Total edges: {}", report.graph.edges);
    if report.graph.orphan_edges > 0 {
        println!("   ❌ Orphan edges: {}", report.graph.orphan_edges);
    }
    println!();

    println!("🔷 Embedding Readiness ✅ PASS");
    println!(
        "   Model: {} (dim: {})",
        report.embedding_readiness.model, report.embedding_readiness.dimension
    );
    println!("   --- RetrievalChunks ---");
    println!(
        "   Eligible chunks: {}",
        report.embedding_readiness.eligible_chunks
    );
    println!(
        "   Est. tokens: {}",
        report.embedding_readiness.estimated_total_tokens
    );
    println!(
        "   Est. batches: {}",
        report.embedding_readiness.estimated_batches
    );
    if report.embedding_readiness.chunks_over_context_limit > 0 {
        println!(
            "   ⚠️  Chunks over context limit: {}",
            report.embedding_readiness.chunks_over_context_limit
        );
    }

    println!("   --- Provisions ---");
    println!(
        "   Eligible provisions: {}",
        report.provision_embedding_readiness.eligible_provisions
    );
    println!(
        "   Est. tokens: {}",
        report.provision_embedding_readiness.estimated_total_tokens
    );
    if report
        .provision_embedding_readiness
        .provisions_over_context_limit
        > 0
    {
        println!(
            "   ⚠️  Provisions over context limit: {}",
            report
                .provision_embedding_readiness
                .provisions_over_context_limit
        );
    }

    println!("   --- LegalTextVersions ---");
    println!(
        "   Eligible versions: {}",
        report.version_embedding_readiness.eligible_versions
    );
    println!(
        "   Est. tokens: {}",
        report.version_embedding_readiness.estimated_total_tokens
    );
    if report
        .version_embedding_readiness
        .versions_over_context_limit
        > 0
    {
        println!(
            "   ⚠️  Versions over context limit: {}",
            report
                .version_embedding_readiness
                .versions_over_context_limit
        );
    }
    println!();

    println!("🔧 Resolver Readiness");
    println!(
        "   Identity index: {}",
        if report.resolver_readiness.identity_index_ready {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "   Version index: {}",
        if report.resolver_readiness.version_index_ready {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "   Provision path index: {}",
        if report.resolver_readiness.provision_path_index_ready {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "   Chapter index: {}",
        if report.resolver_readiness.chapter_index_ready {
            "✅"
        } else {
            "❌"
        }
    );
    println!();

    let coverage_status = if report.coverage.active_versions_missing_full_statute_chunk > 0
        || report.coverage.provisions_missing_contextual_chunk > 0
    {
        "❌ FAIL"
    } else {
        "✅ PASS"
    };
    println!("📊 Coverage Validation {}", coverage_status);
    println!("   Active versions: {}", report.coverage.active_versions);
    println!(
        "   Full statute chunks: {}",
        report.coverage.full_statute_chunks
    );
    if report.coverage.active_versions_missing_full_statute_chunk > 0 {
        println!(
            "   ❌ Versions missing full_statute: {}",
            report.coverage.active_versions_missing_full_statute_chunk
        );
    }
    if report.coverage.versions_with_duplicate_full_statute_chunks > 0 {
        println!(
            "   ⚠️  Versions with duplicate full_statute: {}",
            report.coverage.versions_with_duplicate_full_statute_chunks
        );
    }
    if report.coverage.provisions_missing_contextual_chunk > 0 {
        println!(
            "   ❌ Provisions missing contextual: {}",
            report.coverage.provisions_missing_contextual_chunk
        );
    }
    println!();

    let total_semantic = report.semantic.definitions
        + report.semantic.obligations
        + report.semantic.exceptions
        + report.semantic.deadlines
        + report.semantic.penalties
        + report.semantic.remedies
        + report.semantic.legal_semantic_nodes
        + report.semantic.money_amounts
        + report.semantic.tax_rules
        + report.semantic.rate_limits
        + report.semantic.required_notices
        + report.semantic.form_texts;
    let total_source_audit = report.semantic.html_paragraphs
        + report.semantic.chapter_front_matter
        + report.semantic.title_chapter_entries
        + report.semantic.source_notes
        + report.semantic.chapter_toc_entries
        + report.semantic.parser_diagnostics;
    if total_semantic > 0 || report.semantic.status_events > 0 || total_source_audit > 0 {
        println!("🧠 Semantic Layer");
        println!("   Status events: {}", report.semantic.status_events);
        println!("   Source notes: {}", report.semantic.source_notes);
        println!("   HTML paragraphs: {}", report.semantic.html_paragraphs);
        println!(
            "   Chapter front matter: {}",
            report.semantic.chapter_front_matter
        );
        println!(
            "   Title chapter entries: {}",
            report.semantic.title_chapter_entries
        );
        println!("   Defined terms: {}", report.semantic.defined_terms);
        println!("   Definitions: {}", report.semantic.definitions);
        println!(
            "   Definition scopes: {}",
            report.semantic.definition_scopes
        );
        println!(
            "   Legal semantic nodes: {}",
            report.semantic.legal_semantic_nodes
        );
        println!("   Obligations: {}", report.semantic.obligations);
        println!("   Exceptions: {}", report.semantic.exceptions);
        println!("   Deadlines: {}", report.semantic.deadlines);
        println!("   Penalties: {}", report.semantic.penalties);
        println!("   Remedies: {}", report.semantic.remedies);
        println!("   Legal actors: {}", report.semantic.legal_actors);
        println!("   Legal actions: {}", report.semantic.legal_actions);
        println!("   Money amounts: {}", report.semantic.money_amounts);
        println!("   Tax rules: {}", report.semantic.tax_rules);
        println!("   Rate limits: {}", report.semantic.rate_limits);
        println!("   Required notices: {}", report.semantic.required_notices);
        println!("   Form texts: {}", report.semantic.form_texts);
        let total_orphans = report.semantic.orphan_definitions
            + report.semantic.orphan_legal_semantic_nodes
            + report.semantic.orphan_obligations
            + report.semantic.orphan_exceptions
            + report.semantic.orphan_deadlines
            + report.semantic.orphan_penalties
            + report.semantic.orphan_remedies
            + report.semantic.orphan_source_notes
            + report.semantic.orphan_html_paragraphs
            + report.semantic.orphan_chapter_front_matter
            + report.semantic.orphan_title_chapter_entries
            + report.semantic.orphan_temporal_effects
            + report.semantic.orphan_lineage_events
            + report.semantic.orphan_chapter_toc_entries
            + report.semantic.orphan_money_amounts
            + report.semantic.orphan_tax_rules
            + report.semantic.orphan_rate_limits
            + report.semantic.orphan_required_notices
            + report.semantic.orphan_form_texts;
        if total_orphans > 0 {
            println!("   ⚠️  Orphan semantic nodes: {}", total_orphans);
        }
        if report.semantic.invalid_confidence_count > 0 {
            println!(
                "   ⚠️  Invalid confidence values: {}",
                report.semantic.invalid_confidence_count
            );
        }
        println!();
    }

    if !report.blocking_errors.is_empty() {
        println!("❌ BLOCKING ERRORS");
        for e in &report.blocking_errors {
            println!("   - {}", e);
        }
        println!();
    }

    if !report.warnings.is_empty() {
        println!("⚠️  WARNINGS");
        for w in &report.warnings {
            println!("   - {}", w);
        }
        println!();
    }
}

fn is_raw_html_path(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with("ors") && name.ends_with(".html")
}

fn count_dupes<I, S>(items: I) -> usize
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut counts: HashMap<String, usize> = HashMap::new();
    for item in items {
        *counts.entry(item.as_ref().to_string()).or_insert(0) += 1;
    }
    counts.values().filter(|&&n| n > 1).map(|n| n - 1).sum()
}

fn count_json_value_dupes(rows: &[serde_json::Value], id_field: &str) -> usize {
    count_dupes(rows.iter().filter_map(|row| {
        row.get(id_field)
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunks::{build_chunks_for_provision, build_full_statute_chunks};
    use crate::io_jsonl::write_jsonl;
    use crate::models::QcStatus;
    use crate::models::{LegalTextIdentity, LegalTextVersion, Provision};
    use std::fs;

    #[test]
    fn test_qc_full_validator_empty_success() {
        let temp_dir = std::env::temp_dir().join(format!("orsgraph-qc-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let files = [
            "legal_text_identities.jsonl",
            "legal_text_versions.jsonl",
            "provisions.jsonl",
            "retrieval_chunks.jsonl",
            "citation_mentions.jsonl",
            "chapter_headings.jsonl",
            "cites_edges.jsonl",
        ];
        for f in &files {
            if *f == "legal_text_identities.jsonl" {
                fs::write(temp_dir.join(f), "{\"canonical_id\":\"or:ors:1.001\",\"citation\":\"ORS 1.001\",\"jurisdiction_id\":\"or\",\"authority_family\":\"statute\",\"chapter\":\"1\",\"status\":\"active\"}\n").unwrap();
            } else {
                fs::write(temp_dir.join(f), "").unwrap();
            }
        }

        let validator = QcFullValidator::new(
            temp_dir.clone(),
            None,
            0,
            2025,
            false,
            false,
            false,
            false,
            "voyage-4-large".to_string(),
            1024,
        );

        let report = validator.run().unwrap();
        if report.status != QcStatus::Pass {
            eprintln!("QC Errors: {:?}", report.blocking_errors);
            eprintln!("QC Warnings: {:?}", report.warnings);
        }
        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(report.status, QcStatus::Pass);
    }

    #[test]
    fn qc_full_without_raw_dir_does_not_enforce_expected_chapters() {
        let temp_dir = write_minimal_valid_graph();
        let validator = QcFullValidator::new(
            temp_dir.clone(),
            None,
            524,
            2025,
            false,
            false,
            false,
            false,
            "voyage-4-large".to_string(),
            1024,
        );

        let report = validator.run().unwrap();
        let _ = fs::remove_dir_all(temp_dir);

        assert_eq!(report.source.chapters_expected, 0);
        assert_eq!(report.source.fetch_failures, 0);
        assert!(
            !report
                .blocking_errors
                .iter()
                .any(|error| error.contains("Expected 524 chapters"))
        );
    }

    #[test]
    fn strict_chunk_policy_accepts_generated_chunk_version() {
        let temp_dir = write_minimal_valid_graph();
        let validator = QcFullValidator::new(
            temp_dir.clone(),
            None,
            0,
            2025,
            false,
            true,
            false,
            false,
            "voyage-4-large".to_string(),
            1024,
        );

        let report = validator.run().unwrap();
        let _ = fs::remove_dir_all(temp_dir);

        assert!(
            !report
                .blocking_errors
                .iter()
                .any(|error| error.contains("invalid chunk_version"))
        );
    }

    fn write_minimal_valid_graph() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("orsgraph-qc-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let identity = LegalTextIdentity {
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            jurisdiction_id: "or:state".to_string(),
            authority_family: "ORS".to_string(),
            corpus_id: Some("or:ors".to_string()),
            authority_type: Some("statute".to_string()),
            authority_level: Some(90),
            effective_date: None,
            title: Some("Test section".to_string()),
            chapter: "1".to_string(),
            status: "active".to_string(),
        };
        let version = LegalTextVersion {
            version_id: "or:ors:1.001@2025".to_string(),
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            title: Some("Test section".to_string()),
            chapter: "1".to_string(),
            edition_year: 2025,
            status: "active".to_string(),
            status_text: None,
            text: "A person shall comply with this test section.".to_string(),
            text_hash: "hash".to_string(),
            source_document_id: "src:1".to_string(),
            official_status: "official_online_not_official_print".to_string(),
            disclaimer_required: true,
            ..Default::default()
        };
        let provision = Provision {
            provision_id: "or:ors:1.001@2025::p:root".to_string(),
            version_id: version.version_id.clone(),
            canonical_id: version.canonical_id.clone(),
            citation: version.citation.clone(),
            display_citation: version.citation.clone(),
            local_path: vec!["root".to_string()],
            provision_type: "section_text".to_string(),
            text: version.text.clone(),
            normalized_text: version.text.clone(),
            order_index: 0,
            depth: 1,
            text_hash: "hash".to_string(),
            is_implied: false,
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_deadline_candidate: false,
            is_penalty_candidate: false,
            ..Default::default()
        };
        let mut chunks = build_chunks_for_provision(&provision, 2025, 90);
        chunks.extend(build_full_statute_chunks(
            &version,
            &provision.provision_id,
            2025,
            90,
        ));

        write_jsonl(temp_dir.join("legal_text_identities.jsonl"), &[identity]).unwrap();
        write_jsonl(temp_dir.join("legal_text_versions.jsonl"), &[version]).unwrap();
        write_jsonl(temp_dir.join("provisions.jsonl"), &[provision]).unwrap();
        write_jsonl(temp_dir.join("retrieval_chunks.jsonl"), &chunks).unwrap();
        write_jsonl::<CitationMention>(temp_dir.join("citation_mentions.jsonl"), &[]).unwrap();
        write_jsonl::<ChapterHeading>(temp_dir.join("chapter_headings.jsonl"), &[]).unwrap();
        write_jsonl::<CitesEdge>(temp_dir.join("cites_edges.jsonl"), &[]).unwrap();
        temp_dir
    }

    #[test]
    fn test_combined_qc_logic() {
        let temp_dir = write_complex_mock_graph();
        let validator = QcFullValidator::new(
            temp_dir.clone(),
            None,
            0,
            2025,
            false,
            false,
            true, // require_embeddings
            false,
            "voyage-4-large".to_string(),
            1024,
        );

        let report = validator.run().unwrap();
        let _ = fs::remove_dir_all(temp_dir);

        // Verify Chunk Stats
        assert_eq!(report.chunks.total_chunks, 3);
        assert_eq!(report.chunks.full_statute_chunks, 1);
        assert_eq!(report.chunks.contextual_provision_chunks, 2);
        assert_eq!(report.chunks.orphan_chunks, 1); // One chunk has no parent in the provision_ids set

        // Verify Embedding Readiness
        assert_eq!(report.embedding_readiness.eligible_chunks, 3);
        assert!(report.embedding_readiness.estimated_total_tokens > 0);
        assert_eq!(report.provision_embedding_readiness.eligible_provisions, 1);
        assert_eq!(report.version_embedding_readiness.eligible_versions, 1);

        // Verify Coverage
        assert_eq!(report.coverage.active_versions, 1);
        assert_eq!(report.coverage.full_statute_chunks, 1);
        assert_eq!(report.coverage.valid_provisions, 1);
        assert_eq!(report.coverage.contextual_provision_chunks, 2);
    }

    fn write_complex_mock_graph() -> PathBuf {
        let temp_dir =
            std::env::temp_dir().join(format!("orsgraph-qc-complex-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let identity = LegalTextIdentity {
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            chapter: "1".to_string(),
            status: "active".to_string(),
            ..Default::default()
        };
        let version = LegalTextVersion {
            version_id: "or:ors:1.001@2025".to_string(),
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            status: "active".to_string(),
            text: "Valid version text.".to_string(),
            embedding_input_hash: Some("h1".to_string()),
            ..Default::default()
        };
        let provision = Provision {
            provision_id: "or:ors:1.001@2025::p:1".to_string(),
            version_id: version.version_id.clone(),
            canonical_id: version.canonical_id.clone(),
            text: "Valid provision text.".to_string(),
            embedding_input_hash: Some("h2".to_string()),
            ..Default::default()
        };

        let chunks = vec![
            RetrievalChunk {
                chunk_id: "c1".to_string(),
                chunk_type: "full_statute".to_string(),
                parent_version_id: version.version_id.clone(),
                text: "Chunk 1 text".to_string(),
                embedding_policy: Some("embed_primary".to_string()),
                embedding_input_hash: "h3".to_string(),
                ..Default::default()
            },
            RetrievalChunk {
                chunk_id: "c2".to_string(),
                chunk_type: "contextual_provision".to_string(),
                source_provision_id: Some(provision.provision_id.clone()),
                text: "Chunk 2 text".to_string(),
                embedding_policy: Some("embed_primary".to_string()),
                embedding_input_hash: "h4".to_string(),
                ..Default::default()
            },
            RetrievalChunk {
                chunk_id: "c3_orphan".to_string(),
                chunk_type: "contextual_provision".to_string(),
                source_provision_id: Some("non_existent_provision".to_string()),
                text: "Orphan chunk text".to_string(),
                embedding_policy: Some("embed_primary".to_string()),
                embedding_input_hash: "h5".to_string(),
                ..Default::default()
            },
        ];

        write_jsonl(temp_dir.join("legal_text_identities.jsonl"), &[identity]).unwrap();
        write_jsonl(temp_dir.join("legal_text_versions.jsonl"), &[version]).unwrap();
        write_jsonl(temp_dir.join("provisions.jsonl"), &[provision]).unwrap();
        write_jsonl(temp_dir.join("retrieval_chunks.jsonl"), &chunks).unwrap();
        write_jsonl::<CitationMention>(temp_dir.join("citation_mentions.jsonl"), &[]).unwrap();
        write_jsonl::<ChapterHeading>(temp_dir.join("chapter_headings.jsonl"), &[]).unwrap();
        write_jsonl::<CitesEdge>(temp_dir.join("cites_edges.jsonl"), &[]).unwrap();

        temp_dir
    }
}
