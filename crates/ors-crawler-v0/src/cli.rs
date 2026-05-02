use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ors-crawler")]
#[command(about = "ORSGraph registry crawler, import tools, graph QC, and Neo4j maintenance")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
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
    ParseUtcrPdf {
        #[arg(long)]
        input: PathBuf,

        #[arg(long)]
        out: PathBuf,

        #[arg(long, default_value_t = 2025)]
        edition_year: i32,

        #[arg(long, default_value = "2025-08-01")]
        effective_date: String,

        #[arg(
            long,
            default_value = "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf"
        )]
        source_url: String,

        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
    },
    ParseCourtRulesRegistry {
        #[arg(long)]
        input: PathBuf,

        #[arg(long)]
        out: PathBuf,

        #[arg(long, default_value = "Linn")]
        jurisdiction: String,

        #[arg(long, default_value = "2026-05-01")]
        snapshot_date: String,

        #[arg(
            long,
            default_value = "https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx"
        )]
        source_url: String,

        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
    },
    ParseLocalRulePdf {
        #[arg(long)]
        input: PathBuf,

        #[arg(long)]
        out: PathBuf,

        #[arg(long, default_value = "or:linn")]
        jurisdiction_id: String,

        #[arg(long, default_value = "Linn County")]
        jurisdiction_name: String,

        #[arg(long, default_value = "or:linn:circuit_court")]
        court_id: String,

        #[arg(long, default_value = "Linn County Circuit Court")]
        court_name: String,

        #[arg(long, default_value = "23rd Judicial District")]
        judicial_district: String,

        #[arg(long, default_value_t = 2026)]
        edition_year: i32,

        #[arg(long, default_value = "2026-02-01")]
        effective_date: String,

        #[arg(
            long,
            default_value = "https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx"
        )]
        source_url: String,

        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
    },
    #[command(name = "import-ors-legacy", alias = "crawl")]
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
    ValidateSourceRegistry {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        write_yaml: bool,
    },
    SourceIngest {
        #[arg(long)]
        source_id: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long, default_value = "data/sources")]
        out: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long, default_value = "all")]
        mode: String,
        #[arg(long)]
        fixture_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 2025)]
        edition_year: i32,
        #[arg(long)]
        chapters: Option<String>,
        #[arg(long)]
        session_key: Option<String>,
        #[arg(long, default_value_t = 0)]
        max_items: usize,
        #[arg(
            long,
            env = "ORS_CRAWLER_USER_AGENT",
            default_value = "NeighborOS-ORSGraph/0.1 registry crawler"
        )]
        user_agent: String,
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,
        #[arg(long, default_value_t = 3)]
        max_attempts: u32,
        #[arg(long, default_value_t = 2)]
        concurrency: usize,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        allow_network: bool,
        #[arg(long, default_value_t = false)]
        refresh: bool,
        #[arg(long, default_value_t = false)]
        fail_on_qc: bool,
    },
    CombineGraph {
        #[arg(long, default_value = "data/sources")]
        sources_dir: PathBuf,
        #[arg(long, default_value = "data/graph")]
        out: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        source_id: Option<String>,
        #[arg(long)]
        priority: Option<String>,
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
    #[command(name = "import-ors-cache", alias = "parse-cached")]
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
        #[arg(long, default_value_t = 100)]
        batch_size: usize,
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ChunkFilePolicy {
    RootOnly,
    Recursive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_subcommand_is_valid_for_railway_default_worker() {
        let cli = Cli::try_parse_from(["ors-crawler-v0"]).expect("parse cli");
        assert!(cli.command.is_none());
    }
}
