use ors_crawler_v0::{
    embeddings, io_jsonl, models, neo4j_loader, ors_dom_parser, qc, qc_full, qc_neo4j, resolver,
    semantic, voyage,
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use io_jsonl::{read_jsonl_batches, write_jsonl, write_jsonl_atomic, write_one_json};
use models::{
    Amendment, ChapterFrontMatter, ChapterHeading, ChapterTocEntry, CitationMention, CitesEdge,
    Deadline, DefinedTerm, Definition, DefinitionScope, Exception, FormText, HtmlParagraph,
    LegalAction, LegalActor, LegalSemanticNode, LegalTextIdentity, LegalTextVersion, LineageEvent,
    MoneyAmount, Obligation, ParserDiagnostic, ParserDiagnostics, Penalty, Provision, QcStatus,
    RateLimit, Remedy, RequiredNotice, ReservedRange, RetrievalChunk, SessionLaw, SourceDocument,
    SourceNote, StatusEvent, TaxRule, TemporalEffect, TimeInterval, TitleChapterEntry,
};
use neo4rs::query;
use ors_dom_parser::parse_ors_chapter_html;
use qc::validate_outputs;
use qc_full::{print_console_summary, QcFullValidator};
use reqwest::{Client, StatusCode};
use resolver::{build_global_symbol_table, resolve_all_citations};
use scraper::{Html, Selector};
use semantic::{
    derive_historical_nodes, derive_note_semantics, derive_provision_temporal_effects,
    derive_semantic_nodes, derive_session_laws_from_amendments, derive_source_note_status_events,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};
use voyage::{estimate_tokens, model_config, VoyageClient};

// ── CLI ────────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "ors-crawler")]
#[command(about = "ORS crawler/parser")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    ParseLocal {
        #[arg(long)]
        input: PathBuf,

        #[arg(long)]
        out: PathBuf,

        #[arg(long)]
        chapter: String,

        #[arg(long, default_value_t = 2025)]
        edition_year: i32,

        #[arg(long)]
        source_url: Option<String>,

        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
    },
    Crawl {
        #[arg(long, default_value = "data")]
        out: PathBuf,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = 900)]
        delay_ms: u64,
        #[arg(long, default_value_t = 0)]
        max_chapters: usize,
        #[arg(long)]
        chapters: Option<String>,
        #[arg(
            long,
            env = "ORS_CRAWLER_USER_AGENT",
            default_value = "NeighborOS-ORSGraph/0.1 research crawler"
        )]
        user_agent: String,
        #[arg(long)]
        fetch_only: bool,
        #[arg(long)]
        skip_citation_resolution: bool,
    },
    QcFull {
        #[arg(long)]
        graph_dir: PathBuf,
        #[arg(long)]
        raw_dir: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 524)]
        expected_chapters: usize,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = false)]
        require_resolved_citations: bool,
        #[arg(long, default_value_t = false)]
        strict_chunk_policy: bool,
        #[arg(long, default_value_t = false)]
        require_embeddings: bool,
        #[arg(long, default_value_t = false)]
        require_golden: bool,
        #[arg(long, default_value = "voyage-4-large")]
        embedding_model: String,
        #[arg(long, default_value_t = 1024)]
        embedding_dim: usize,
    },
    Rag {
        #[arg(long, env = "NEO4J_URI")]
        uri: String,
        #[arg(long, env = "NEO4J_USER")]
        user: String,
        #[arg(long, env = "NEO4J_PASS")]
        pass: String,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long, env = "VOYAGE_API_KEY")]
        voyage_key: String,
    },

    SeedNeo4j {
        #[arg(long)]
        graph_dir: PathBuf,
        #[arg(long)]
        neo4j_uri: String,
        #[arg(long)]
        neo4j_user: String,
        #[arg(long)]
        neo4j_password_env: String,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = false)]
        embed: bool,
        #[arg(long, default_value = "legal_chunk_primary_v1")]
        embedding_profile: String,
        #[arg(long, default_value = "voyage-4-large")]
        embedding_model: String,
        #[arg(long, default_value_t = 1024)]
        embedding_dimension: i32,
        #[arg(long, default_value = "float")]
        embedding_dtype: String,
        #[arg(long, default_value_t = 100)]
        embedding_batch_size: usize,
        #[arg(long, default_value_t = 500_000)]
        max_batch_chars: usize,
        #[arg(long, default_value_t = 110_000)]
        max_batch_estimated_tokens: usize,
        #[arg(long, default_value_t = false)]
        create_vector_index: bool,
        #[arg(long, default_value_t = true)]
        embed_chunks: bool,
        #[arg(long, default_value_t = false)]
        embed_provisions: bool,
        #[arg(long, default_value_t = false)]
        embed_versions: bool,
        #[arg(long, default_value_t = false)]
        resume_embeddings: bool,
        #[arg(long, value_enum, default_value_t = ChunkFilePolicy::RootOnly)]
        chunk_file_policy: ChunkFilePolicy,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = 5000)]
        node_batch_size: usize,
        #[arg(long, default_value_t = 5000)]
        edge_batch_size: usize,
        #[arg(long, default_value_t = 5000)]
        relationship_batch_size: usize,
    },

    MaterializeNeo4j {
        #[arg(long)]
        graph_dir: PathBuf,
        #[arg(long)]
        neo4j_uri: String,
        #[arg(long)]
        neo4j_user: String,
        #[arg(long)]
        neo4j_password_env: String,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = 5000)]
        edge_batch_size: usize,
        #[arg(long, default_value_t = 5000)]
        relationship_batch_size: usize,
    },

    QcNeo4j {
        #[arg(long)]
        graph_dir: Option<PathBuf>,
        #[arg(long)]
        neo4j_uri: String,
        #[arg(long)]
        neo4j_user: String,
        #[arg(long)]
        neo4j_password_env: String,
        #[arg(long, default_value_t = false)]
        require_embeddings: bool,
        #[arg(long, default_value = "legal_chunk_primary_v1")]
        embedding_profile: String,
        #[arg(long, default_value = "voyage-4-large")]
        embedding_model: String,
        #[arg(long, default_value_t = 1024)]
        embedding_dim: i32,
        #[arg(long, default_value = "float")]
        embedding_dtype: String,
    },
    EmbedNeo4j {
        #[arg(long, env = "NEO4J_URI")]
        neo4j_uri: String,
        #[arg(long, env = "NEO4J_USER")]
        neo4j_user: String,
        #[arg(long, default_value = "NEO4J_PASSWORD")]
        neo4j_password_env: String,
        #[arg(long, env = "VOYAGE_API_KEY")]
        voyage_key: String,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        smoke: bool,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        resume: bool,
        #[arg(long, default_value_t = false)]
        create_vector_indexes: bool,
        #[arg(long)]
        phase: Vec<u8>,
        #[arg(long)]
        max_label_nodes: Option<usize>,
        #[arg(long, default_value_t = 100)]
        embedding_batch_size: usize,
        #[arg(long, default_value_t = 500)]
        scan_batch_size: usize,
        #[arg(long, default_value_t = 500_000)]
        max_batch_chars: usize,
        #[arg(long, default_value_t = 110_000)]
        max_batch_estimated_tokens: usize,
    },
    ParseCached {
        #[arg(long, default_value = "data/raw/official")]
        raw_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        chapters: String,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
        #[arg(long, default_value_t = false)]
        append: bool,
    },
    ResolveCitations {
        #[arg(long)]
        graph_dir: PathBuf,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
    },
    ClearNeo4j {
        #[arg(long)]
        neo4j_uri: String,
        #[arg(long)]
        neo4j_user: String,
        #[arg(long)]
        neo4j_password: Option<String>,
        #[arg(long, default_value = "NEO4J_PASSWORD")]
        neo4j_password_env: String,
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ChunkFilePolicy {
    RootOnly,
    Recursive,
}

// ── Stats types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ParseStats {
    chapter: String,
    edition_year: i32,
    sections_parsed: usize,
    provisions_parsed: usize,
    citations_extracted: usize,
    chunks_created: usize,
    source_notes: usize,
    amendments: usize,
    parser_diagnostics: ParserDiagnostics,
    duplicate_provision_ids: usize,
    duplicate_version_ids: usize,
    duplicate_provision_paths: usize,
    orphan_chunks: usize,
    orphan_citations: usize,
    active_sections_missing_titles: usize,
    heading_leaks: usize,
    artifact_leaks: usize,
    reserved_tail_leaks: usize,
    chunk_year_mismatches: usize,
    contextual_chunks: usize,
    valid_provisions: usize,
    qc_failed: bool,
    qc_errors: Vec<String>,
    qc_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrawlStats {
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    duration_secs: Option<f64>,
    chapters_discovered: usize,
    chapters_cached: usize,
    chapters_new_fetched: usize,
    chapters_failed: usize,
    total_raw_bytes: u64,
    sections_parsed: usize,
    provisions_parsed: usize,
    citations_extracted: usize,
    citations_resolved: usize,
    citations_unresolved: usize,
    chunks_created: usize,
    qc_warnings: usize,
    qc_failures: usize,
    citation_warnings: usize,
    citation_errors: usize,
    failed_chapters: Vec<String>,
}

// ── Progress reporting ─────────────────────────────────────────────────────────

struct Progress {
    phase: &'static str,
    total: usize,
    done: usize,
    failed: usize,
    start: Instant,
}

impl Progress {
    fn new(phase: &'static str, total: usize) -> Self {
        Self {
            phase,
            total,
            done: 0,
            failed: 0,
            start: Instant::now(),
        }
    }

    fn tick(&mut self, label: &str, detail: &str) {
        self.done += 1;
        let elapsed = self.start.elapsed().as_secs_f64();
        let rate = self.done as f64 / elapsed.max(0.001);
        let remaining = self.total.saturating_sub(self.done);
        let eta_secs = remaining as f64 / rate.max(0.001);
        info!(
            "[{} {}/{}]  {}  {}  ({:.1}/s, ETA {})",
            self.phase,
            self.done,
            self.total,
            label,
            detail,
            rate,
            format_duration(eta_secs),
        );
    }

    fn tick_fail(&mut self, label: &str, detail: &str) {
        self.done += 1;
        self.failed += 1;
        let elapsed = self.start.elapsed().as_secs_f64();
        let rate = self.done as f64 / elapsed.max(0.001);
        let remaining = self.total.saturating_sub(self.done);
        let eta_secs = remaining as f64 / rate.max(0.001);
        warn!(
            "[{} {}/{}]  {}  FAILED: {}  ({:.1}/s, ETA {})",
            self.phase,
            self.done,
            self.total,
            label,
            detail,
            rate,
            format_duration(eta_secs),
        );
    }

    fn summary(&self) -> String {
        let elapsed = self.start.elapsed().as_secs_f64();
        format!(
            "{} complete: {}/{} succeeded, {} failed in {}",
            self.phase,
            self.done - self.failed,
            self.total,
            self.failed,
            format_duration(elapsed),
        )
    }
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        format!("{}m{:02}s", secs as u64 / 60, secs as u64 % 60)
    } else {
        format!("{}h{:02}m", secs as u64 / 3600, (secs as u64 % 3600) / 60)
    }
}

fn parse_embedding_phases(values: &[u8]) -> Result<BTreeSet<embeddings::EmbeddingPhase>> {
    if values.is_empty() {
        return Ok(BTreeSet::from([
            embeddings::EmbeddingPhase::Chunks,
            embeddings::EmbeddingPhase::Authority,
            embeddings::EmbeddingPhase::Semantic,
            embeddings::EmbeddingPhase::DefinitionsHistory,
            embeddings::EmbeddingPhase::Specialized,
        ]));
    }

    values
        .iter()
        .copied()
        .map(embeddings::EmbeddingPhase::from_u8)
        .collect()
}

// ── Main ───────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Install rustls crypto provider for TLS connections (required for neo4j+s://)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();

    match cli.command {
        Command::Rag {
            uri,
            user,
            pass,
            query,
            limit,
            voyage_key,
        } => {
            let loader = neo4j_loader::Neo4jLoader::new(&uri, &user, &pass).await?;
            let voyage = voyage::VoyageClient::new(voyage_key, "voyage-law-2")?;
            let augmentor = ors_crawler_v0::rag::RetrievalAugmentor::new(loader);

            info!("Generating embedding for query: '{}'...", query);
            let embedding = voyage.embed_query(&query, Some("float")).await?;

            info!("Performing hybrid search and multi-hop enrichment...");
            let context = augmentor.retrieve_context(&query, embedding, limit).await?;

            println!("\n{}", context);
            Ok(())
        }
        Command::ParseLocal {
            input,
            out,
            chapter,
            edition_year,
            source_url,
            fail_on_qc,
        } => {
            let html = read_raw_html(&input)?;

            let source_url = source_url.unwrap_or_else(|| {
                format!(
                    "https://www.oregonlegislature.gov/bills_laws/ors/ors{:0>3}.html",
                    chapter
                )
            });

            let parsed = parse_ors_chapter_html(&html, &source_url, &chapter, edition_year)?;

            let graph_dir = out.join("graph");
            write_jsonl(
                graph_dir.join("source_documents.jsonl"),
                &[parsed.source_document.clone()],
            )?;
            write_jsonl(
                graph_dir.join("legal_text_identities.jsonl"),
                &parsed.identities,
            )?;
            write_jsonl(
                graph_dir.join("legal_text_versions.jsonl"),
                &parsed.versions,
            )?;
            write_jsonl(graph_dir.join("provisions.jsonl"), &parsed.provisions)?;
            write_jsonl(graph_dir.join("citation_mentions.jsonl"), &parsed.citations)?;
            write_jsonl(graph_dir.join("retrieval_chunks.jsonl"), &parsed.chunks)?;
            write_jsonl(graph_dir.join("chapter_headings.jsonl"), &parsed.headings)?;
            write_jsonl(
                graph_dir.join("html_paragraphs.debug.jsonl"),
                &parsed.html_paragraphs_debug,
            )?;
            write_jsonl(
                graph_dir.join("chapter_front_matter.jsonl"),
                &parsed.chapter_front_matter,
            )?;
            write_jsonl(
                graph_dir.join("title_chapter_entries.jsonl"),
                &parsed.title_chapter_entries,
            )?;
            write_jsonl(graph_dir.join("source_notes.jsonl"), &parsed.source_notes)?;
            write_jsonl(
                graph_dir.join("chapter_toc_entries.jsonl"),
                &parsed.chapter_toc_entries,
            )?;
            write_jsonl(
                graph_dir.join("reserved_ranges.jsonl"),
                &parsed.reserved_ranges,
            )?;
            write_jsonl(
                graph_dir.join("parser_diagnostics.jsonl"),
                &parsed.parser_diagnostic_rows,
            )?;
            write_derived_graph_nodes(&graph_dir, &parsed)?;

            let report = validate_outputs(
                &parsed.versions,
                &parsed.provisions,
                &parsed.citations,
                &parsed.chunks,
            );

            let stats = ParseStats {
                chapter,
                edition_year,
                sections_parsed: parsed.versions.len(),
                provisions_parsed: parsed.provisions.len(),
                citations_extracted: parsed.citations.len(),
                chunks_created: parsed.chunks.len(),
                source_notes: parsed.source_notes.len(),
                amendments: parsed.amendments.len(),
                parser_diagnostics: parsed.parser_diagnostics.clone(),
                duplicate_provision_ids: report.duplicate_provision_ids,
                duplicate_version_ids: report.duplicate_version_ids,
                duplicate_provision_paths: report.duplicate_provision_paths,
                orphan_chunks: report.orphan_chunks,
                orphan_citations: report.orphan_citations,
                active_sections_missing_titles: report.active_sections_missing_titles,
                heading_leaks: report.heading_leaks,
                artifact_leaks: report.artifact_leaks,
                reserved_tail_leaks: report.reserved_tail_leaks,
                chunk_year_mismatches: report.chunk_year_mismatches,
                contextual_chunks: report.contextual_chunks,
                valid_provisions: report.valid_provisions,
                qc_failed: report.is_blocking_failure(),
                qc_errors: report.errors.clone(),
                qc_warnings: report.warnings.clone(),
            };

            write_one_json(out.join("stats.json"), &stats)?;

            if fail_on_qc && report.is_blocking_failure() {
                return Err(anyhow!("QC failed: {:?}", report.errors));
            }

            Ok(())
        }
        Command::Crawl {
            out,
            edition_year,
            delay_ms,
            max_chapters,
            chapters,
            user_agent,
            fetch_only,
            skip_citation_resolution,
        } => {
            run_crawl(
                out,
                edition_year,
                delay_ms,
                max_chapters,
                chapters,
                user_agent,
                fetch_only,
                skip_citation_resolution,
            )
            .await?;
            Ok(())
        }
        Command::QcFull {
            graph_dir,
            raw_dir,
            out,
            expected_chapters,
            edition_year,
            require_resolved_citations,
            strict_chunk_policy,
            require_embeddings,
            require_golden,
            embedding_model,
            embedding_dim,
        } => {
            let validator = QcFullValidator::new(
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
            );
            let report = validator.run()?;

            print_console_summary(&report);

            fs::create_dir_all(&out)?;
            let report_path = out.join("qc_report.json");
            io_jsonl::write_one_json(&report_path, &report)?;
            println!("\n📄 Report written to: {}", report_path.display());

            match report.status {
                QcStatus::Pass | QcStatus::Warning => Ok(()),
                QcStatus::Fail => Err(anyhow!(
                    "QC validation failed with {} blocking errors",
                    report.blocking_errors.len()
                )),
            }
        }
        Command::SeedNeo4j {
            graph_dir,
            neo4j_uri,
            neo4j_user,
            neo4j_password_env,
            edition_year,
            embed,
            embedding_profile,
            embedding_model,
            embedding_dimension,
            embedding_dtype,
            embedding_batch_size,
            max_batch_chars,
            max_batch_estimated_tokens,
            create_vector_index,
            embed_chunks,
            embed_provisions,
            embed_versions,
            resume_embeddings,
            chunk_file_policy,
            dry_run,
            node_batch_size,
            edge_batch_size,
            relationship_batch_size,
        } => {
            if dry_run {
                let counts = validate_seed_graph_contract(&graph_dir, chunk_file_policy)?;
                info!("Seed dry-run validated {} JSONL rows", counts.total_rows());
                for (file, rows) in counts.files {
                    info!("  {}: {} rows", file, rows);
                }
                return Ok(());
            }

            let pass = std::env::var(neo4j_password_env)?;
            let loader = neo4j_loader::Neo4jLoader::new(&neo4j_uri, &neo4j_user, &pass).await?;

            // Version check for Cypher 25 SEARCH support
            let (_, _, version) = loader.health_check().await?;
            if !neo4j_loader::Neo4jLoader::supports_search_clause(&version) {
                warn!("Neo4j version {} may not support Cypher 25 SEARCH clause. Vector search might fail.", version);
            }

            let seed_batch_config = neo4j_loader::SeedBatchConfig::new(
                node_batch_size,
                edge_batch_size,
                relationship_batch_size,
            );
            run_seed(
                graph_dir,
                loader,
                edition_year,
                embed,
                embedding_profile,
                embedding_model,
                embedding_dimension,
                embedding_dtype,
                embedding_batch_size,
                max_batch_chars,
                max_batch_estimated_tokens,
                create_vector_index,
                embed_chunks,
                embed_provisions,
                embed_versions,
                resume_embeddings,
                chunk_file_policy,
                seed_batch_config,
            )
            .await?;
            Ok(())
        }
        Command::MaterializeNeo4j {
            graph_dir,
            neo4j_uri,
            neo4j_user,
            neo4j_password_env,
            edition_year,
            edge_batch_size,
            relationship_batch_size,
        } => {
            let pass = std::env::var(neo4j_password_env)?;
            let loader = neo4j_loader::Neo4jLoader::new(&neo4j_uri, &neo4j_user, &pass).await?;
            materialize_seed_relationships(
                &graph_dir,
                &loader,
                edition_year,
                edge_batch_size,
                relationship_batch_size,
            )
            .await?;
            Ok(())
        }
        Command::QcNeo4j {
            graph_dir,
            neo4j_uri,
            neo4j_user,
            neo4j_password_env,
            require_embeddings,
            embedding_profile,
            embedding_model,
            embedding_dim,
            embedding_dtype,
        } => {
            let pass = std::env::var(neo4j_password_env)?;
            let validator = qc_neo4j::QcNeo4jValidator::new(
                &neo4j_uri,
                &neo4j_user,
                &pass,
                require_embeddings,
                embedding_profile,
                embedding_model,
                embedding_dim,
                embedding_dtype,
                graph_dir,
            )
            .await?;
            let report = validator.run().await?;
            println!("\n📊 Neo4j QC Report: {:?}", report.status);
            Ok(())
        }
        Command::EmbedNeo4j {
            neo4j_uri,
            neo4j_user,
            neo4j_password_env,
            voyage_key,
            edition_year,
            smoke,
            resume,
            create_vector_indexes,
            phase,
            max_label_nodes,
            embedding_batch_size,
            scan_batch_size,
            max_batch_chars,
            max_batch_estimated_tokens,
        } => {
            let pass = std::env::var(&neo4j_password_env)
                .with_context(|| format!("missing Neo4j password env var {neo4j_password_env}"))?;
            let loader = neo4j_loader::Neo4jLoader::new(&neo4j_uri, &neo4j_user, &pass).await?;
            let voyage = VoyageClient::new(voyage_key, "voyage-4-large")?;
            let phases = parse_embedding_phases(&phase)?;
            let report = embeddings::run_neo4j_embeddings(
                &loader,
                &voyage,
                embeddings::EmbeddingRunConfig {
                    edition_year,
                    smoke,
                    resume,
                    max_label_nodes,
                    phases,
                    embedding_batch_size,
                    scan_batch_size,
                    max_batch_chars,
                    max_batch_estimated_tokens,
                    create_vector_indexes,
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Command::ParseCached {
            raw_dir,
            out,
            chapters,
            edition_year,
            fail_on_qc,
            append,
        } => {
            run_parse_cached(raw_dir, out, chapters, edition_year, fail_on_qc, append).await?;
            Ok(())
        }
        Command::ResolveCitations {
            graph_dir,
            edition_year,
        } => {
            run_resolve_citations(graph_dir, edition_year)?;
            Ok(())
        }
        Command::ClearNeo4j {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            neo4j_password_env,
            yes,
        } => {
            if !yes {
                return Err(anyhow!("Destructive operation: you MUST specify --yes to clear the database. Use with caution."));
            }
            let pass = match neo4j_password {
                Some(password) => password,
                None => std::env::var(&neo4j_password_env).with_context(|| {
                    format!("missing Neo4j password env var {neo4j_password_env}")
                })?,
            };
            let loader = neo4j_loader::Neo4jLoader::new(&neo4j_uri, &neo4j_user, &pass).await?;

            let (is_community, _, version) = loader.health_check().await?;
            info!(
                "Connected to Neo4j {} (Community: {})",
                version, is_community
            );

            loader.clear_database().await?;
            info!("Neo4j database cleared");
            Ok(())
        }
    }
}

// ── Parse cached orchestrator ─────────────────────────────────────────────────

async fn run_parse_cached(
    raw_dir: PathBuf,
    out: PathBuf,
    chapters: String,
    edition_year: i32,
    fail_on_qc: bool,
    append: bool,
) -> Result<()> {
    info!("═══ Parse Cached ═══");
    let chapters = parse_chapter_list(&chapters)?;
    let graph_dir = out.join("graph");

    if !append {
        // Clear old graph files for clean output
        let _ = fs::remove_dir_all(&graph_dir);
    }
    fs::create_dir_all(&graph_dir)?;

    let mut total_sections = 0;
    let mut total_provisions = 0;
    let mut total_citations = 0;
    let mut total_chunks = 0;
    let mut qc_failures = 0;

    for chapter in &chapters {
        let raw_path = raw_chapter_path(&raw_dir, chapter);

        if !raw_path.exists() {
            warn!("Cached file not found: {}", raw_path.display());
            continue;
        }

        let official_url = official_chapter_url(chapter);
        let html = read_raw_html(&raw_path)?;

        match parse_ors_chapter_html(&html, &official_url, chapter, edition_year) {
            Ok(parsed) => {
                let sec_count = parsed.versions.len();
                let prov_count = parsed.provisions.len();
                let cite_count = parsed.citations.len();
                let chunk_count = parsed.chunks.len();

                total_sections += sec_count;
                total_provisions += prov_count;
                total_citations += cite_count;
                total_chunks += chunk_count;

                append_jsonl(
                    &graph_dir.join("source_documents.jsonl"),
                    &[parsed.source_document.clone()],
                )?;
                append_jsonl(
                    &graph_dir.join("legal_text_identities.jsonl"),
                    &parsed.identities,
                )?;
                append_jsonl(
                    &graph_dir.join("legal_text_versions.jsonl"),
                    &parsed.versions,
                )?;
                append_jsonl(&graph_dir.join("provisions.jsonl"), &parsed.provisions)?;
                append_jsonl(
                    &graph_dir.join("citation_mentions.jsonl"),
                    &parsed.citations,
                )?;
                append_jsonl(&graph_dir.join("retrieval_chunks.jsonl"), &parsed.chunks)?;
                append_jsonl(&graph_dir.join("chapter_headings.jsonl"), &parsed.headings)?;
                append_jsonl(
                    &graph_dir.join("html_paragraphs.debug.jsonl"),
                    &parsed.html_paragraphs_debug,
                )?;
                append_jsonl(
                    &graph_dir.join("chapter_front_matter.jsonl"),
                    &parsed.chapter_front_matter,
                )?;
                append_jsonl(
                    &graph_dir.join("title_chapter_entries.jsonl"),
                    &parsed.title_chapter_entries,
                )?;
                append_jsonl(&graph_dir.join("source_notes.jsonl"), &parsed.source_notes)?;
                append_jsonl(
                    &graph_dir.join("chapter_toc_entries.jsonl"),
                    &parsed.chapter_toc_entries,
                )?;
                append_jsonl(
                    &graph_dir.join("reserved_ranges.jsonl"),
                    &parsed.reserved_ranges,
                )?;
                append_jsonl(
                    &graph_dir.join("parser_diagnostics.jsonl"),
                    &parsed.parser_diagnostic_rows,
                )?;
                append_derived_graph_nodes(&graph_dir, &parsed)?;

                let report = validate_outputs(
                    &parsed.versions,
                    &parsed.provisions,
                    &parsed.citations,
                    &parsed.chunks,
                );

                if report.is_blocking_failure() {
                    qc_failures += 1;
                    warn!("Chapter {} QC FAIL: {:?}", chapter, report.errors);
                } else {
                    info!(
                        "Parsed chapter {}: {} sections, {} provisions, {} citations, {} chunks",
                        chapter, sec_count, prov_count, cite_count, chunk_count
                    );
                }
            }
            Err(e) => {
                qc_failures += 1;
                warn!("Chapter {} parse failed: {}", chapter, e);
            }
        }
    }

    info!("═══ Parse Complete ═══");
    info!("Chapters parsed: {}", chapters.len());
    info!(
        "Total: {} sections, {} provisions, {} citations, {} chunks",
        total_sections, total_provisions, total_citations, total_chunks
    );
    info!("QC failures: {}", qc_failures);

    if fail_on_qc && qc_failures > 0 {
        return Err(anyhow!("QC failed for {} chapters", qc_failures));
    }

    Ok(())
}

// ── Resolve citations orchestrator ─────────────────────────────────────────────

fn run_resolve_citations(graph_dir: PathBuf, edition_year: i32) -> Result<()> {
    info!("═══ Resolve Citations ═══");

    // Build Global Symbol Table
    info!("Building global symbol table...");
    let table_result = build_global_symbol_table(&graph_dir, edition_year);

    let table = match table_result {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to build symbol table: {}", e);
            return Err(e);
        }
    };

    info!(
        "Symbol table: {} identities, {} versions, {} provisions",
        table.identities.len(),
        table.versions.len(),
        table.provisions.len()
    );

    // Resolve Citations
    info!("Resolving citations...");
    let citations_path = graph_dir.join("citation_mentions.jsonl");
    let mut citations: Vec<CitationMention> = if citations_path.exists() {
        io_jsonl::read_jsonl_strict(&citations_path)?
    } else {
        Vec::new()
    };

    if citations.is_empty() {
        info!("No citations to resolve");
        return Ok(());
    }

    let (edges, resolution_stats) = resolve_all_citations(&table, &mut citations, edition_year);

    info!("Resolution: {} total citations", resolution_stats.total);
    info!(
        "  Resolved: {} section, {} section+provision, {} chapter, {} range",
        resolution_stats.resolved_section,
        resolution_stats.resolved_section_and_provision,
        resolution_stats.resolved_chapter,
        resolution_stats.resolved_range
    );

    if resolution_stats.resolved_section_unresolved_subpath > 0 {
        info!(
            "  Warnings: {} resolved_section_unresolved_subpath",
            resolution_stats.resolved_section_unresolved_subpath
        );
    }

    if resolution_stats.unresolved_target_not_in_corpus > 0 {
        info!(
            "  Warnings: {} unresolved_target_not_in_corpus",
            resolution_stats.unresolved_target_not_in_corpus
        );
    }

    if resolution_stats.unresolved_malformed_citation > 0 {
        info!(
            "  Warnings: {} unresolved_malformed_citation",
            resolution_stats.unresolved_malformed_citation
        );
    }

    // Rewrite citations with resolver_status (atomic write for safety)
    write_jsonl_atomic(&citations_path, &citations)?;

    // Materialize CITES Edges
    info!("Materializing CITES edges...");
    let edges_path = graph_dir.join("cites_edges.jsonl");
    write_jsonl_atomic(&edges_path, &edges)?;
    info!("Created {} CITES edges", edges.len());

    // Citation QC
    info!("Running citation integrity checks...");
    let provision_ids: std::collections::HashSet<String> =
        table.provisions.keys().cloned().collect();
    let identity_ids: std::collections::HashSet<String> =
        table.identities.keys().cloned().collect();

    let mut citation_integrity_errors = 0;

    for citation in &citations {
        // Check source provision exists
        if !provision_ids.contains(&citation.source_provision_id) {
            citation_integrity_errors += 1;
            warn!(
                "Source provision not found: {} for citation {}",
                citation.source_provision_id, citation.citation_mention_id
            );
        }

        // If resolved, check target exists. Chapter citations resolve to
        // ChapterVersion nodes, not LegalTextIdentity nodes.
        if citation.resolver_status.starts_with("resolved") {
            if let Some(ref target_id) = citation.target_canonical_id {
                if !citation_target_exists(&table, &identity_ids, target_id, edition_year) {
                    citation_integrity_errors += 1;
                    warn!(
                        "Resolved target not found: {} for citation {}",
                        target_id, citation.citation_mention_id
                    );
                }
            }
        }
    }

    if citation_integrity_errors == 0 {
        info!("All citations passed integrity checks");
    } else {
        warn!("{} citation integrity errors", citation_integrity_errors);
    }

    info!("═══ Resolution Complete ═══");
    Ok(())
}

fn citation_target_exists(
    table: &resolver::GlobalSymbolTable,
    identity_ids: &std::collections::HashSet<String>,
    target_id: &str,
    edition_year: i32,
) -> bool {
    if identity_ids.contains(target_id) {
        return true;
    }

    let Some(chapter) = target_id.strip_prefix("or:ors:chapter:") else {
        return false;
    };

    let chapter_version_id = format!("or:ors:chapter:{}@{}", chapter, edition_year);
    table
        .chapter_versions
        .values()
        .any(|known_id| known_id == &chapter_version_id)
}

// ── Seeding orchestrator ─────────────────────────────────────────────────────

async fn run_seed(
    graph_dir: PathBuf,
    loader: neo4j_loader::Neo4jLoader,
    edition_year: i32,
    embed: bool,
    embedding_profile: String,
    embedding_model: String,
    embedding_dimension: i32,
    embedding_dtype: String,
    embedding_batch_size: usize,
    max_batch_chars: usize,
    max_batch_estimated_tokens: usize,
    create_vector_index: bool,
    embed_chunks: bool,
    embed_provisions: bool,
    embed_versions: bool,
    resume_embeddings: bool,
    chunk_file_policy: ChunkFilePolicy,
    seed_batch_config: neo4j_loader::SeedBatchConfig,
) -> Result<()> {
    info!("═══════════════════════════════════════════════════════════");
    info!("Starting optimized Neo4j seed from {}", graph_dir.display());
    info!("═══════════════════════════════════════════════════════════");

    // Resolve embedding profile
    let profile = ors_crawler_v0::embedding_profiles::get_embedding_profile(&embedding_profile)
        .unwrap_or_else(|| {
            warn!(
                "Unknown embedding profile '{}', using default",
                embedding_profile
            );
            ors_crawler_v0::embedding_profiles::default_chunk_profile()
        });

    // Emit warning if both profile and manual flags are provided
    if embedding_profile != profile.name
        || embedding_model != profile.model
        || embedding_dimension != profile.output_dimension
        || embedding_dtype != profile.output_dtype
    {
        warn!("Embedding profile '{}' differs from manual flags (model={}, dim={}, dtype={}). Profile will be used.",
              profile.name, embedding_model, embedding_dimension, embedding_dtype);
    }

    // Use profile values
    let embedding_model = profile.model.to_string();
    let embedding_dimension = profile.output_dimension;
    let embedding_dtype = profile.output_dtype.to_string();

    // Validate dtype for Neo4j
    if embedding_dtype != "float" {
        return Err(anyhow!(
            "Neo4j vector storage only supports 'float' dtype. Requested '{}'. For quantized vectors (int8, uint8, binary, ubinary), use a future external vector DB path.",
            embedding_dtype
        ));
    }

    info!(
        "Using embedding profile: {} (model={}, dim={}, dtype={})",
        profile.name, embedding_model, embedding_dimension, embedding_dtype
    );

    preflight_seed_graph_dir(&graph_dir)?;
    let counts = validate_seed_graph_contract(&graph_dir, chunk_file_policy)?;
    info!("Seed preflight parsed {} JSONL rows", counts.total_rows());
    info!(
        "Seed batch sizes: nodes={}, CITES edges={}, relationship transactions={}",
        seed_batch_config.node_batch_size,
        seed_batch_config.edge_batch_size,
        seed_batch_config.relationship_batch_size
    );

    // Health check and database info
    let (is_community, _, version) = loader.health_check().await?;
    info!("✓ Neo4j {version} (Community Edition: {})", is_community);

    loader
        .create_constraints(embedding_dimension, create_vector_index)
        .await?;
    info!("✓ Constraints and indexes created");

    if create_vector_index {
        loader.verify_vector_index(embedding_dimension).await?;
    }

    info!("═══ Phase 1: Loading Jurisdiction / Corpus Structure ═══");
    loader.load_jurisdictions().await?;
    loader.load_public_bodies().await?;
    loader.load_corpus().await?;
    loader.load_corpus_editions(edition_year).await?;

    // Phase 2: Load all node types from JSONL files
    info!("═══ Phase 2: Loading Source Documents ═══");
    let source_docs_path = graph_dir.join("source_documents.jsonl");
    if source_docs_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in read_jsonl_batches::<SourceDocument>(
            &source_docs_path,
            seed_batch_config.node_batch_size,
        )? {
            let docs = batch?;
            total += docs.len();
            loader
                .load_source_documents(docs, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Source Documents", total, start);
    }

    info!("═══ Phase 3: Loading Legal Text Identities ═══");
    let identities_path = graph_dir.join("legal_text_identities.jsonl");
    if identities_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in read_jsonl_batches::<LegalTextIdentity>(
            &identities_path,
            seed_batch_config.node_batch_size,
        )? {
            let identities = batch?;
            total += identities.len();
            loader
                .load_identities(identities, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Legal Text Identities", total, start);
    }

    info!("═══ Phase 4: Loading Legal Text Versions ═══");
    let versions_path = graph_dir.join("legal_text_versions.jsonl");
    if versions_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in read_jsonl_batches::<LegalTextVersion>(
            &versions_path,
            seed_batch_config.node_batch_size,
        )? {
            let mut versions = batch?;
            for v in &mut versions {
                let input_text = format!(
                    "Oregon Revised Statutes. {} Edition.\nCitation: {}\nTitle: {}\nStatus: {}\nText:\n{}",
                    edition_year,
                    v.citation,
                    v.title.as_deref().unwrap_or(""),
                    v.status,
                    v.text
                );
                v.embedding_input_hash = Some(calculate_embedding_input_hash(&input_text));
            }
            total += versions.len();
            loader
                .load_versions(versions, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Legal Text Versions", total, start);
    }

    let chapter_versions_start = Instant::now();
    info!("Creating chapter versions from loaded legal versions");
    loader.load_chapter_versions().await?;
    log_seed_phase_done("Chapter Versions", 0, chapter_versions_start);

    info!("═══ Phase 5: Loading Provisions ═══");
    let provisions_path = graph_dir.join("provisions.jsonl");
    if provisions_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in
            read_jsonl_batches::<Provision>(&provisions_path, seed_batch_config.node_batch_size)?
        {
            let mut provisions = batch?;
            for p in &mut provisions {
                let input_text = format!(
                    "Oregon Revised Statutes. {} Edition.\nCitation: {}\nProvision type: {}.\nStatus: active.\nText:\n{}",
                    edition_year,
                    p.display_citation,
                    p.provision_type,
                    p.text
                );
                p.embedding_input_hash = Some(calculate_embedding_input_hash(&input_text));
            }
            total += provisions.len();
            loader
                .load_provisions(provisions, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Provisions", total, start);
    }

    info!("═══ Phase 6: Loading Citation Mentions ═══");
    let citations_path = graph_dir.join("citation_mentions.jsonl");
    if citations_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in read_jsonl_batches::<CitationMention>(
            &citations_path,
            seed_batch_config.node_batch_size,
        )? {
            let citations = batch?;
            total += citations.len();
            loader
                .load_citation_mentions(citations, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Citation Mentions", total, start);
    }

    info!("═══ Phase 7: Loading Chapter Headings ═══");
    let headings_path = graph_dir.join("chapter_headings.jsonl");
    if headings_path.exists() {
        let start = Instant::now();
        let mut total = 0usize;
        for batch in
            read_jsonl_batches::<ChapterHeading>(&headings_path, seed_batch_config.node_batch_size)?
        {
            let headings = batch?;
            total += headings.len();
            loader
                .load_chapter_headings(headings, seed_batch_config.node_batch_size)
                .await?;
        }
        log_seed_phase_done("Chapter Headings", total, start);
    }

    info!("═══ Phase 8: Loading Retrieval Chunks ═══");
    let chunks_start = Instant::now();
    let mut chunks_total = 0usize;
    let chunk_files = find_chunk_files(&graph_dir, chunk_file_policy)?;
    info!("Found {} chunk files", chunk_files.len());
    for file in chunk_files {
        let mut file_total = 0usize;
        for batch in read_jsonl_batches::<RetrievalChunk>(&file, seed_batch_config.node_batch_size)?
        {
            let chunks = batch?;
            file_total += chunks.len();
            chunks_total += chunks.len();
            loader
                .load_chunks(chunks, seed_batch_config.node_batch_size)
                .await?;
        }
        info!("Loaded chunks from {}", file.display());
        log_seed_phase_done(
            &format!("Retrieval Chunks ({})", file.display()),
            file_total,
            chunks_start,
        );
    }
    log_seed_phase_done("Retrieval Chunks Total", chunks_total, chunks_start);

    macro_rules! load_optional_node_file {
        ($phase:literal, $file:literal, $ty:ty, $method:ident) => {{
            let path = graph_dir.join($file);
            if path.exists() {
                info!(concat!("═══ ", $phase, " ═══"));
                let start = Instant::now();
                let mut total = 0usize;
                for batch in read_jsonl_batches::<$ty>(&path, seed_batch_config.node_batch_size)? {
                    let rows = batch?;
                    total += rows.len();
                    loader
                        .$method(rows, seed_batch_config.node_batch_size)
                        .await?;
                }
                log_seed_phase_done($phase, total, start);
            }
        }};
    }

    load_optional_node_file!(
        "Phase 8a: Loading Status Events",
        "status_events.jsonl",
        StatusEvent,
        load_status_events
    );
    load_optional_node_file!(
        "Phase 8aa: Loading Source Notes",
        "source_notes.jsonl",
        SourceNote,
        load_source_notes
    );
    load_optional_node_file!(
        "Phase 8aaa: Loading HTML Paragraphs",
        "html_paragraphs.debug.jsonl",
        HtmlParagraph,
        load_html_paragraphs
    );
    load_optional_node_file!(
        "Phase 8aab: Loading Chapter Front Matter",
        "chapter_front_matter.jsonl",
        ChapterFrontMatter,
        load_chapter_front_matter
    );
    load_optional_node_file!(
        "Phase 8aac: Loading Title Chapter Entries",
        "title_chapter_entries.jsonl",
        TitleChapterEntry,
        load_title_chapter_entries
    );
    load_optional_node_file!(
        "Phase 8ab: Loading Chapter TOC Entries",
        "chapter_toc_entries.jsonl",
        ChapterTocEntry,
        load_chapter_toc_entries
    );
    load_optional_node_file!(
        "Phase 8ac: Loading Reserved Ranges",
        "reserved_ranges.jsonl",
        ReservedRange,
        load_reserved_ranges
    );
    load_optional_node_file!(
        "Phase 8ad: Loading Parser Diagnostics",
        "parser_diagnostics.jsonl",
        ParserDiagnostic,
        load_parser_diagnostics
    );
    load_optional_node_file!(
        "Phase 8ae: Loading Temporal Effects",
        "temporal_effects.jsonl",
        TemporalEffect,
        load_temporal_effects
    );
    load_optional_node_file!(
        "Phase 8af: Loading Lineage Events",
        "lineage_events.jsonl",
        LineageEvent,
        load_lineage_events
    );
    load_optional_node_file!(
        "Phase 8b: Loading Session Laws",
        "session_laws.jsonl",
        SessionLaw,
        load_session_laws
    );
    load_optional_node_file!(
        "Phase 8c: Loading Amendments",
        "amendments.jsonl",
        Amendment,
        load_amendments
    );
    load_optional_node_file!(
        "Phase 8d: Loading Time Intervals",
        "time_intervals.jsonl",
        TimeInterval,
        load_time_intervals
    );
    load_optional_node_file!(
        "Phase 8e: Loading Defined Terms",
        "defined_terms.jsonl",
        DefinedTerm,
        load_defined_terms
    );
    load_optional_node_file!(
        "Phase 8f: Loading Definition Scopes",
        "definition_scopes.jsonl",
        DefinitionScope,
        load_definition_scopes
    );
    load_optional_node_file!(
        "Phase 8g: Loading Definitions",
        "definitions.jsonl",
        Definition,
        load_definitions
    );
    load_optional_node_file!(
        "Phase 8h: Loading Legal Semantic Nodes",
        "legal_semantic_nodes.jsonl",
        LegalSemanticNode,
        load_legal_semantic_nodes
    );
    load_optional_node_file!(
        "Phase 8i: Loading Legal Actors",
        "legal_actors.jsonl",
        LegalActor,
        load_legal_actors
    );
    load_optional_node_file!(
        "Phase 8j: Loading Legal Actions",
        "legal_actions.jsonl",
        LegalAction,
        load_legal_actions
    );
    load_optional_node_file!(
        "Phase 8k: Loading Obligations",
        "obligations.jsonl",
        Obligation,
        load_obligations
    );
    load_optional_node_file!(
        "Phase 8l: Loading Exceptions",
        "exceptions.jsonl",
        Exception,
        load_exceptions
    );
    load_optional_node_file!(
        "Phase 8m: Loading Deadlines",
        "deadlines.jsonl",
        Deadline,
        load_deadlines
    );
    load_optional_node_file!(
        "Phase 8n: Loading Penalties",
        "penalties.jsonl",
        Penalty,
        load_penalties
    );
    load_optional_node_file!(
        "Phase 8o: Loading Remedies",
        "remedies.jsonl",
        Remedy,
        load_remedies
    );
    load_optional_node_file!(
        "Phase 8p: Loading Money Amounts",
        "money_amounts.jsonl",
        MoneyAmount,
        load_money_amounts
    );
    load_optional_node_file!(
        "Phase 8q: Loading Tax Rules",
        "tax_rules.jsonl",
        TaxRule,
        load_tax_rules
    );
    load_optional_node_file!(
        "Phase 8r: Loading Rate Limits",
        "rate_limits.jsonl",
        RateLimit,
        load_rate_limits
    );
    load_optional_node_file!(
        "Phase 8s: Loading Required Notices",
        "required_notices.jsonl",
        RequiredNotice,
        load_required_notices
    );
    load_optional_node_file!(
        "Phase 8t: Loading Form Texts",
        "form_texts.jsonl",
        FormText,
        load_form_texts
    );

    materialize_seed_relationships(
        &graph_dir,
        &loader,
        edition_year,
        seed_batch_config.edge_batch_size,
        seed_batch_config.relationship_batch_size,
    )
    .await?;

    // Warm up vector index after all data is loaded
    if create_vector_index {
        info!("Warming up vector index...");
        loader.warmup_vector_index().await?;
        info!("✓ Vector index ready");
    }

    if embed {
        let api_key = std::env::var("VOYAGE_API_KEY")?;
        let voyage = VoyageClient::new(api_key, &embedding_model)?;
        let voyage_config = model_config(&embedding_model).unwrap_or(&voyage::VOYAGE_4_LARGE);
        let context_token_limit = voyage_config.context_tokens;
        let batch_token_limit =
            max_batch_estimated_tokens.min(voyage_config.batch_token_safety_limit);

        if voyage_config.model != embedding_model {
            warn!(
                "Unknown Voyage model {}; using voyage-4-large limits for embedding batching",
                embedding_model
            );
        }

        if !voyage_config
            .allowed_dimensions
            .contains(&(embedding_dimension as usize))
        {
            return Err(anyhow!(
                "embedding dimension {} is not supported by {}. Allowed dimensions: {:?}",
                embedding_dimension,
                embedding_model,
                voyage_config.allowed_dimensions
            ));
        }

        let targets = [
            ("RetrievalChunk", "chunk_id", embed_chunks),
            ("Provision", "provision_id", embed_provisions),
            ("LegalTextVersion", "version_id", embed_versions),
        ];

        for (label, id_field, enabled) in targets {
            if !enabled {
                continue;
            }

            info!("═══ Embedding Surface: {} ═══", label);

            loop {
                let batch = loader
                    .get_embedding_targets(
                        label,
                        &embedding_model,
                        embedding_dimension,
                        embedding_batch_size * 2,
                        edition_year,
                        resume_embeddings,
                    )
                    .await?;

                if batch.is_empty() {
                    info!("All nodes of type {} already embedded", label);
                    break;
                }

                let mut safe_batch = Vec::new();
                let mut batch_chars = 0usize;
                let mut batch_tokens = 0usize;

                for (id, text, hash) in batch {
                    let chars = text.chars().count();
                    let tokens = estimate_tokens(&text, &embedding_model);

                    if tokens > context_token_limit {
                        warn!(
                            "Node {} ({}) exceeds {} token context ({} estimated tokens)",
                            label, id, context_token_limit, tokens
                        );

                        if label == "LegalTextVersion" {
                            // Update strategy to split_chunks_only
                            let q = "MATCH (v:LegalTextVersion {version_id: $id}) SET v.embedding_strategy = 'split_chunks_only'";
                            loader.run_query(query(q).param("id", id.clone())).await?;
                        }
                        continue;
                    }

                    if !safe_batch.is_empty()
                        && (safe_batch.len() >= embedding_batch_size
                            || batch_chars + chars > max_batch_chars
                            || batch_tokens + tokens > batch_token_limit)
                    {
                        break;
                    }

                    batch_chars += chars;
                    batch_tokens += tokens;
                    safe_batch.push((id, text, hash));
                }

                if safe_batch.is_empty() {
                    break; // Only over-length nodes left
                }

                info!(
                    "Embedding batch of {} {} nodes ({} chars, {} estimated tokens)",
                    safe_batch.len(),
                    label,
                    batch_chars,
                    batch_tokens
                );

                let texts: Vec<String> = safe_batch.iter().map(|(_, t, _)| t.clone()).collect();
                let response = voyage
                    .embed(
                        texts,
                        &embedding_model,
                        Some(embedding_dimension),
                        Some("document"),
                        Some(&embedding_dtype),
                    )
                    .await?;

                let mut updates = Vec::new();
                for (i, (id, _, hash)) in safe_batch.into_iter().enumerate() {
                    updates.push(neo4j_loader::EmbeddingUpdate {
                        chunk_id: id,
                        embedding: response.data[i].embedding.clone(),
                        embedding_model: embedding_model.clone(),
                        embedding_dim: embedding_dimension,
                        embedding_input_hash: hash,
                        embedding_profile: Some(profile.name.to_string()),
                        embedding_output_dtype: Some(embedding_dtype.clone()),
                        embedding_source_dimension: Some(embedding_dimension),
                    });
                }

                if label == "LegalTextVersion" {
                    // Update strategy to full_text for successfully embedded versions
                    let embedded_ids: Vec<String> =
                        updates.iter().map(|u| u.chunk_id.clone()).collect();
                    let q = "UNWIND $ids AS id MATCH (v:LegalTextVersion {version_id: id}) SET v.embedding_strategy = 'full_text'";
                    loader
                        .run_query(query(q).param("ids", embedded_ids))
                        .await?;
                }

                loader
                    .update_node_embeddings(label, id_field, updates)
                    .await?;
            }

            // Progress report for this surface
            let (total, embedded, pending, outdated) = loader
                .get_embedding_stats(label, &embedding_model, embedding_dimension)
                .await?;
            info!(
                "📊 {} embedding progress: {}/{} ({:.1}%) | Pending: {} | Outdated: {}",
                label,
                embedded,
                total,
                if total > 0 {
                    (embedded as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
                pending,
                outdated
            );
        }
    }

    info!("═══════════════════════════════════════════════════════════");
    info!("✅ Neo4j seed complete");
    info!("═══════════════════════════════════════════════════════════");
    Ok(())
}

async fn materialize_seed_relationships(
    graph_dir: &Path,
    loader: &neo4j_loader::Neo4jLoader,
    edition_year: i32,
    edge_batch_size: usize,
    relationship_batch_size: usize,
) -> Result<()> {
    info!("═══ Phase 9: Creating Graph Relationships ═══");
    let start_all = Instant::now();

    info!("Creating core structural relationships...");
    loader
        .materialize_identity_version_edges(relationship_batch_size)
        .await?;
    loader
        .materialize_version_provision_edges(relationship_batch_size)
        .await?;
    loader
        .materialize_structural_edges(edition_year, relationship_batch_size)
        .await?;
    loader
        .materialize_provision_hierarchy_edges(relationship_batch_size)
        .await?;

    info!("Creating leaf relationships concurrently...");
    tokio::try_join!(
        loader.materialize_citation_edges(relationship_batch_size),
        loader.materialize_chunk_edges(relationship_batch_size),
        loader.materialize_source_edges(relationship_batch_size),
        loader.materialize_semantic_edges(relationship_batch_size),
        loader.materialize_definition_edges(relationship_batch_size),
        loader.materialize_obligation_edges(relationship_batch_size),
        loader.materialize_history_edges(relationship_batch_size),
        loader.materialize_specialized_edges(relationship_batch_size),
    )?;

    info!("Enforcing current flags on LegalTextVersion nodes...");
    loader.enforce_current_flags().await?;

    log_seed_phase_done("All relationships materialized", 0, start_all);

    let edges_path = graph_dir.join("cites_edges.jsonl");
    if edges_path.exists() {
        info!("═══ Phase 10: Loading CITES Edges ═══");
        let start = Instant::now();
        let mut total = 0usize;
        for batch in read_jsonl_batches::<CitesEdge>(&edges_path, edge_batch_size)? {
            let edges = batch?;
            total += edges.len();
            loader.create_cites_edges(edges, edge_batch_size).await?;
        }
        log_seed_phase_done("CITES Edges", total, start);
    }

    Ok(())
}

fn log_seed_phase_done(phase: &str, rows: usize, start: Instant) {
    let elapsed = start.elapsed();
    if rows > 0 {
        let rows_per_sec = rows as f64 / elapsed.as_secs_f64().max(0.001);
        info!(
            "✓ {} complete: {} rows in {:.2}s ({:.0} rows/s)",
            phase,
            rows,
            elapsed.as_secs_f64(),
            rows_per_sec
        );
    } else {
        info!("✓ {} complete in {:.2}s", phase, elapsed.as_secs_f64());
    }
}

#[derive(Debug, Default)]
struct SeedGraphContractCounts {
    files: Vec<(String, usize)>,
}

impl SeedGraphContractCounts {
    fn add(&mut self, file: impl Into<String>, rows: usize) {
        self.files.push((file.into(), rows));
    }

    fn total_rows(&self) -> usize {
        self.files.iter().map(|(_, rows)| rows).sum()
    }
}

fn find_chunk_files(dir: &Path, policy: ChunkFilePolicy) -> Result<Vec<PathBuf>> {
    let root_file = dir.join("retrieval_chunks.jsonl");
    if matches!(policy, ChunkFilePolicy::RootOnly) {
        return Ok(if root_file.exists() {
            vec![root_file]
        } else {
            Vec::new()
        });
    }

    let mut files = Vec::new();
    collect_chunk_files_recursive(dir, &mut files)?;
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_chunk_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_chunk_files_recursive(&path, files)?;
        } else if path
            .file_name()
            .map_or(false, |n| n == "retrieval_chunks.jsonl")
        {
            files.push(path);
        }
    }
    Ok(())
}

fn preflight_seed_graph_dir(graph_dir: &Path) -> Result<()> {
    if !graph_dir.is_dir() {
        return Err(anyhow!("graph_dir does not exist: {}", graph_dir.display()));
    }

    for file in [
        "source_documents.jsonl",
        "legal_text_identities.jsonl",
        "legal_text_versions.jsonl",
        "provisions.jsonl",
        "citation_mentions.jsonl",
        "retrieval_chunks.jsonl",
    ] {
        let path = graph_dir.join(file);
        if !path.exists() {
            return Err(anyhow!(
                "required graph seed file is missing: {}",
                path.display()
            ));
        }
    }

    for file in ["chapter_headings.jsonl", "cites_edges.jsonl"] {
        let path = graph_dir.join(file);
        if !path.exists() {
            warn!(
                "optional graph topology file is missing: {}",
                path.display()
            );
        }
    }

    Ok(())
}

fn validate_seed_graph_contract(
    graph_dir: &Path,
    chunk_file_policy: ChunkFilePolicy,
) -> Result<SeedGraphContractCounts> {
    preflight_seed_graph_dir(graph_dir)?;

    let mut counts = SeedGraphContractCounts::default();
    macro_rules! count_required {
        ($file:literal, $ty:ty) => {{
            let path = graph_dir.join($file);
            let rows: Vec<$ty> = io_jsonl::read_jsonl_strict(&path)?;
            counts.add($file, rows.len());
        }};
    }
    macro_rules! count_optional {
        ($file:literal, $ty:ty) => {{
            let path = graph_dir.join($file);
            if path.exists() {
                let rows: Vec<$ty> = io_jsonl::read_jsonl_strict(&path)?;
                counts.add($file, rows.len());
            }
        }};
    }

    count_required!("source_documents.jsonl", SourceDocument);
    count_required!("legal_text_identities.jsonl", LegalTextIdentity);
    count_required!("legal_text_versions.jsonl", LegalTextVersion);
    count_required!("provisions.jsonl", Provision);
    count_required!("citation_mentions.jsonl", CitationMention);

    for chunk_file in find_chunk_files(graph_dir, chunk_file_policy)? {
        let rows: Vec<RetrievalChunk> = io_jsonl::read_jsonl_strict(&chunk_file)?;
        let label = chunk_file
            .strip_prefix(graph_dir)
            .unwrap_or(&chunk_file)
            .display()
            .to_string();
        counts.add(label, rows.len());
    }

    count_optional!("chapter_headings.jsonl", ChapterHeading);
    count_optional!("html_paragraphs.debug.jsonl", HtmlParagraph);
    count_optional!("chapter_front_matter.jsonl", ChapterFrontMatter);
    count_optional!("title_chapter_entries.jsonl", TitleChapterEntry);
    count_optional!("source_notes.jsonl", SourceNote);
    count_optional!("chapter_toc_entries.jsonl", ChapterTocEntry);
    count_optional!("reserved_ranges.jsonl", ReservedRange);
    count_optional!("parser_diagnostics.jsonl", ParserDiagnostic);
    count_optional!("cites_edges.jsonl", CitesEdge);
    count_optional!("status_events.jsonl", StatusEvent);
    count_optional!("temporal_effects.jsonl", TemporalEffect);
    count_optional!("lineage_events.jsonl", LineageEvent);
    count_optional!("session_laws.jsonl", SessionLaw);
    count_optional!("amendments.jsonl", Amendment);
    count_optional!("time_intervals.jsonl", TimeInterval);
    count_optional!("defined_terms.jsonl", DefinedTerm);
    count_optional!("definition_scopes.jsonl", DefinitionScope);
    count_optional!("definitions.jsonl", Definition);
    count_optional!("legal_semantic_nodes.jsonl", LegalSemanticNode);
    count_optional!("legal_actors.jsonl", LegalActor);
    count_optional!("legal_actions.jsonl", LegalAction);
    count_optional!("obligations.jsonl", Obligation);
    count_optional!("exceptions.jsonl", Exception);
    count_optional!("deadlines.jsonl", Deadline);
    count_optional!("penalties.jsonl", Penalty);
    count_optional!("remedies.jsonl", Remedy);
    count_optional!("money_amounts.jsonl", MoneyAmount);
    count_optional!("tax_rules.jsonl", TaxRule);
    count_optional!("rate_limits.jsonl", RateLimit);
    count_optional!("required_notices.jsonl", RequiredNotice);
    count_optional!("form_texts.jsonl", FormText);

    Ok(counts)
}

// ── Crawl orchestrator ─────────────────────────────────────────────────────────

async fn run_crawl(
    out: PathBuf,
    edition_year: i32,
    delay_ms: u64,
    max_chapters: usize,
    explicit_chapters: Option<String>,
    user_agent: String,
    fetch_only: bool,
    skip_citation_resolution: bool,
) -> Result<()> {
    let crawl_start = Instant::now();
    ensure_dirs(&out)?;

    let client = Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let mut stats = CrawlStats {
        started_at: Utc::now(),
        finished_at: None,
        duration_secs: None,
        chapters_discovered: 0,
        chapters_cached: 0,
        chapters_new_fetched: 0,
        chapters_failed: 0,
        total_raw_bytes: 0,
        sections_parsed: 0,
        provisions_parsed: 0,
        citations_extracted: 0,
        citations_resolved: 0,
        citations_unresolved: 0,
        chunks_created: 0,
        qc_warnings: 0,
        qc_failures: 0,
        citation_warnings: 0,
        citation_errors: 0,
        failed_chapters: Vec::new(),
    };

    // ── Discovery ──────────────────────────────────────────────────────────

    info!("═══ Phase 0: Discovery ═══");
    let chapters = if let Some(list) = explicit_chapters {
        let set = parse_chapter_list(&list)?;
        info!(
            "[discovery] using {} explicit chapters from --chapters",
            set.len()
        );
        set
    } else {
        discover_public_law_chapters(&client, delay_ms).await?
    };

    let mut chapters: Vec<String> = chapters.into_iter().collect();
    chapters.sort_by_key(|c| chapter_sort_key(c));
    if max_chapters > 0 && chapters.len() > max_chapters {
        chapters.truncate(max_chapters);
    }
    stats.chapters_discovered = chapters.len();
    info!("[discovery] proceeding with {} chapters", chapters.len());

    // ── Phase 1: Fetch all HTMLs ───────────────────────────────────────────

    info!("═══ Phase 1: Fetch ═══");
    let raw_dir = out.join("raw/official");
    let mut progress = Progress::new("fetch", chapters.len());

    for chapter in &chapters {
        let raw_path = raw_chapter_path(&raw_dir, chapter);

        if raw_path.exists() {
            let size = fs::metadata(&raw_path).map(|m| m.len()).unwrap_or(0);
            stats.chapters_cached += 1;
            stats.total_raw_bytes += size;
            progress.tick(
                &format!("ors{}.html", chapter_pad(chapter)),
                &format!("cached  {}KB", size / 1024),
            );
            continue;
        }

        let official_url = official_chapter_url(chapter);

        // Rate-limit before each new fetch
        sleep(Duration::from_millis(delay_ms)).await;

        match fetch_with_retry(&client, &official_url, 3).await {
            Ok(resp) => {
                fs::create_dir_all(raw_path.parent().unwrap())?;
                fs::write(&raw_path, &resp.bytes)?;
                let size = resp.bytes.len() as u64;
                stats.chapters_new_fetched += 1;
                stats.total_raw_bytes += size;
                progress.tick(
                    &format!("ors{}.html", chapter_pad(chapter)),
                    &format!("{}  {}KB", resp.status, size / 1024),
                );
            }
            Err(err) => {
                stats.chapters_failed += 1;
                stats.failed_chapters.push(chapter.clone());
                progress.tick_fail(
                    &format!("ors{}.html", chapter_pad(chapter)),
                    &err.to_string(),
                );
            }
        }
    }

    info!(
        "{}  ({} cached, {} new, {} failed, {:.1}MB total)",
        progress.summary(),
        stats.chapters_cached,
        stats.chapters_new_fetched,
        stats.chapters_failed,
        stats.total_raw_bytes as f64 / (1024.0 * 1024.0),
    );

    // ── Phase 2: Parse all HTMLs ───────────────────────────────────────────

    if fetch_only {
        info!("═══ Fetch-only mode: skipping parse phase ═══");
    } else {
        info!("═══ Phase 2: Parse ═══");

        let graph_dir = out.join("graph");
        // Clear old graph files for clean output
        let _ = fs::remove_dir_all(&graph_dir);
        fs::create_dir_all(&graph_dir)?;

        // Only parse chapters that have HTML on disk
        let parseable: Vec<&String> = chapters
            .iter()
            .filter(|ch| {
                let p = raw_chapter_path(&raw_dir, ch);
                p.exists()
            })
            .collect();

        let mut progress = Progress::new("parse", parseable.len());

        for chapter in &parseable {
            let raw_path = raw_chapter_path(&raw_dir, chapter);
            let official_url = official_chapter_url(chapter);

            let html = read_raw_html(&raw_path)?;

            match parse_ors_chapter_html(&html, &official_url, chapter, edition_year) {
                Ok(parsed) => {
                    let sec_count = parsed.versions.len();
                    let prov_count = parsed.provisions.len();
                    let cite_count = parsed.citations.len();
                    let chunk_count = parsed.chunks.len();

                    stats.sections_parsed += sec_count;
                    stats.provisions_parsed += prov_count;
                    stats.citations_extracted += cite_count;
                    stats.chunks_created += chunk_count;

                    append_jsonl(
                        &graph_dir.join("source_documents.jsonl"),
                        &[parsed.source_document.clone()],
                    )?;
                    append_jsonl(
                        &graph_dir.join("legal_text_identities.jsonl"),
                        &parsed.identities,
                    )?;
                    append_jsonl(
                        &graph_dir.join("legal_text_versions.jsonl"),
                        &parsed.versions,
                    )?;
                    append_jsonl(&graph_dir.join("provisions.jsonl"), &parsed.provisions)?;
                    append_jsonl(
                        &graph_dir.join("citation_mentions.jsonl"),
                        &parsed.citations,
                    )?;
                    append_jsonl(&graph_dir.join("retrieval_chunks.jsonl"), &parsed.chunks)?;
                    append_jsonl(&graph_dir.join("chapter_headings.jsonl"), &parsed.headings)?;
                    append_jsonl(
                        &graph_dir.join("html_paragraphs.debug.jsonl"),
                        &parsed.html_paragraphs_debug,
                    )?;
                    append_jsonl(
                        &graph_dir.join("chapter_front_matter.jsonl"),
                        &parsed.chapter_front_matter,
                    )?;
                    append_jsonl(
                        &graph_dir.join("title_chapter_entries.jsonl"),
                        &parsed.title_chapter_entries,
                    )?;
                    append_jsonl(&graph_dir.join("source_notes.jsonl"), &parsed.source_notes)?;
                    append_jsonl(
                        &graph_dir.join("chapter_toc_entries.jsonl"),
                        &parsed.chapter_toc_entries,
                    )?;
                    append_jsonl(
                        &graph_dir.join("reserved_ranges.jsonl"),
                        &parsed.reserved_ranges,
                    )?;
                    append_jsonl(
                        &graph_dir.join("parser_diagnostics.jsonl"),
                        &parsed.parser_diagnostic_rows,
                    )?;
                    append_derived_graph_nodes(&graph_dir, &parsed)?;

                    let report = validate_outputs(
                        &parsed.versions,
                        &parsed.provisions,
                        &parsed.citations,
                        &parsed.chunks,
                    );

                    if report.is_blocking_failure() {
                        stats.qc_failures += 1;
                        progress.tick(
                            &format!("ch {}", chapter),
                            &format!(
                                "{} secs  {} provs  QC FAIL: {:?}",
                                sec_count, prov_count, report.errors
                            ),
                        );
                    } else {
                        if !report.warnings.is_empty() {
                            stats.qc_warnings += 1;
                        }
                        progress.tick(
                            &format!("ch {}", chapter),
                            &format!(
                                "{} secs  {} provs  {} cites  {} chunks",
                                sec_count, prov_count, cite_count, chunk_count
                            ),
                        );
                    }
                }
                Err(e) => {
                    stats.qc_failures += 1;
                    progress.tick_fail(&format!("ch {}", chapter), &e.to_string());
                }
            }
        }

        info!("{}", progress.summary());
    }

    // ── Phase 3-6: Citation Resolution (unless skipped) ─────────────────────

    if fetch_only || skip_citation_resolution {
        if skip_citation_resolution {
            info!("═══ Phase 3-6: Skipping citation resolution (--skip-citation-resolution) ═══");
        }
    } else {
        // Phase 3: Build Global Symbol Tables
        info!("═══ Phase 3: Build Global Symbol Tables ═══");
        let graph_dir = out.join("graph");

        let table_result = build_global_symbol_table(&graph_dir, edition_year);

        if let Err(ref e) = table_result {
            warn!("[symbol-table] Failed to build: {}", e);
            stats.citation_errors += 1;
            info!("═══ Skipping citation resolution due to symbol table failure ═══");
        }

        if let Ok(ref table) = table_result {
            info!(
                "[symbol-table] {} identities, {} versions, {} provisions",
                table.identities.len(),
                table.versions.len(),
                table.provisions.len()
            );

            // Phase 4: Resolve Citations
            info!("═══ Phase 4: Resolve Citations ═══");
            let citations_path = graph_dir.join("citation_mentions.jsonl");
            let mut citations: Vec<CitationMention> = if citations_path.exists() {
                io_jsonl::read_jsonl_strict(&citations_path)?
            } else {
                Vec::new()
            };

            if citations.is_empty() {
                info!("[resolver] No citations to resolve");
            } else {
                let (edges, resolution_stats) =
                    resolve_all_citations(&table, &mut citations, edition_year);

                info!(
                "[resolver] {} citations: {} resolved_section, {} resolved_section_and_provision, {} resolved_chapter, {} resolved_range",
                resolution_stats.total,
                resolution_stats.resolved_section,
                resolution_stats.resolved_section_and_provision,
                resolution_stats.resolved_chapter,
                resolution_stats.resolved_range
            );

                if resolution_stats.resolved_section_unresolved_subpath > 0 {
                    info!(
                        "[resolver] {} resolved_section_unresolved_subpath (warnings)",
                        resolution_stats.resolved_section_unresolved_subpath
                    );
                }

                if resolution_stats.unresolved_target_not_in_corpus > 0 {
                    info!(
                        "[resolver] {} unresolved_target_not_in_corpus (warnings)",
                        resolution_stats.unresolved_target_not_in_corpus
                    );
                }

                stats.citations_resolved = resolution_stats.resolved_section
                    + resolution_stats.resolved_section_and_provision
                    + resolution_stats.resolved_chapter
                    + resolution_stats.resolved_range;
                stats.citations_unresolved = resolution_stats.unresolved_target_not_in_corpus
                    + resolution_stats.unresolved_malformed_citation
                    + resolution_stats.unsupported_citation_type;
                stats.citation_warnings = resolution_stats.warnings;
                stats.citation_errors = resolution_stats.errors;

                // Rewrite citations with resolver_status (atomic write for safety)
                write_jsonl_atomic(&citations_path, &citations)?;

                // Phase 5: Materialize CITES Edges
                info!("═══ Phase 5: Materialize CITES Edges ═══");
                let edges_path = graph_dir.join("cites_edges.jsonl");
                write_jsonl_atomic(&edges_path, &edges)?;
                info!("[edges] {} CITES edges materialized", edges.len());

                // Phase 6: Citation QC
                info!("═══ Phase 6: Citation QC ═══");
                let provision_ids: std::collections::HashSet<String> =
                    table.provisions.keys().cloned().collect();
                let _version_ids: std::collections::HashSet<String> =
                    table.versions.keys().cloned().collect();
                let identity_ids: std::collections::HashSet<String> =
                    table.identities.keys().cloned().collect();

                let mut citation_integrity_errors = 0;

                for citation in &citations {
                    // Check source provision exists
                    if !provision_ids.contains(&citation.source_provision_id) {
                        citation_integrity_errors += 1;
                        warn!(
                            "[citation-qc] Source provision not found: {} for citation {}",
                            citation.source_provision_id, citation.citation_mention_id
                        );
                    }

                    // If resolved, check target exists. Chapter citations resolve
                    // to ChapterVersion nodes, not LegalTextIdentity nodes.
                    if citation.resolver_status.starts_with("resolved") {
                        if let Some(ref target_id) = citation.target_canonical_id {
                            if !citation_target_exists(
                                table,
                                &identity_ids,
                                target_id,
                                edition_year,
                            ) {
                                citation_integrity_errors += 1;
                                warn!(
                                    "[citation-qc] Resolved target not found: {} for citation {}",
                                    target_id, citation.citation_mention_id
                                );
                            }
                        }
                    }
                }

                if citation_integrity_errors == 0 {
                    info!("[citation-qc] All citations passed integrity checks");
                } else {
                    warn!(
                        "[citation-qc] {} citation integrity errors",
                        citation_integrity_errors
                    );
                    stats.citation_errors += citation_integrity_errors;
                }
            }
        } // Close if let Ok(ref table)
    }

    // ── Final summary ──────────────────────────────────────────────────────

    let total_elapsed = crawl_start.elapsed().as_secs_f64();
    stats.finished_at = Some(Utc::now());
    stats.duration_secs = Some(total_elapsed);

    fs::write(
        out.join("stats.json"),
        serde_json::to_string_pretty(&stats)?,
    )?;

    info!("═══ Crawl Complete ═══");
    info!("Discovery:  {} chapters", stats.chapters_discovered);
    info!(
        "Fetch:      {} cached, {} new, {} failed  ({:.1}MB total)",
        stats.chapters_cached,
        stats.chapters_new_fetched,
        stats.chapters_failed,
        stats.total_raw_bytes as f64 / (1024.0 * 1024.0),
    );
    if !fetch_only {
        info!(
            "Parse:      {} sections, {} provisions, {} citations, {} chunks",
            stats.sections_parsed,
            stats.provisions_parsed,
            stats.citations_extracted,
            stats.chunks_created,
        );
        info!(
            "QC:         {} warnings, {} failures",
            stats.qc_warnings, stats.qc_failures,
        );
        if !skip_citation_resolution {
            info!(
                "Resolution: {} resolved, {} unresolved",
                stats.citations_resolved, stats.citations_unresolved,
            );
            if stats.citation_warnings > 0 || stats.citation_errors > 0 {
                info!(
                    "CitationQC: {} warnings, {} errors",
                    stats.citation_warnings, stats.citation_errors,
                );
            }
        }
    }
    if !stats.failed_chapters.is_empty() {
        warn!("Failed chapters: {:?}", stats.failed_chapters);
    }
    info!("Duration:   {}", format_duration(total_elapsed));

    Ok(())
}

// ── Discovery: 3-level drill via public.law ────────────────────────────────────

async fn discover_public_law_chapters(client: &Client, delay_ms: u64) -> Result<BTreeSet<String>> {
    // public.law uses relative hrefs: "ors_volume_1", "ors_title_1", "ors_chapter_5"
    // These regexes match both relative and absolute forms.
    let vol_re = regex::Regex::new(r"(?:^|/)ors_volume_(\d+)\b").unwrap();
    let title_re = regex::Regex::new(r"(?:^|/)ors_title_(\d+[A-Z]?)\b").unwrap();
    let chapter_re = regex::Regex::new(r"(?:^|/)ors_chapter_(\d+[A-Z]?)\b").unwrap();

    let base = "https://oregon.public.law";
    let root = "https://oregon.public.law/statutes";
    let sel = Selector::parse("a").unwrap();

    let mut all_chapters = BTreeSet::<String>::new();

    // Helper: turn a relative or absolute href into a full URL under /statutes/
    let make_url = |href: &str| -> String {
        if href.starts_with("http") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{}{}", base, href)
        } else {
            // bare slug like "ors_title_1"
            format!("{}/statutes/{}", base, href)
        }
    };

    // ── Step 1: root page → volume URLs ──────────────────────────────────
    info!("[discovery] fetching root: {}", root);
    let resp = fetch_bytes(client, root).await?;
    let html = String::from_utf8_lossy(&resp.bytes).to_string();
    let doc = Html::parse_document(&html);

    let mut volume_urls: Vec<String> = Vec::new();
    for el in doc.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            if vol_re.is_match(href) {
                let full = make_url(href);
                if !volume_urls.contains(&full) {
                    volume_urls.push(full);
                }
            }
            if let Some(c) = chapter_re.captures(href) {
                all_chapters.insert(c.get(1).unwrap().as_str().to_string());
            }
        }
    }
    info!(
        "[discovery] found {} volumes on root page",
        volume_urls.len()
    );

    // ── Step 2: each volume → title URLs ─────────────────────────────────
    let mut title_urls: Vec<String> = Vec::new();
    for (vi, vol_url) in volume_urls.iter().enumerate() {
        sleep(Duration::from_millis(delay_ms)).await;
        match fetch_bytes(client, vol_url).await {
            Ok(resp) => {
                let h = String::from_utf8_lossy(&resp.bytes).to_string();
                let d = Html::parse_document(&h);
                let mut vol_titles = 0;
                for el in d.select(&sel) {
                    if let Some(href) = el.value().attr("href") {
                        if title_re.is_match(href) {
                            let full = make_url(href);
                            if !title_urls.contains(&full) {
                                title_urls.push(full);
                                vol_titles += 1;
                            }
                        }
                        if let Some(c) = chapter_re.captures(href) {
                            all_chapters.insert(c.get(1).unwrap().as_str().to_string());
                        }
                    }
                }
                info!(
                    "[discovery] volume {}/{}: {} titles",
                    vi + 1,
                    volume_urls.len(),
                    vol_titles
                );
            }
            Err(e) => {
                warn!(
                    "[discovery] volume {}/{} failed: {}",
                    vi + 1,
                    volume_urls.len(),
                    e
                );
            }
        }
    }
    info!("[discovery] found {} titles total", title_urls.len());

    // ── Step 3: each title → chapter numbers ─────────────────────────────
    let mut progress = Progress::new("discover", title_urls.len());
    for title_url in &title_urls {
        sleep(Duration::from_millis(delay_ms)).await;
        match fetch_bytes(client, title_url).await {
            Ok(resp) => {
                let h = String::from_utf8_lossy(&resp.bytes).to_string();
                let d = Html::parse_document(&h);
                let mut new_chapters: Vec<String> = Vec::new();
                for el in d.select(&sel) {
                    if let Some(href) = el.value().attr("href") {
                        if let Some(c) = chapter_re.captures(href) {
                            let ch = c.get(1).unwrap().as_str().to_string();
                            if all_chapters.insert(ch.clone()) {
                                new_chapters.push(ch);
                            }
                        }
                    }
                }
                let title_name = title_url.rsplit('/').next().unwrap_or("?");
                progress.tick(
                    title_name,
                    &format!("{} chapters {:?}", new_chapters.len(), new_chapters),
                );
            }
            Err(e) => {
                let title_name = title_url.rsplit('/').next().unwrap_or("?");
                progress.tick_fail(title_name, &e.to_string());
            }
        }
    }

    info!(
        "[discovery] total: {} chapters discovered from {} volumes, {} titles",
        all_chapters.len(),
        volume_urls.len(),
        title_urls.len(),
    );

    if all_chapters.is_empty() {
        return Err(anyhow!(
            "Discovery found 0 chapters — check network or page structure"
        ));
    }

    Ok(all_chapters)
}

// ── HTTP fetching with retry ───────────────────────────────────────────────────

#[derive(Debug)]
struct FetchResponse {
    status: StatusCode,
    #[allow(dead_code)]
    content_type: Option<String>,
    bytes: Vec<u8>,
}

async fn fetch_bytes(client: &Client, url: &str) -> Result<FetchResponse> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("GET {url} failed with status {status}"));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = response.bytes().await?.to_vec();
    Ok(FetchResponse {
        status,
        content_type,
        bytes,
    })
}

async fn fetch_with_retry(client: &Client, url: &str, max_attempts: u32) -> Result<FetchResponse> {
    let mut last_err = anyhow!("no attempts made");
    for attempt in 1..=max_attempts {
        match fetch_bytes(client, url).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = e;
                if attempt < max_attempts {
                    let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                    warn!(
                        "[retry] attempt {}/{} failed for {}, retrying in {:?}",
                        attempt, max_attempts, url, backoff
                    );
                    sleep(backoff).await;
                }
            }
        }
    }
    Err(last_err.context(format!("all {} attempts failed for {}", max_attempts, url)))
}

// ── File I/O helpers ───────────────────────────────────────────────────────────

fn append_jsonl<T: Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    use std::io::Write;
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut writer = std::io::BufWriter::new(file);
    for row in rows {
        writer.write_all(serde_json::to_string(row)?.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn write_derived_graph_nodes(graph_dir: &Path, parsed: &models::ParsedChapter) -> Result<()> {
    let historical = derive_historical_nodes(&parsed.versions, &parsed.source_document);
    let note_semantics = derive_note_semantics(
        &parsed.source_notes,
        &parsed.source_document,
        parsed.edition_year,
    );
    let mut temporal_effects = note_semantics.temporal_effects;
    temporal_effects.extend(derive_provision_temporal_effects(&parsed.provisions));
    dedupe_temporal_effects(&mut temporal_effects);
    let mut status_events = historical.status_events;
    status_events.extend(derive_source_note_status_events(
        &parsed.source_notes,
        &parsed.source_document,
        parsed.edition_year,
    ));
    let semantic = derive_semantic_nodes(&parsed.provisions);
    let mut session_laws = note_semantics.session_laws;
    session_laws.extend(derive_session_laws_from_amendments(
        &parsed.amendments,
        &parsed.source_document,
    ));
    dedupe_session_laws(&mut session_laws);

    write_jsonl(graph_dir.join("status_events.jsonl"), &status_events)?;
    write_jsonl(graph_dir.join("temporal_effects.jsonl"), &temporal_effects)?;
    write_jsonl(
        graph_dir.join("lineage_events.jsonl"),
        &note_semantics.lineage_events,
    )?;
    write_jsonl(graph_dir.join("amendments.jsonl"), &parsed.amendments)?;
    write_jsonl(graph_dir.join("session_laws.jsonl"), &session_laws)?;
    write_jsonl(
        graph_dir.join("time_intervals.jsonl"),
        &note_semantics.time_intervals,
    )?;
    write_jsonl(
        graph_dir.join("defined_terms.jsonl"),
        &semantic.defined_terms,
    )?;
    write_jsonl(
        graph_dir.join("definition_scopes.jsonl"),
        &semantic.definition_scopes,
    )?;
    write_jsonl(graph_dir.join("definitions.jsonl"), &semantic.definitions)?;
    write_jsonl(
        graph_dir.join("legal_semantic_nodes.jsonl"),
        &semantic.legal_semantic_nodes,
    )?;
    write_jsonl(graph_dir.join("legal_actors.jsonl"), &semantic.legal_actors)?;
    write_jsonl(
        graph_dir.join("legal_actions.jsonl"),
        &semantic.legal_actions,
    )?;
    write_jsonl(graph_dir.join("obligations.jsonl"), &semantic.obligations)?;
    write_jsonl(graph_dir.join("exceptions.jsonl"), &semantic.exceptions)?;
    write_jsonl(graph_dir.join("deadlines.jsonl"), &semantic.deadlines)?;
    write_jsonl(graph_dir.join("penalties.jsonl"), &semantic.penalties)?;
    write_jsonl(graph_dir.join("remedies.jsonl"), &semantic.remedies)?;
    write_jsonl(
        graph_dir.join("money_amounts.jsonl"),
        &semantic.money_amounts,
    )?;
    write_jsonl(graph_dir.join("tax_rules.jsonl"), &semantic.tax_rules)?;
    write_jsonl(graph_dir.join("rate_limits.jsonl"), &semantic.rate_limits)?;
    write_jsonl(
        graph_dir.join("required_notices.jsonl"),
        &semantic.required_notices,
    )?;
    write_jsonl(graph_dir.join("form_texts.jsonl"), &semantic.form_texts)?;

    Ok(())
}

fn append_derived_graph_nodes(graph_dir: &Path, parsed: &models::ParsedChapter) -> Result<()> {
    let historical = derive_historical_nodes(&parsed.versions, &parsed.source_document);
    let note_semantics = derive_note_semantics(
        &parsed.source_notes,
        &parsed.source_document,
        parsed.edition_year,
    );
    let mut temporal_effects = note_semantics.temporal_effects;
    temporal_effects.extend(derive_provision_temporal_effects(&parsed.provisions));
    dedupe_temporal_effects(&mut temporal_effects);
    let mut status_events = historical.status_events;
    status_events.extend(derive_source_note_status_events(
        &parsed.source_notes,
        &parsed.source_document,
        parsed.edition_year,
    ));
    let semantic = derive_semantic_nodes(&parsed.provisions);
    let mut session_laws = note_semantics.session_laws;
    session_laws.extend(derive_session_laws_from_amendments(
        &parsed.amendments,
        &parsed.source_document,
    ));
    dedupe_session_laws(&mut session_laws);

    append_jsonl(&graph_dir.join("status_events.jsonl"), &status_events)?;
    append_jsonl(&graph_dir.join("temporal_effects.jsonl"), &temporal_effects)?;
    append_jsonl(
        &graph_dir.join("lineage_events.jsonl"),
        &note_semantics.lineage_events,
    )?;
    append_jsonl(&graph_dir.join("amendments.jsonl"), &parsed.amendments)?;
    append_jsonl(&graph_dir.join("session_laws.jsonl"), &session_laws)?;
    append_jsonl(
        &graph_dir.join("time_intervals.jsonl"),
        &note_semantics.time_intervals,
    )?;
    append_jsonl(
        &graph_dir.join("defined_terms.jsonl"),
        &semantic.defined_terms,
    )?;
    append_jsonl(
        &graph_dir.join("definition_scopes.jsonl"),
        &semantic.definition_scopes,
    )?;
    append_jsonl(&graph_dir.join("definitions.jsonl"), &semantic.definitions)?;
    append_jsonl(
        &graph_dir.join("legal_semantic_nodes.jsonl"),
        &semantic.legal_semantic_nodes,
    )?;
    append_jsonl(
        &graph_dir.join("legal_actors.jsonl"),
        &semantic.legal_actors,
    )?;
    append_jsonl(
        &graph_dir.join("legal_actions.jsonl"),
        &semantic.legal_actions,
    )?;
    append_jsonl(&graph_dir.join("obligations.jsonl"), &semantic.obligations)?;
    append_jsonl(&graph_dir.join("exceptions.jsonl"), &semantic.exceptions)?;
    append_jsonl(&graph_dir.join("deadlines.jsonl"), &semantic.deadlines)?;
    append_jsonl(&graph_dir.join("penalties.jsonl"), &semantic.penalties)?;
    append_jsonl(&graph_dir.join("remedies.jsonl"), &semantic.remedies)?;
    append_jsonl(
        &graph_dir.join("money_amounts.jsonl"),
        &semantic.money_amounts,
    )?;
    append_jsonl(&graph_dir.join("tax_rules.jsonl"), &semantic.tax_rules)?;
    append_jsonl(&graph_dir.join("rate_limits.jsonl"), &semantic.rate_limits)?;
    append_jsonl(
        &graph_dir.join("required_notices.jsonl"),
        &semantic.required_notices,
    )?;
    append_jsonl(&graph_dir.join("form_texts.jsonl"), &semantic.form_texts)?;
    // Ensure stub files exist for consistency with write_derived_graph_nodes
    append_jsonl(&graph_dir.join("amendments.jsonl"), &parsed.amendments)?;
    append_jsonl(&graph_dir.join("session_laws.jsonl"), &session_laws)?;
    append_jsonl(
        &graph_dir.join("time_intervals.jsonl"),
        &Vec::<TimeInterval>::new(),
    )?;

    Ok(())
}

fn dedupe_session_laws(rows: &mut Vec<SessionLaw>) {
    let mut seen = std::collections::HashSet::new();
    rows.retain(|row| seen.insert(row.session_law_id.clone()));
}

fn dedupe_temporal_effects(rows: &mut Vec<TemporalEffect>) {
    let mut seen = std::collections::HashSet::new();
    rows.retain(|row| seen.insert(row.temporal_effect_id.clone()));
}

fn ensure_dirs(out: &Path) -> Result<()> {
    fs::create_dir_all(out.join("raw/official"))?;
    fs::create_dir_all(out.join("normalized/chapters"))?;
    fs::create_dir_all(out.join("graph"))?;
    Ok(())
}

// ── URL and chapter helpers ────────────────────────────────────────────────────

fn parse_chapter_list(list: &str) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for item in list.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some((start, end)) = item.split_once('-') {
            let start: u32 = start.trim().parse()?;
            let end: u32 = end.trim().parse()?;
            if start > end {
                return Err(anyhow!("invalid chapter range: {item}"));
            }
            for chapter in start..=end {
                out.insert(chapter.to_string());
            }
        } else {
            let chapter: u32 = item.parse()?;
            out.insert(chapter.to_string());
        }
    }
    Ok(out)
}

fn official_chapter_url(chapter: &str) -> String {
    format!(
        "https://www.oregonlegislature.gov/bills_laws/ors/ors{}.html",
        chapter_pad(chapter)
    )
}

fn raw_chapter_path(raw_dir: &Path, chapter: &str) -> PathBuf {
    raw_dir.join(format!("ors{}.html", chapter_pad(chapter)))
}

fn read_raw_html(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    Ok(cow.to_string())
}

fn chapter_pad(chapter: &str) -> String {
    if chapter.chars().all(|c| c.is_ascii_digit()) {
        format!("{:03}", chapter.parse::<u32>().unwrap_or(0))
    } else {
        chapter.to_string()
    }
}

fn chapter_sort_key(chapter: &str) -> (u32, String) {
    let digits: String = chapter.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n = digits.parse::<u32>().unwrap_or(9999);
    (n, chapter.to_string())
}

fn calculate_embedding_input_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{find_chunk_files, ChunkFilePolicy};
    use std::fs;

    #[test]
    fn root_only_chunk_policy_ignores_nested_chunk_files() {
        let temp_dir =
            std::env::temp_dir().join(format!("orsgraph-chunks-{}", uuid::Uuid::new_v4()));
        let nested = temp_dir.join("nested/graph");
        fs::create_dir_all(&nested).unwrap();
        fs::write(temp_dir.join("retrieval_chunks.jsonl"), "{}\n").unwrap();
        fs::write(nested.join("retrieval_chunks.jsonl"), "{}\n").unwrap();

        let root_only = find_chunk_files(&temp_dir, ChunkFilePolicy::RootOnly).unwrap();
        let recursive = find_chunk_files(&temp_dir, ChunkFilePolicy::Recursive).unwrap();
        let _ = fs::remove_dir_all(temp_dir);

        assert_eq!(root_only.len(), 1);
        assert_eq!(recursive.len(), 2);
    }
}
