use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::neo4j::Neo4jService;
use crate::services::object_store::{
    ObjectStore, PutOptions, StoredObject, build_document_object_key, clean_etag, normalize_sha256,
};
use bytes::Bytes;
use flate2::read::DeflateDecoder;
use neo4rs::query;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;

mod ai_patch;
mod ast_diff;
mod ast_patch;
mod ast_validation;
mod authority;
mod citation_resolver;
mod complaints;
mod documents;
mod docx_renderer;
mod graph_projection;
mod html_renderer;
mod ids;
mod indexes;
mod markdown_adapter;
mod matters;
mod parsing;
mod pdf_renderer;
mod repository;
mod rule_engine;
mod storage;
mod support_linker;
mod timeline;
mod transcription;
mod work_product_ast;
mod work_products;

use ids::*;
use parsing::*;
use work_product_ast::{
    canonical_work_product_type, humanize_product_type, normalize_work_product_type_lossy,
    prosemirror_doc_for_text,
};

#[derive(Clone, Copy)]
struct AstStoragePolicy {
    entity_inline_bytes: u64,
    snapshot_inline_bytes: u64,
    block_inline_bytes: u64,
}

#[derive(Clone)]
pub struct CaseBuilderService {
    neo4j: Arc<Neo4jService>,
    object_store: Arc<dyn ObjectStore>,
    upload_ttl_seconds: u64,
    download_ttl_seconds: u64,
    max_upload_bytes: u64,
    ast_storage_policy: AstStoragePolicy,
    assemblyai: AssemblyAiProviderConfig,
    http_client: reqwest::Client,
}

#[derive(Clone)]
pub struct AssemblyAiProviderConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: String,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
    pub timeout_ms: u64,
    pub max_media_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
struct AssemblyAiTranscriptCreateRequest {
    audio_url: String,
    speech_models: Vec<String>,
    language_detection: bool,
    speaker_labels: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    speakers_expected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_options: Option<AssemblyAiSpeakerOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii_policies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii_sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii_return_unredacted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redact_pii_audio_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyterms_prompt: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remove_audio_tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_auth_header_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_auth_header_value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssemblyAiUploadResponse {
    upload_url: String,
}

const ASSEMBLYAI_CAPTION_CHARS_PER_CAPTION: u64 = 80;
const ASSEMBLYAI_REDACTED_AUDIO_QUALITY: &str = "mp3";
const ASSEMBLYAI_TRANSCRIPT_LIST_DEFAULT_LIMIT: u64 = 10;
const ASSEMBLYAI_TRANSCRIPT_LIST_MAX_LIMIT: u64 = 200;
const ASSEMBLYAI_WORD_SEARCH_MAX_TERMS: usize = 20;
const ASSEMBLYAI_WORD_SEARCH_MAX_WORDS_PER_TERM: usize = 5;
const ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL: usize = 1000;
const ASSEMBLYAI_KEYTERM_MAX_WORDS: usize = 6;
const ASSEMBLYAI_PROMPT_MAX_WORDS: usize = 1500;
const ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL: &str = "all";
const ASSEMBLYAI_PROMPT_PRESETS: &[&str] = &[
    "verbatim_multilingual",
    "unclear_masked",
    "unclear",
    "legal",
    "medical",
    "financial",
    "technical",
    "code_switching",
    "customer_support",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssemblyAiSubtitleFormat {
    Srt,
    Vtt,
}

impl AssemblyAiSubtitleFormat {
    fn as_str(self) -> &'static str {
        match self {
            AssemblyAiSubtitleFormat::Srt => "srt",
            AssemblyAiSubtitleFormat::Vtt => "vtt",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiTranscriptResponse {
    id: String,
    status: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    utterances: Vec<AssemblyAiUtterance>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    words: Vec<AssemblyAiWord>,
    #[serde(default)]
    unredacted_text: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    unredacted_utterances: Vec<AssemblyAiUtterance>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    unredacted_words: Vec<AssemblyAiWord>,
    #[serde(default)]
    language_code: Option<String>,
    #[serde(default)]
    audio_duration: Option<f64>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    redact_pii: Option<bool>,
    #[serde(default)]
    redact_pii_return_unredacted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiSentencesResponse {
    id: String,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    audio_duration: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    sentences: Vec<AssemblyAiSentence>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiParagraphsResponse {
    id: String,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    audio_duration: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    paragraphs: Vec<AssemblyAiParagraph>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiRedactedAudioResponse {
    status: String,
    redacted_audio_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiWordSearchResponse {
    id: String,
    total_count: u64,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    matches: Vec<AssemblyAiWordSearchMatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiWordSearchMatch {
    text: String,
    count: u64,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    timestamps: Vec<Vec<u64>>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    indexes: Vec<u64>,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiUtterance {
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    text: String,
    start: u64,
    end: u64,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiSentence {
    text: String,
    start: u64,
    end: u64,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    words: Vec<AssemblyAiWord>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    speaker: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiParagraph {
    text: String,
    start: u64,
    end: u64,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    words: Vec<AssemblyAiWord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssemblyAiWord {
    text: String,
    start: u64,
    end: u64,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    channel: Option<String>,
}

#[derive(Clone)]
pub struct BinaryUploadRequest {
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: Bytes,
    pub document_type: Option<String>,
    pub folder: Option<String>,
    pub confidentiality: Option<String>,
    pub relative_path: Option<String>,
    pub upload_batch_id: Option<String>,
}

#[derive(Clone)]
pub struct CaseBuilderServiceConfig {
    pub neo4j: Arc<Neo4jService>,
    pub object_store: Arc<dyn ObjectStore>,
    pub upload_ttl_seconds: u64,
    pub download_ttl_seconds: u64,
    pub max_upload_bytes: u64,
    pub ast_entity_inline_bytes: u64,
    pub ast_snapshot_inline_bytes: u64,
    pub ast_block_inline_bytes: u64,
    pub assemblyai: AssemblyAiProviderConfig,
}

#[derive(Clone, Copy)]
struct NodeSpec {
    label: &'static str,
    id_key: &'static str,
    edge: &'static str,
}

#[derive(Clone)]
struct DocumentProvenance {
    object_blob: ObjectBlob,
    document_version: DocumentVersion,
    ingestion_run: IngestionRun,
}

#[derive(Clone)]
struct SourceContext {
    document_version_id: Option<String>,
    object_blob_id: Option<String>,
    ingestion_run_id: Option<String>,
}

#[derive(Clone)]
struct SentenceCandidate {
    text: String,
    byte_start: u64,
    byte_end: u64,
    char_start: u64,
    char_end: u64,
}

#[derive(Clone)]
struct DateCandidate {
    iso_date: String,
    date_text: String,
    confidence: f32,
    byte_start: u64,
    byte_end: u64,
    char_start: u64,
    char_end: u64,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct ZipPackage {
    entries: Vec<ZipEntryRecord>,
    central_directory_offset: usize,
}

#[derive(Clone)]
struct ZipEntryRecord {
    name: String,
    version_made_by: u16,
    version_needed: u16,
    flags: u16,
    compression: u16,
    last_modified_time: u16,
    last_modified_date: u16,
    crc32: u32,
    compressed_size: usize,
    uncompressed_size: usize,
    internal_attrs: u16,
    external_attrs: u32,
    local_header_offset: usize,
}

#[derive(Clone)]
struct ZipCentralRecord {
    name: String,
    version_made_by: u16,
    version_needed: u16,
    flags: u16,
    compression: u16,
    last_modified_time: u16,
    last_modified_date: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    internal_attrs: u16,
    external_attrs: u32,
    local_header_offset: u32,
}

#[derive(Clone)]
struct VersionChangeInput {
    target_type: String,
    target_id: String,
    operation: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    summary: String,
    legal_impact: LegalImpactSummary,
    ai_audit_id: Option<String>,
}

struct WorkProductHashes {
    document_hash: String,
    support_graph_hash: String,
    qc_state_hash: String,
    formatting_hash: String,
}

const PARSER_REGISTRY_VERSION: &str = "casebuilder-parser-registry-v1";
const CHUNKER_VERSION: &str = "casebuilder-line-chunker-v1";
const CITATION_RESOLVER_VERSION: &str = "casebuilder-citation-resolver-v1";
const CASE_INDEX_VERSION: &str = "casebuilder-case-graph-index-v1";
const ORS_2025_SOURCE_URL: &str = "https://www.oregonlegislature.gov/bills_laws/pages/ors.aspx";
const ORCP_2025_SOURCE_URL: &str = "https://www.oregonlegislature.gov/bills_laws/Pages/ORCP.aspx";
const UTCR_CURRENT_SOURCE_URL: &str = "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf";

static PLEADING_PARAGRAPH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*(?:\*\*)?(?:¶\s*)?([0-9]+[A-Za-z]?)\.\s*(?:\*\*)?\s*(.+?)\s*$"#).unwrap()
});
static CLAIM_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bCLAIM\s+FOR\s+RELIEF\b").unwrap());
static ORS_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+(?:chapters?\s+)?[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\s*(?:to|through)\s*[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?)?(?:\([^)]+\))*",
    )
    .unwrap()
});
static ORCP_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bORCP\s+[0-9]+[A-Z]?(?:\s*[A-Z])?(?:\([^)]+\))*").unwrap());
static UTCR_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bUTCR\s+[0-9]+(?:\.[0-9]+)?(?:\([^)]+\))*").unwrap());
static SESSION_LAW_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Or(?:egon)?\.?\s+Laws?|Oregon Laws)\s+([0-9]{4}),?\s+ch(?:apter)?\.?\s*([0-9]+)(?:,?\s*(?:§|sec(?:tion)?\.?)\s*([0-9A-Za-z.-]+))?",
    )
    .unwrap()
});
static EXHIBIT_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:PX|EXHIBIT)\s*[- ]?([A-Z]{1,3}|[0-9]{1,4})\b").unwrap());
static ISO_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(20[0-9]{2})-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])\b").unwrap());
static MONTH_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(January|February|March|April|May|June|July|August|September|October|November|December|Jan\.?|Feb\.?|Mar\.?|Apr\.?|Jun\.?|Jul\.?|Aug\.?|Sep\.?|Sept\.?|Oct\.?|Nov\.?|Dec\.?)\s+([0-9]{1,2})(?:st|nd|rd|th)?[,]?\s+(20[0-9]{2})\b",
    )
    .unwrap()
});
static NUMERIC_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(0?[1-9]|1[0-2])/([0-2]?[0-9]|3[01])/(20[0-9]{2})\b").unwrap());

#[derive(Clone)]
struct ParserOutcome {
    parser_id: String,
    status: String,
    message: String,
    text: Option<String>,
}

#[derive(Clone)]
struct ParsedComplaintParagraph {
    original_label: String,
    text: String,
    section_key: String,
    count_key: Option<String>,
    byte_start: u64,
    byte_end: u64,
    char_start: u64,
    char_end: u64,
}

#[derive(Default)]
struct ParsedComplaintStructure {
    sections: Vec<(String, String, String)>,
    counts: Vec<(String, String)>,
    paragraphs: Vec<ParsedComplaintParagraph>,
}

impl CaseBuilderService {
    pub fn new(config: CaseBuilderServiceConfig) -> Self {
        Self {
            neo4j: config.neo4j,
            object_store: config.object_store,
            upload_ttl_seconds: config.upload_ttl_seconds,
            download_ttl_seconds: config.download_ttl_seconds,
            max_upload_bytes: config.max_upload_bytes,
            ast_storage_policy: AstStoragePolicy {
                entity_inline_bytes: config.ast_entity_inline_bytes,
                snapshot_inline_bytes: config.ast_snapshot_inline_bytes,
                block_inline_bytes: config.ast_block_inline_bytes,
            },
            assemblyai: config.assemblyai,
            http_client: reqwest::Client::new(),
        }
    }
}

fn party_spec() -> NodeSpec {
    NodeSpec {
        label: "Party",
        id_key: "party_id",
        edge: "HAS_PARTY",
    }
}
fn document_spec() -> NodeSpec {
    NodeSpec {
        label: "CaseDocument",
        id_key: "document_id",
        edge: "HAS_DOCUMENT",
    }
}
fn fact_spec() -> NodeSpec {
    NodeSpec {
        label: "Fact",
        id_key: "fact_id",
        edge: "HAS_FACT",
    }
}
fn timeline_spec() -> NodeSpec {
    NodeSpec {
        label: "TimelineEvent",
        id_key: "event_id",
        edge: "HAS_EVENT",
    }
}
fn timeline_suggestion_spec() -> NodeSpec {
    NodeSpec {
        label: "TimelineSuggestion",
        id_key: "suggestion_id",
        edge: "HAS_TIMELINE_SUGGESTION",
    }
}
fn timeline_agent_run_spec() -> NodeSpec {
    NodeSpec {
        label: "TimelineAgentRun",
        id_key: "agent_run_id",
        edge: "HAS_TIMELINE_AGENT_RUN",
    }
}
fn evidence_spec() -> NodeSpec {
    NodeSpec {
        label: "Evidence",
        id_key: "evidence_id",
        edge: "HAS_EVIDENCE",
    }
}
fn claim_spec() -> NodeSpec {
    NodeSpec {
        label: "Claim",
        id_key: "claim_id",
        edge: "HAS_CLAIM",
    }
}
fn defense_spec() -> NodeSpec {
    NodeSpec {
        label: "Defense",
        id_key: "defense_id",
        edge: "HAS_DEFENSE",
    }
}
fn deadline_spec() -> NodeSpec {
    NodeSpec {
        label: "DeadlineInstance",
        id_key: "deadline_id",
        edge: "HAS_DEADLINE",
    }
}
fn task_spec() -> NodeSpec {
    NodeSpec {
        label: "Task",
        id_key: "task_id",
        edge: "HAS_TASK",
    }
}
fn draft_spec() -> NodeSpec {
    NodeSpec {
        label: "Draft",
        id_key: "draft_id",
        edge: "HAS_DRAFT",
    }
}
fn fact_check_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "FactCheckFinding",
        id_key: "finding_id",
        edge: "HAS_FACT_CHECK_FINDING",
    }
}
fn citation_check_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "CitationCheckFinding",
        id_key: "finding_id",
        edge: "HAS_CITATION_CHECK_FINDING",
    }
}
fn document_version_spec() -> NodeSpec {
    NodeSpec {
        label: "DocumentVersion",
        id_key: "document_version_id",
        edge: "HAS_DOCUMENT_VERSION",
    }
}
fn ingestion_run_spec() -> NodeSpec {
    NodeSpec {
        label: "IngestionRun",
        id_key: "ingestion_run_id",
        edge: "HAS_INGESTION_RUN",
    }
}
fn source_span_spec() -> NodeSpec {
    NodeSpec {
        label: "SourceSpan",
        id_key: "source_span_id",
        edge: "HAS_SOURCE_SPAN",
    }
}
fn index_run_spec() -> NodeSpec {
    NodeSpec {
        label: "IndexRun",
        id_key: "index_run_id",
        edge: "HAS_INDEX_RUN",
    }
}
fn page_spec() -> NodeSpec {
    NodeSpec {
        label: "Page",
        id_key: "page_id",
        edge: "HAS_PAGE",
    }
}
fn text_chunk_spec() -> NodeSpec {
    NodeSpec {
        label: "TextChunk",
        id_key: "text_chunk_id",
        edge: "HAS_TEXT_CHUNK",
    }
}
fn evidence_span_spec() -> NodeSpec {
    NodeSpec {
        label: "EvidenceSpan",
        id_key: "evidence_span_id",
        edge: "HAS_EVIDENCE_SPAN",
    }
}
fn entity_mention_spec() -> NodeSpec {
    NodeSpec {
        label: "EntityMention",
        id_key: "entity_mention_id",
        edge: "HAS_ENTITY_MENTION",
    }
}
fn search_index_record_spec() -> NodeSpec {
    NodeSpec {
        label: "SearchIndexRecord",
        id_key: "search_index_record_id",
        edge: "HAS_SEARCH_INDEX_RECORD",
    }
}
fn extraction_artifact_manifest_spec() -> NodeSpec {
    NodeSpec {
        label: "ExtractionArtifactManifest",
        id_key: "manifest_id",
        edge: "HAS_EXTRACTION_MANIFEST",
    }
}
fn document_annotation_spec() -> NodeSpec {
    NodeSpec {
        label: "DocumentAnnotation",
        id_key: "annotation_id",
        edge: "HAS_DOCUMENT_ANNOTATION",
    }
}
fn transcription_job_spec() -> NodeSpec {
    NodeSpec {
        label: "TranscriptionJob",
        id_key: "transcription_job_id",
        edge: "HAS_TRANSCRIPTION_JOB",
    }
}
fn transcript_segment_spec() -> NodeSpec {
    NodeSpec {
        label: "TranscriptSegment",
        id_key: "segment_id",
        edge: "HAS_TRANSCRIPT_SEGMENT",
    }
}
fn transcript_speaker_spec() -> NodeSpec {
    NodeSpec {
        label: "TranscriptSpeaker",
        id_key: "speaker_id",
        edge: "HAS_TRANSCRIPT_SPEAKER",
    }
}
fn transcript_review_change_spec() -> NodeSpec {
    NodeSpec {
        label: "TranscriptReviewChange",
        id_key: "review_change_id",
        edge: "HAS_TRANSCRIPT_REVIEW_CHANGE",
    }
}
fn complaint_spec() -> NodeSpec {
    NodeSpec {
        label: "ComplaintDraft",
        id_key: "complaint_id",
        edge: "HAS_COMPLAINT",
    }
}
fn complaint_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "RuleCheckFinding",
        id_key: "finding_id",
        edge: "HAS_RULE_CHECK_FINDING",
    }
}
fn complaint_artifact_spec() -> NodeSpec {
    NodeSpec {
        label: "ExportArtifact",
        id_key: "artifact_id",
        edge: "HAS_EXPORT_ARTIFACT",
    }
}
fn work_product_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProduct",
        id_key: "work_product_id",
        edge: "HAS_WORK_PRODUCT",
    }
}
fn work_product_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProductFinding",
        id_key: "finding_id",
        edge: "HAS_WORK_PRODUCT_FINDING",
    }
}
fn work_product_artifact_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProductArtifact",
        id_key: "artifact_id",
        edge: "HAS_WORK_PRODUCT_ARTIFACT",
    }
}
fn change_set_spec() -> NodeSpec {
    NodeSpec {
        label: "ChangeSet",
        id_key: "change_set_id",
        edge: "HAS_CHANGE_SET",
    }
}
fn version_snapshot_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionSnapshot",
        id_key: "snapshot_id",
        edge: "HAS_VERSION_SNAPSHOT",
    }
}
fn snapshot_manifest_spec() -> NodeSpec {
    NodeSpec {
        label: "SnapshotManifest",
        id_key: "manifest_id",
        edge: "HAS_SNAPSHOT_MANIFEST",
    }
}
fn snapshot_entity_state_spec() -> NodeSpec {
    NodeSpec {
        label: "SnapshotEntityState",
        id_key: "entity_state_id",
        edge: "HAS_SNAPSHOT_ENTITY_STATE",
    }
}
fn version_change_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionChange",
        id_key: "change_id",
        edge: "HAS_VERSION_CHANGE",
    }
}
fn version_branch_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionBranch",
        id_key: "branch_id",
        edge: "HAS_VERSION_BRANCH",
    }
}
fn ai_edit_audit_spec() -> NodeSpec {
    NodeSpec {
        label: "AIEditAudit",
        id_key: "ai_audit_id",
        edge: "HAS_AI_EDIT_AUDIT",
    }
}

fn matter_reference_error(error: ApiError, target_type: &str) -> ApiError {
    match error {
        ApiError::NotFound(_) => {
            ApiError::NotFound(format!("Matter-owned {target_type} reference not found"))
        }
        other => other,
    }
}

fn normalize_work_product_type(value: &str) -> ApiResult<String> {
    canonical_work_product_type(value)
        .map(str::to_string)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported work product type {value}")))
}

fn default_work_product_from_matter(
    matter: &MatterSummary,
    work_product_id: &str,
    title: &str,
    product_type: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
    now: &str,
) -> WorkProduct {
    let blocks = match product_type {
        "complaint" => complaint_work_product_blocks(matter, work_product_id, facts, claims),
        "motion" => motion_blocks(matter, work_product_id, facts, claims),
        "answer" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "responses",
                    "Responses to allegations",
                    "Build the admit, deny, and lack-knowledge responses from the allegation grid.",
                ),
                (
                    "affirmative_defenses",
                    "Affirmative defenses",
                    "Add defenses, supporting facts, evidence, and authority.",
                ),
                (
                    "counterclaims",
                    "Counterclaims",
                    "Add any counterclaims or mark this section intentionally omitted.",
                ),
                ("prayer", "Prayer for relief", "State the requested relief."),
            ],
        ),
        "declaration" | "affidavit" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "declarant",
                    "Declarant identity",
                    "Identify the declarant and basis of personal knowledge.",
                ),
                (
                    "facts",
                    "Declaration facts",
                    "Add numbered factual statements supported by evidence.",
                ),
                (
                    "signature",
                    "Signature",
                    "Add date, place, signature, and review required language.",
                ),
            ],
        ),
        "memo" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "question",
                    "Question presented",
                    "State the legal question.",
                ),
                (
                    "brief_answer",
                    "Brief answer",
                    "Give the short answer with caveats.",
                ),
                ("facts", "Relevant facts", "Link facts and evidence."),
                ("analysis", "Analysis", "Add source-backed legal analysis."),
                (
                    "conclusion",
                    "Conclusion",
                    "State the recommended next step.",
                ),
            ],
        ),
        "notice" | "letter" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "recipient",
                    "Recipient",
                    "Identify the recipient and delivery context.",
                ),
                ("purpose", "Purpose", "State the notice or letter purpose."),
                ("body", "Body", "Draft the operative text."),
                (
                    "signature",
                    "Signature",
                    "Add sender signature and contact details.",
                ),
            ],
        ),
        "proposed_order" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "caption",
                    "Caption",
                    "Confirm court, parties, and case number.",
                ),
                ("findings", "Findings", "Add any findings or recitals."),
                ("order", "Order", "State the ordered relief."),
                ("signature", "Judge signature", "Reserve signature block."),
            ],
        ),
        "exhibit_list" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "exhibits",
                    "Exhibits",
                    "List exhibits with stable labels and source documents.",
                ),
                (
                    "foundation",
                    "Foundation notes",
                    "Track authentication and admissibility notes for each exhibit.",
                ),
            ],
        ),
        _ => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "summary",
                    "Summary",
                    "Describe the purpose of this work product.",
                ),
                (
                    "facts",
                    "Relevant facts",
                    "Link facts and evidence before relying on this text.",
                ),
                (
                    "analysis",
                    "Analysis",
                    "Add source-backed legal analysis or argument.",
                ),
                (
                    "conclusion",
                    "Conclusion",
                    "Add requested next step or relief.",
                ),
            ],
        ),
    };
    let mut product = WorkProduct {
        id: work_product_id.to_string(),
        work_product_id: work_product_id.to_string(),
        matter_id: matter.matter_id.clone(),
        title: title.to_string(),
        product_type: product_type.to_string(),
        status: "draft".to_string(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "guided_setup".to_string(),
        source_draft_id: None,
        source_complaint_id: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        profile: work_product_profile(product_type),
        document_ast: WorkProductDocument::default(),
        blocks,
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: Vec::new(),
        artifacts: Vec::new(),
        history: vec![work_product_event(
            &matter.matter_id,
            work_product_id,
            "work_product_created",
            "work_product",
            work_product_id,
            "Shared WorkProduct AST created.",
        )],
        ai_commands: default_work_product_ai_commands(product_type),
        formatting_profile: default_work_product_formatting_profile(product_type),
        rule_pack: work_product_rule_pack(product_type),
    };
    apply_matter_rule_profile(
        &mut product.rule_pack,
        matter,
        now.get(0..10).unwrap_or(now),
        product_type,
    );
    refresh_work_product_state(&mut product);
    product
}

fn complaint_work_product_blocks(
    matter: &MatterSummary,
    work_product_id: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
) -> Vec<WorkProductBlock> {
    let fact_text = if facts.is_empty() {
        "Add numbered, evidence-supported factual allegations.".to_string()
    } else {
        facts
            .iter()
            .take(8)
            .map(|fact| fact.statement.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let count_text = if claims.is_empty() {
        "Add each claim for relief with elements, supporting facts, evidence, and authority."
            .to_string()
    } else {
        claims
            .iter()
            .filter(|claim| claim.kind != "defense")
            .take(4)
            .map(|claim| format!("{}: {}", claim.title, claim.legal_theory))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let rows = [
        (
            "caption",
            "Caption",
            format!("{} · {}", matter.court, matter.name),
        ),
        (
            "jurisdiction_venue",
            "Jurisdiction and venue",
            format!(
                "Jurisdiction and venue are alleged in {}.",
                matter.jurisdiction
            ),
        ),
        ("factual_paragraph", "Factual allegations", fact_text),
        ("count", "Claims for relief", count_text),
        (
            "prayer_for_relief",
            "Prayer for relief",
            "Plaintiff requests relief according to proof after human review.".to_string(),
        ),
        (
            "signature_block",
            "Signature block",
            "Complete signature, contact, and certification information before filing.".to_string(),
        ),
    ];
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            work_product_id: work_product_id.to_string(),
            block_type: if *role == "caption" {
                "caption".to_string()
            } else {
                "section".to_string()
            },
            role: role.to_string(),
            title: title.to_string(),
            text: text.clone(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: if *role == "factual_paragraph" {
                facts
                    .iter()
                    .take(8)
                    .map(|fact| fact.fact_id.clone())
                    .collect()
            } else {
                Vec::new()
            },
            evidence_ids: Vec::new(),
            authorities: if *role == "count" {
                claims
                    .iter()
                    .filter(|claim| claim.kind != "defense")
                    .flat_map(|claim| claim.authorities.clone())
                    .collect()
            } else {
                Vec::new()
            },
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn motion_blocks(
    matter: &MatterSummary,
    work_product_id: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
) -> Vec<WorkProductBlock> {
    let fact_text = if facts.is_empty() {
        "Add record-supported facts before relying on the motion.".to_string()
    } else {
        facts
            .iter()
            .take(6)
            .map(|fact| fact.statement.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let argument_text = if claims.is_empty() {
        "Add the controlling legal standard, authority, and argument for the order requested."
            .to_string()
    } else {
        claims
            .iter()
            .take(4)
            .map(|claim| format!("{}: {}", claim.title, claim.legal_theory))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let rows = [
        (
            "notice_motion",
            "Notice and motion",
            format!(
                "Movant will ask {} for an order granting the relief requested below.",
                matter.court
            ),
        ),
        (
            "relief_requested",
            "Relief requested",
            "State the exact order or relief sought with particularity.".to_string(),
        ),
        ("factual_background", "Factual background", fact_text),
        (
            "legal_standard",
            "Legal standard",
            "Add controlling authority and explain the applicable standard.".to_string(),
        ),
        ("argument", "Argument", argument_text),
        (
            "conclusion",
            "Conclusion",
            "For these reasons, the motion should be granted after human review.".to_string(),
        ),
        (
            "proposed_order",
            "Proposed order placeholder",
            "Prepare or attach a proposed order if required by the court or local practice."
                .to_string(),
        ),
    ];
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            work_product_id: work_product_id.to_string(),
            block_type: if *role == "notice_motion" {
                "heading".to_string()
            } else {
                "section".to_string()
            },
            role: role.to_string(),
            title: title.to_string(),
            text: text.clone(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: if *role == "factual_background" {
                facts
                    .iter()
                    .take(6)
                    .map(|fact| fact.fact_id.clone())
                    .collect()
            } else {
                Vec::new()
            },
            evidence_ids: Vec::new(),
            authorities: if *role == "argument" {
                claims
                    .iter()
                    .flat_map(|claim| claim.authorities.clone())
                    .collect()
            } else {
                Vec::new()
            },
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn profile_blocks(
    matter_id: &str,
    work_product_id: &str,
    rows: &[(&str, &str, &str)],
) -> Vec<WorkProductBlock> {
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_type: "section".to_string(),
            role: role.to_string(),
            title: title.to_string(),
            text: text.to_string(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: Vec::new(),
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn work_product_profile(product_type: &str) -> WorkProductProfile {
    let (name, required, optional) = match product_type {
        "complaint" => (
            "Structured Complaint",
            vec![
                "caption",
                "jurisdiction",
                "facts",
                "count",
                "relief",
                "signature",
            ],
            vec!["certificate", "exhibits"],
        ),
        "motion" => (
            "Oregon Civil Motion",
            vec![
                "notice_motion",
                "relief_requested",
                "legal_standard",
                "argument",
                "conclusion",
            ],
            vec![
                "factual_background",
                "conferral_certificate",
                "proposed_order",
            ],
        ),
        "answer" => (
            "Structured Answer",
            vec!["responses", "affirmative_defenses", "prayer"],
            vec!["counterclaims"],
        ),
        "declaration" | "affidavit" => (
            if product_type == "affidavit" {
                "Affidavit"
            } else {
                "Declaration"
            },
            vec!["declarant", "facts", "signature"],
            vec!["exhibits"],
        ),
        "memo" => (
            "Legal Memo",
            vec![
                "question",
                "brief_answer",
                "facts",
                "analysis",
                "conclusion",
            ],
            vec!["authorities", "exhibits"],
        ),
        "notice" => (
            "Notice",
            vec!["recipient", "purpose", "body", "signature"],
            vec!["certificate", "exhibits"],
        ),
        "letter" => (
            "Letter",
            vec!["recipient", "purpose", "body", "signature"],
            vec!["enclosures", "exhibits"],
        ),
        "proposed_order" => (
            "Proposed Order",
            vec!["caption", "findings", "order", "signature"],
            vec!["service"],
        ),
        "exhibit_list" => ("Exhibit List", vec!["exhibits"], vec!["foundation"]),
        _ => (
            "Structured Work Product",
            vec!["summary", "facts", "analysis", "conclusion"],
            vec!["exhibits", "certificate"],
        ),
    };
    WorkProductProfile {
        profile_id: format!("work-product-{product_type}-v1"),
        product_type: product_type.to_string(),
        name: name.to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        route_slug: product_type.replace('_', "-"),
        required_block_roles: required.into_iter().map(str::to_string).collect(),
        optional_block_roles: optional.into_iter().map(str::to_string).collect(),
        supports_rich_text: true,
    }
}

fn default_work_product_formatting_profile(product_type: &str) -> FormattingProfile {
    let mut profile = default_formatting_profile();
    profile.profile_id = format!("oregon-circuit-civil-{product_type}");
    profile.name = format!(
        "Oregon Circuit Civil {}",
        humanize_product_type(product_type)
    );
    profile
}

fn work_product_rule_pack(product_type: &str) -> RulePack {
    if product_type == "motion" {
        return oregon_civil_motion_rule_pack();
    }
    if product_type == "complaint" {
        return oregon_civil_complaint_rule_pack();
    }
    RulePack {
        rule_pack_id: format!("oregon-circuit-civil-{product_type}-baseline"),
        name: format!(
            "Oregon Circuit Civil {} - baseline",
            humanize_product_type(product_type)
        ),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2025-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![rule_definition(
            "work-product-review-needed",
            "Human review",
            "internal://casebuilder/rules/work-product-review-needed",
            "warning",
            "work_product",
            "review",
            "Work product requires human review.",
            "Generated or structured work product is not legal advice and is not filing-ready.",
            "Review the full document, authorities, evidence links, and formatting before use.",
            false,
        )],
    }
}

fn default_rule_profile_summary() -> RuleProfileSummary {
    RuleProfileSummary {
        jurisdiction_id: "or:state".to_string(),
        court_id: None,
        court: None,
        filing_date: None,
        utcr_edition_id: Some("or:utcr@2025".to_string()),
        slr_edition_id: None,
        active_statewide_order_ids: Vec::new(),
        active_local_order_ids: Vec::new(),
        active_out_of_cycle_amendment_ids: Vec::new(),
        currentness_warnings: vec![
            "Resolve filing-date rule context through /api/v1/rules/applicable before filing or export."
                .to_string(),
        ],
        resolver_endpoint:
            "/api/v1/rules/applicable?jurisdiction=Linn&date=YYYY-MM-DD&type=complaint"
                .to_string(),
    }
}

fn apply_matter_rule_profile(
    rule_pack: &mut RulePack,
    matter: &MatterSummary,
    filing_date: &str,
    product_type: &str,
) {
    let jurisdiction_id = casebuilder_jurisdiction_id(matter);
    let court_id = casebuilder_court_id(matter, &jurisdiction_id);
    rule_pack.rule_profile.jurisdiction_id = jurisdiction_id.clone();
    rule_pack.rule_profile.court_id = court_id;
    rule_pack.rule_profile.court = if matter.court.is_empty() {
        None
    } else {
        Some(matter.court.clone())
    };
    rule_pack.rule_profile.filing_date = Some(filing_date.to_string());
    rule_pack.rule_profile.resolver_endpoint = format!(
        "/api/v1/rules/applicable?jurisdiction={}&date={}&type={}",
        jurisdiction_id, filing_date, product_type
    );
}

fn casebuilder_jurisdiction_id(matter: &MatterSummary) -> String {
    let jurisdiction = matter.jurisdiction.trim();
    if jurisdiction.is_empty() || jurisdiction.eq_ignore_ascii_case("oregon") {
        return county_from_court(&matter.court)
            .map(|county| format!("or:{}", slug_casebuilder_id(&county)))
            .unwrap_or_else(|| "or:state".to_string());
    }
    let normalized = jurisdiction.to_ascii_lowercase();
    if normalized.starts_with("or:") {
        return normalized;
    }
    if normalized == "statewide" {
        return "or:state".to_string();
    }
    let county_source = if normalized.ends_with(" county") {
        normalized.trim_end_matches(" county").trim().to_string()
    } else if let Some(county) = county_from_court(&matter.court) {
        county
    } else {
        normalized
    };
    format!("or:{}", slug_casebuilder_id(&county_source))
}

fn casebuilder_court_id(matter: &MatterSummary, jurisdiction_id: &str) -> Option<String> {
    if jurisdiction_id == "or:state" {
        return None;
    }
    let court = matter.court.trim().to_ascii_lowercase();
    if court.starts_with("or:") {
        Some(court)
    } else if court.contains("circuit court") || court.is_empty() {
        Some(format!("{jurisdiction_id}:circuit_court"))
    } else {
        Some(format!("{jurisdiction_id}:{}", slug_casebuilder_id(&court)))
    }
}

fn county_from_court(court: &str) -> Option<String> {
    let lower = court.to_ascii_lowercase();
    lower
        .split(" county")
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != lower)
        .map(str::to_string)
}

fn slug_casebuilder_id(value: &str) -> String {
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

fn build_case_graph(matter: &MatterBundle) -> CaseGraphResponse {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = HashSet::new();
    let mut seen_edges = HashSet::new();
    let base = format!("/casebuilder/matters/{}", matter.summary.matter_id);

    add_case_graph_node(
        &mut nodes,
        &mut seen_nodes,
        CaseGraphNode {
            id: matter.summary.matter_id.clone(),
            kind: "matter".to_string(),
            label: matter.summary.name.clone(),
            subtitle: Some(matter.summary.court.clone()),
            status: Some(matter.summary.status.clone()),
            risk: None,
            href: Some(base.clone()),
            metadata: graph_metadata([
                ("jurisdiction", matter.summary.jurisdiction.clone()),
                (
                    "case_number",
                    matter
                        .summary
                        .case_number
                        .clone()
                        .unwrap_or_else(|| "unassigned".to_string()),
                ),
            ]),
        },
    );

    for party in &matter.parties {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: party.party_id.clone(),
                kind: "party".to_string(),
                label: party.name.clone(),
                subtitle: Some(format!("{} / {}", party.role, party.party_type)),
                status: None,
                risk: None,
                href: Some(format!("{base}/parties")),
                metadata: graph_metadata([
                    ("role", party.role.clone()),
                    ("party_type", party.party_type.clone()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &party.party_id,
            "has_party",
            "has party",
            None,
        );
    }

    for document in &matter.documents {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: document.document_id.clone(),
                kind: "document".to_string(),
                label: document.title.clone(),
                subtitle: Some(document.filename.clone()),
                status: Some(document.processing_status.clone()),
                risk: document
                    .contradictions_flagged
                    .gt(&0)
                    .then(|| "contradiction".to_string()),
                href: Some(format!("{base}/documents/{}", document.document_id)),
                metadata: graph_metadata([
                    ("document_type", document.document_type.clone()),
                    ("folder", document.folder.clone()),
                    ("storage_status", document.storage_status.clone()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &document.document_id,
            "has_document",
            "has document",
            Some(document.processing_status.as_str()),
        );
    }

    for fact in &matter.facts {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: fact.fact_id.clone(),
                kind: "fact".to_string(),
                label: truncate_graph_label(&fact.statement),
                subtitle: fact.date.clone(),
                status: Some(fact.status.clone()),
                risk: if fact.needs_verification {
                    Some("needs_verification".to_string())
                } else {
                    None
                },
                href: Some(format!("{base}/facts#{}", fact.fact_id)),
                metadata: graph_metadata([
                    ("confidence", format!("{:.0}%", fact.confidence * 100.0)),
                    ("sources", fact.source_document_ids.len().to_string()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &fact.fact_id,
            "has_fact",
            "has fact",
            Some(fact.status.as_str()),
        );
        for document_id in &fact.source_document_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                document_id,
                &fact.fact_id,
                "supports_fact",
                "supports fact",
                Some(fact.status.as_str()),
            );
        }
        for evidence_id in &fact.source_evidence_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                evidence_id,
                &fact.fact_id,
                "supports_fact",
                "supports fact",
                Some(fact.status.as_str()),
            );
        }
        for evidence_id in &fact.contradicted_by_evidence_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                evidence_id,
                &fact.fact_id,
                "contradicts_fact",
                "contradicts",
                Some("open"),
            );
        }
    }

    for evidence in &matter.evidence {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: evidence.evidence_id.clone(),
                kind: "evidence".to_string(),
                label: truncate_graph_label(&evidence.quote),
                subtitle: Some(evidence.evidence_type.clone()),
                status: Some(evidence.strength.clone()),
                risk: (!evidence.contradicts_fact_ids.is_empty())
                    .then(|| "contradiction".to_string()),
                href: Some(format!("{base}/evidence")),
                metadata: graph_metadata([
                    ("strength", evidence.strength.clone()),
                    ("confidence", format!("{:.0}%", evidence.confidence * 100.0)),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &evidence.evidence_id,
            "has_evidence",
            "has evidence",
            Some(evidence.strength.as_str()),
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &evidence.document_id,
            &evidence.evidence_id,
            "derived_from",
            "derived from",
            None,
        );
        for fact_id in &evidence.supports_fact_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                &evidence.evidence_id,
                fact_id,
                "supports_fact",
                "supports",
                Some("support"),
            );
        }
        for fact_id in &evidence.contradicts_fact_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                &evidence.evidence_id,
                fact_id,
                "contradicts_fact",
                "contradicts",
                Some("open"),
            );
        }
    }

    for claim in &matter.claims {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: claim.claim_id.clone(),
                kind: claim.kind.clone(),
                label: claim.title.clone(),
                subtitle: Some(claim.claim_type.clone()),
                status: Some(claim.status.clone()),
                risk: Some(claim.risk_level.clone()),
                href: Some(format!("{base}/claims#{}", claim.claim_id)),
                metadata: graph_metadata([
                    ("elements", claim.elements.len().to_string()),
                    ("authorities", claim.authorities.len().to_string()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &claim.claim_id,
            "has_claim",
            "has claim",
            Some(claim.status.as_str()),
        );
        for fact_id in &claim.fact_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                fact_id,
                &claim.claim_id,
                "supports_claim",
                "supports claim",
                None,
            );
        }
        for evidence_id in &claim.evidence_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                evidence_id,
                &claim.claim_id,
                "supports_claim",
                "supports claim",
                None,
            );
        }
        for authority in &claim.authorities {
            add_authority_node_and_edge(
                &mut nodes,
                &mut edges,
                &mut seen_nodes,
                &mut seen_edges,
                &claim.claim_id,
                authority,
            );
        }
        for element in &claim.elements {
            add_case_graph_node(
                &mut nodes,
                &mut seen_nodes,
                CaseGraphNode {
                    id: element.element_id.clone(),
                    kind: "element".to_string(),
                    label: truncate_graph_label(&element.text),
                    subtitle: Some(claim.title.clone()),
                    status: Some(if element.satisfied {
                        "supported".to_string()
                    } else {
                        "missing".to_string()
                    }),
                    risk: (!element.satisfied).then(|| "gap".to_string()),
                    href: Some(format!("{base}/claims#{}", claim.claim_id)),
                    metadata: graph_metadata([
                        ("facts", element.fact_ids.len().to_string()),
                        ("evidence", element.evidence_ids.len().to_string()),
                        ("authorities", element.authorities.len().to_string()),
                    ]),
                },
            );
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                &claim.claim_id,
                &element.element_id,
                "has_element",
                "has element",
                if element.satisfied {
                    Some("supported")
                } else {
                    Some("missing")
                },
            );
            for fact_id in &element.fact_ids {
                add_case_graph_edge(
                    &mut edges,
                    &mut seen_edges,
                    fact_id,
                    &element.element_id,
                    "satisfies_element",
                    "satisfies",
                    None,
                );
            }
            for evidence_id in &element.evidence_ids {
                add_case_graph_edge(
                    &mut edges,
                    &mut seen_edges,
                    evidence_id,
                    &element.element_id,
                    "supports_element",
                    "supports",
                    None,
                );
            }
            for authority in &element.authorities {
                add_authority_node_and_edge(
                    &mut nodes,
                    &mut edges,
                    &mut seen_nodes,
                    &mut seen_edges,
                    &element.element_id,
                    authority,
                );
            }
        }
    }

    for event in &matter.timeline {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: event.event_id.clone(),
                kind: "event".to_string(),
                label: event.title.clone(),
                subtitle: Some(event.date.clone()),
                status: Some(event.status.clone()),
                risk: event.disputed.then(|| "disputed".to_string()),
                href: Some(format!("{base}/timeline")),
                metadata: graph_metadata([
                    ("kind", event.kind.clone()),
                    (
                        "date_confidence",
                        format!("{:.0}%", event.date_confidence * 100.0),
                    ),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &event.event_id,
            "has_event",
            "has event",
            Some(event.status.as_str()),
        );
        if let Some(document_id) = &event.source_document_id {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                document_id,
                &event.event_id,
                "documents_event",
                "documents",
                None,
            );
        }
        for fact_id in &event.linked_fact_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                fact_id,
                &event.event_id,
                "supports_event",
                "supports event",
                None,
            );
        }
    }

    for suggestion in &matter.timeline_suggestions {
        if suggestion.status == "rejected" {
            continue;
        }
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: suggestion.suggestion_id.clone(),
                kind: "timeline_suggestion".to_string(),
                label: suggestion.title.clone(),
                subtitle: Some(suggestion.date.clone()),
                status: Some(suggestion.status.clone()),
                risk: (!suggestion.warnings.is_empty()).then(|| "needs_review".to_string()),
                href: Some(format!("{base}/timeline#{}", suggestion.suggestion_id)),
                metadata: graph_metadata([
                    ("kind", suggestion.kind.clone()),
                    ("source_type", suggestion.source_type.clone()),
                    (
                        "date_confidence",
                        format!("{:.0}%", suggestion.date_confidence * 100.0),
                    ),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &suggestion.suggestion_id,
            "has_timeline_suggestion",
            "has suggestion",
            Some(suggestion.status.as_str()),
        );
        if let Some(document_id) = &suggestion.source_document_id {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                document_id,
                &suggestion.suggestion_id,
                "proposes_timeline",
                "proposes",
                Some(suggestion.status.as_str()),
            );
        }
        for fact_id in &suggestion.linked_fact_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                fact_id,
                &suggestion.suggestion_id,
                "proposes_timeline",
                "proposes",
                Some(suggestion.status.as_str()),
            );
        }
    }

    for deadline in &matter.deadlines {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: deadline.deadline_id.clone(),
                kind: "deadline".to_string(),
                label: deadline.title.clone(),
                subtitle: Some(deadline.due_date.clone()),
                status: Some(deadline.status.clone()),
                risk: Some(deadline.severity.clone()),
                href: Some(format!("{base}/deadlines#{}", deadline.deadline_id)),
                metadata: graph_metadata([
                    ("kind", deadline.kind.clone()),
                    ("source", deadline.source.clone()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &deadline.deadline_id,
            "has_deadline",
            "has deadline",
            Some(deadline.status.as_str()),
        );
        if let Some(event_id) = &deadline.triggered_by_event_id {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                event_id,
                &deadline.deadline_id,
                "triggers_deadline",
                "triggers",
                None,
            );
        }
    }

    for task in &matter.tasks {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: task.task_id.clone(),
                kind: "task".to_string(),
                label: task.title.clone(),
                subtitle: task.due_date.clone(),
                status: Some(task.status.clone()),
                risk: Some(task.priority.clone()),
                href: Some(format!("{base}/tasks")),
                metadata: graph_metadata([
                    ("source", task.source.clone()),
                    ("priority", task.priority.clone()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &task.task_id,
            "has_task",
            "has task",
            Some(task.status.as_str()),
        );
        if let Some(deadline_id) = &task.related_deadline_id {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                deadline_id,
                &task.task_id,
                "drives_task",
                "drives task",
                None,
            );
        }
        for claim_id in &task.related_claim_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                claim_id,
                &task.task_id,
                "requires_task",
                "requires task",
                None,
            );
        }
        for document_id in &task.related_document_ids {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                document_id,
                &task.task_id,
                "requires_task",
                "requires task",
                None,
            );
        }
    }

    for product in &matter.work_products {
        add_case_graph_node(
            &mut nodes,
            &mut seen_nodes,
            CaseGraphNode {
                id: product.work_product_id.clone(),
                kind: "work_product".to_string(),
                label: product.title.clone(),
                subtitle: Some(product.product_type.clone()),
                status: Some(product.review_status.clone()),
                risk: (!product
                    .findings
                    .iter()
                    .all(|finding| finding.status != "open"))
                .then(|| "open_findings".to_string()),
                href: Some(format!(
                    "{base}/work-products/{}/editor",
                    product.work_product_id
                )),
                metadata: graph_metadata([
                    ("blocks", product.blocks.len().to_string()),
                    ("findings", product.findings.len().to_string()),
                ]),
            },
        );
        add_case_graph_edge(
            &mut edges,
            &mut seen_edges,
            &matter.summary.matter_id,
            &product.work_product_id,
            "has_work_product",
            "has work product",
            Some(product.review_status.as_str()),
        );
        for anchor in &product.anchors {
            add_case_graph_edge(
                &mut edges,
                &mut seen_edges,
                &product.work_product_id,
                &anchor.target_id,
                "cites_support",
                "uses support",
                Some(anchor.status.as_str()),
            );
        }
    }

    CaseGraphResponse {
        matter_id: matter.summary.matter_id.clone(),
        generated_at: now_string(),
        modes: vec![
            "overview".to_string(),
            "evidence".to_string(),
            "claims".to_string(),
            "timeline".to_string(),
            "authority".to_string(),
            "work_product".to_string(),
            "tasks".to_string(),
        ],
        nodes,
        edges,
        warnings: vec![
            "Graph is derived from current matter records; large-matter paging and graph persistence remain future hardening.".to_string(),
        ],
    }
}

fn build_matter_qc_run(matter: &MatterBundle) -> QcRun {
    let now = now_string();
    let evidence_gaps = build_evidence_gaps(matter);
    let authority_gaps = build_authority_gaps(matter);
    let contradictions = build_contradictions(matter);
    let work_product_findings = matter
        .work_products
        .iter()
        .flat_map(|product| product.findings.clone())
        .filter(|finding| finding.status == "open")
        .collect::<Vec<_>>();
    let work_product_sentences = build_work_product_sentences(matter);
    let suggested_tasks = evidence_gaps
        .iter()
        .map(|gap| CreateTaskRequest {
            title: format!("Resolve evidence gap: {}", gap.title),
            status: Some("todo".to_string()),
            priority: Some(match gap.severity.as_str() {
                "blocking" | "critical" => "high".to_string(),
                "warning" => "med".to_string(),
                _ => "low".to_string(),
            }),
            due_date: None,
            assigned_to: None,
            related_claim_ids: Some(
                (gap.target_type == "claim")
                    .then(|| vec![gap.target_id.clone()])
                    .unwrap_or_default(),
            ),
            related_document_ids: Some(Vec::new()),
            related_deadline_id: None,
            source: Some("qc_run".to_string()),
            description: Some(gap.message.clone()),
        })
        .chain(authority_gaps.iter().map(|gap| {
            CreateTaskRequest {
                title: format!("Resolve authority gap: {}", gap.title),
                status: Some("todo".to_string()),
                priority: Some("med".to_string()),
                due_date: None,
                assigned_to: None,
                related_claim_ids: Some(
                    (gap.target_type == "claim")
                        .then(|| vec![gap.target_id.clone()])
                        .unwrap_or_default(),
                ),
                related_document_ids: Some(Vec::new()),
                related_deadline_id: None,
                source: Some("qc_run".to_string()),
                description: Some(gap.message.clone()),
            }
        }))
        .take(20)
        .collect::<Vec<_>>();
    QcRun {
        qc_run_id: generate_id("qc-run", &format!("{}:{now}", matter.summary.matter_id)),
        id: generate_id("qc-run", &format!("{}:{now}", matter.summary.matter_id)),
        matter_id: matter.summary.matter_id.clone(),
        status: "complete".to_string(),
        mode: "deterministic".to_string(),
        generated_at: now,
        evidence_gaps,
        authority_gaps,
        contradictions,
        fact_findings: matter
            .fact_check_findings
            .iter()
            .filter(|finding| finding.status == "open")
            .cloned()
            .collect(),
        citation_findings: matter
            .citation_check_findings
            .iter()
            .filter(|finding| finding.status == "open")
            .cloned()
            .collect(),
        work_product_findings,
        work_product_sentences,
        suggested_tasks,
        warnings: vec![
            "Matter QC is deterministic and provider-free; verify every filing decision manually."
                .to_string(),
        ],
    }
}

fn build_work_product_sentences(matter: &MatterBundle) -> Vec<WorkProductSentence> {
    let mut sentences = Vec::new();
    for product in &matter.work_products {
        for (index, block) in flatten_work_product_blocks(&product.document_ast.blocks)
            .into_iter()
            .filter(|block| !block.text.trim().is_empty())
            .enumerate()
        {
            let mut finding_ids = block.rule_finding_ids.clone();
            for finding in product
                .findings
                .iter()
                .filter(|finding| finding.status == "open" && finding.target_id == block.block_id)
            {
                if !finding_ids.iter().any(|id| id == &finding.finding_id) {
                    finding_ids.push(finding.finding_id.clone());
                }
            }
            let support_status = block.support_status.clone().unwrap_or_else(|| {
                if !finding_ids.is_empty() {
                    "needs_review".to_string()
                } else if block.fact_ids.is_empty()
                    && block.evidence_ids.is_empty()
                    && block.authorities.is_empty()
                {
                    "unsupported".to_string()
                } else {
                    "supported".to_string()
                }
            });
            sentences.push(WorkProductSentence {
                sentence_id: generate_id(
                    "wp-sentence",
                    &format!("{}:{}:{index}", product.work_product_id, block.block_id),
                ),
                id: generate_id(
                    "wp-sentence",
                    &format!("{}:{}:{index}", product.work_product_id, block.block_id),
                ),
                matter_id: matter.summary.matter_id.clone(),
                work_product_id: product.work_product_id.clone(),
                block_id: block.block_id,
                text: truncate_qc_text(&block.text),
                index: index as u64,
                support_status,
                fact_ids: block.fact_ids,
                evidence_ids: block.evidence_ids,
                authority_refs: block.authorities,
                finding_ids,
            });
            if sentences.len() >= 500 {
                return sentences;
            }
        }
    }
    sentences
}

fn build_matter_audit_events(matter: &MatterBundle) -> Vec<AuditEvent> {
    let mut events = vec![audit_event(
        matter,
        "matter_created",
        "matter",
        &matter.summary.matter_id,
        format!("Matter created: {}.", matter.summary.name),
        &matter.summary.created_at,
        BTreeMap::new(),
    )];
    for document in &matter.documents {
        let mut metadata = BTreeMap::new();
        metadata.insert("filename".to_string(), document.filename.clone());
        metadata.insert(
            "processing_status".to_string(),
            document.processing_status.clone(),
        );
        events.push(audit_event(
            matter,
            "document_uploaded",
            "document",
            &document.document_id,
            format!("Uploaded document: {}.", document.title),
            &document.uploaded_at,
            metadata,
        ));
    }
    for task in &matter.tasks {
        let mut metadata = BTreeMap::new();
        metadata.insert("status".to_string(), task.status.clone());
        metadata.insert("priority".to_string(), task.priority.clone());
        events.push(audit_event(
            matter,
            "task_recorded",
            "task",
            &task.task_id,
            format!("Task tracked: {}.", task.title),
            &matter.summary.updated_at,
            metadata,
        ));
    }
    for product in &matter.work_products {
        let mut metadata = BTreeMap::new();
        metadata.insert("product_type".to_string(), product.product_type.clone());
        metadata.insert("status".to_string(), product.status.clone());
        events.push(audit_event(
            matter,
            "work_product_updated",
            "work_product",
            &product.work_product_id,
            format!("Work product updated: {}.", product.title),
            &product.updated_at,
            metadata,
        ));
    }
    events.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    events.truncate(200);
    events
}

fn audit_event(
    matter: &MatterBundle,
    event_type: &str,
    target_type: &str,
    target_id: &str,
    summary: String,
    created_at: &str,
    metadata: BTreeMap<String, String>,
) -> AuditEvent {
    AuditEvent {
        audit_event_id: generate_id(
            "audit",
            &format!(
                "{}:{event_type}:{target_type}:{target_id}:{created_at}",
                matter.summary.matter_id
            ),
        ),
        id: generate_id(
            "audit",
            &format!(
                "{}:{event_type}:{target_type}:{target_id}:{created_at}",
                matter.summary.matter_id
            ),
        ),
        matter_id: matter.summary.matter_id.clone(),
        event_type: event_type.to_string(),
        actor: "system".to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        summary,
        created_at: created_at.to_string(),
        metadata,
    }
}

fn build_issue_spot_response(
    matter: &MatterBundle,
    request: IssueSpotRequest,
) -> IssueSpotResponse {
    let mode = request
        .mode
        .unwrap_or_else(|| "deterministic_review".to_string());
    let limit = request.limit.unwrap_or(12) as usize;
    let mut suggestions = Vec::new();
    if matter.claims.is_empty() && !matter.facts.is_empty() {
        let fact_ids = matter
            .facts
            .iter()
            .take(5)
            .map(|fact| fact.fact_id.clone())
            .collect::<Vec<_>>();
        suggestions.push(IssueSuggestion {
            suggestion_id: generate_id(
                "issue",
                &format!("{}:claim-intake", matter.summary.matter_id),
            ),
            id: generate_id(
                "issue",
                &format!("{}:claim-intake", matter.summary.matter_id),
            ),
            matter_id: matter.summary.matter_id.clone(),
            issue_type: "claim_suggestion".to_string(),
            title: "Build first claim theory from reviewed facts".to_string(),
            summary: "Facts exist, but no claim or defense theory has been created yet."
                .to_string(),
            confidence: 0.68,
            severity: "warning".to_string(),
            status: "open".to_string(),
            fact_ids,
            evidence_ids: Vec::new(),
            document_ids: Vec::new(),
            authority_refs: Vec::new(),
            recommended_action: "Create a claim and map facts to each element.".to_string(),
            mode: mode.clone(),
        });
    }

    for claim in &matter.claims {
        let missing_elements = claim
            .elements
            .iter()
            .filter(|element| !element.satisfied || element.fact_ids.is_empty())
            .count();
        if missing_elements > 0 {
            suggestions.push(IssueSuggestion {
                suggestion_id: generate_id(
                    "issue",
                    &format!(
                        "{}:{}:missing-elements",
                        matter.summary.matter_id, claim.claim_id
                    ),
                ),
                id: generate_id(
                    "issue",
                    &format!(
                        "{}:{}:missing-elements",
                        matter.summary.matter_id, claim.claim_id
                    ),
                ),
                matter_id: matter.summary.matter_id.clone(),
                issue_type: "element_gap".to_string(),
                title: format!(
                    "{} has {} missing element(s)",
                    claim.title, missing_elements
                ),
                summary: "One or more elements lack reviewed facts or evidence.".to_string(),
                confidence: 0.82,
                severity: "serious".to_string(),
                status: "open".to_string(),
                fact_ids: claim.fact_ids.clone(),
                evidence_ids: claim.evidence_ids.clone(),
                document_ids: Vec::new(),
                authority_refs: claim.authorities.clone(),
                recommended_action:
                    "Run element mapping and attach facts/evidence to each missing element."
                        .to_string(),
                mode: mode.clone(),
            });
        }
    }

    for document in &matter.documents {
        if document.document_type == "complaint" {
            suggestions.push(IssueSuggestion {
                suggestion_id: generate_id(
                    "issue",
                    &format!("{}:{}:answer-profile", matter.summary.matter_id, document.document_id),
                ),
                id: generate_id(
                    "issue",
                    &format!("{}:{}:answer-profile", matter.summary.matter_id, document.document_id),
                ),
                matter_id: matter.summary.matter_id.clone(),
                issue_type: "answer_workflow".to_string(),
                title: "Uploaded complaint may need an answer workflow".to_string(),
                summary: "A complaint document is present; parse allegations and build admit/deny responses before answer drafting.".to_string(),
                confidence: 0.74,
                severity: "warning".to_string(),
                status: "open".to_string(),
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                document_ids: vec![document.document_id.clone()],
                authority_refs: Vec::new(),
                recommended_action: "Open the Answer profile and create a response grid from numbered allegations.".to_string(),
                mode: mode.clone(),
            });
        }
    }

    for deadline in &matter.deadlines {
        if deadline.status != "complete" && deadline.days_remaining <= 14 {
            suggestions.push(IssueSuggestion {
                suggestion_id: generate_id(
                    "issue",
                    &format!(
                        "{}:{}:deadline-risk",
                        matter.summary.matter_id, deadline.deadline_id
                    ),
                ),
                id: generate_id(
                    "issue",
                    &format!(
                        "{}:{}:deadline-risk",
                        matter.summary.matter_id, deadline.deadline_id
                    ),
                ),
                matter_id: matter.summary.matter_id.clone(),
                issue_type: "deadline_risk".to_string(),
                title: format!("Deadline due soon: {}", deadline.title),
                summary: format!("{} is due on {}.", deadline.title, deadline.due_date),
                confidence: 0.9,
                severity: if deadline.days_remaining < 0 {
                    "critical".to_string()
                } else {
                    deadline.severity.clone()
                },
                status: "open".to_string(),
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                document_ids: Vec::new(),
                authority_refs: deadline
                    .source_citation
                    .as_ref()
                    .map(|citation| AuthorityRef {
                        citation: citation.clone(),
                        canonical_id: deadline
                            .source_canonical_id
                            .clone()
                            .unwrap_or_else(|| citation.clone()),
                        reason: Some("Deadline source".to_string()),
                        pinpoint: None,
                    })
                    .into_iter()
                    .collect(),
                recommended_action:
                    "Confirm trigger date, create completion tasks, and update status.".to_string(),
                mode: mode.clone(),
            });
        }
    }

    IssueSpotResponse {
        matter_id: matter.summary.matter_id.clone(),
        generated_at: now_string(),
        mode,
        suggestions: suggestions.into_iter().take(limit).collect(),
        warnings: vec![
            "Issue spotting is deterministic and conservative; live AI suggestions are provider-gated.".to_string(),
        ],
    }
}

fn build_evidence_gaps(matter: &MatterBundle) -> Vec<EvidenceGap> {
    let mut gaps = Vec::new();
    for fact in &matter.facts {
        if fact.source_document_ids.is_empty()
            && fact.source_evidence_ids.is_empty()
            && fact.source_spans.is_empty()
        {
            let gap_id = generate_id(
                "evidence-gap",
                &format!("{}:{}:source", matter.summary.matter_id, fact.fact_id),
            );
            gaps.push(EvidenceGap {
                id: gap_id.clone(),
                gap_id,
                matter_id: matter.summary.matter_id.clone(),
                target_type: "fact".to_string(),
                target_id: fact.fact_id.clone(),
                title: truncate_graph_label(&fact.statement),
                message: "Fact has no document, evidence, or source-span support.".to_string(),
                severity: if fact.status == "supported" {
                    "warning".to_string()
                } else {
                    "info".to_string()
                },
                status: "open".to_string(),
                fact_ids: vec![fact.fact_id.clone()],
                evidence_ids: Vec::new(),
            });
        }
    }
    for claim in &matter.claims {
        for element in &claim.elements {
            if !element.satisfied || element.fact_ids.is_empty() {
                let gap_id = generate_id(
                    "evidence-gap",
                    &format!(
                        "{}:{}:{}",
                        matter.summary.matter_id, claim.claim_id, element.element_id
                    ),
                );
                gaps.push(EvidenceGap {
                    id: gap_id.clone(),
                    gap_id,
                    matter_id: matter.summary.matter_id.clone(),
                    target_type: "element".to_string(),
                    target_id: element.element_id.clone(),
                    title: truncate_graph_label(&element.text),
                    message: format!(
                        "Element in '{}' is missing fact or evidence support.",
                        claim.title
                    ),
                    severity: "serious".to_string(),
                    status: "open".to_string(),
                    fact_ids: element.fact_ids.clone(),
                    evidence_ids: element.evidence_ids.clone(),
                });
            }
        }
    }
    gaps
}

fn build_authority_gaps(matter: &MatterBundle) -> Vec<AuthorityGap> {
    let mut gaps = Vec::new();
    for claim in &matter.claims {
        if claim.authorities.is_empty()
            && claim
                .elements
                .iter()
                .all(|element| element.authority.is_none() && element.authorities.is_empty())
        {
            let gap_id = generate_id(
                "authority-gap",
                &format!("{}:{}:authority", matter.summary.matter_id, claim.claim_id),
            );
            gaps.push(AuthorityGap {
                id: gap_id.clone(),
                gap_id,
                matter_id: matter.summary.matter_id.clone(),
                target_type: "claim".to_string(),
                target_id: claim.claim_id.clone(),
                title: claim.title.clone(),
                message: "Claim has no attached controlling authority.".to_string(),
                severity: "warning".to_string(),
                status: "open".to_string(),
                authority_refs: Vec::new(),
            });
        }
    }
    for product in &matter.work_products {
        if product.anchors.iter().all(|anchor| {
            anchor.target_type != "authority" && anchor.target_type != "legal_authority"
        }) {
            let gap_id = generate_id(
                "authority-gap",
                &format!(
                    "{}:{}:authority",
                    matter.summary.matter_id, product.work_product_id
                ),
            );
            gaps.push(AuthorityGap {
                id: gap_id.clone(),
                gap_id,
                matter_id: matter.summary.matter_id.clone(),
                target_type: "work_product".to_string(),
                target_id: product.work_product_id.clone(),
                title: product.title.clone(),
                message: "Work product has no authority anchors yet.".to_string(),
                severity: "info".to_string(),
                status: "open".to_string(),
                authority_refs: Vec::new(),
            });
        }
    }
    gaps
}

fn build_contradictions(matter: &MatterBundle) -> Vec<Contradiction> {
    matter
        .evidence
        .iter()
        .filter(|evidence| !evidence.contradicts_fact_ids.is_empty())
        .map(|evidence| {
            let contradiction_id = generate_id(
                "contradiction",
                &format!("{}:{}", matter.summary.matter_id, evidence.evidence_id),
            );
            Contradiction {
                id: contradiction_id.clone(),
                contradiction_id,
                matter_id: matter.summary.matter_id.clone(),
                title: "Contradictory evidence linked".to_string(),
                message: truncate_graph_label(&evidence.quote),
                severity: "warning".to_string(),
                status: "open".to_string(),
                fact_ids: evidence.contradicts_fact_ids.clone(),
                evidence_ids: vec![evidence.evidence_id.clone()],
                source_document_ids: vec![evidence.document_id.clone()],
            }
        })
        .collect()
}

fn build_matter_export_package(matter: &MatterBundle, format: &str) -> ExportPackage {
    let now = now_string();
    let work_product_ids = matter
        .work_products
        .iter()
        .map(|product| product.work_product_id.clone())
        .collect::<Vec<_>>();
    let open_findings = matter
        .work_products
        .iter()
        .flat_map(|product| product.findings.iter())
        .filter(|finding| finding.status == "open")
        .count()
        + matter
            .fact_check_findings
            .iter()
            .filter(|finding| finding.status == "open")
            .count()
        + matter
            .citation_check_findings
            .iter()
            .filter(|finding| finding.status == "open")
            .count();
    let mut warnings = vec![
        "Package is review-needed and not court-ready until final formatting/QC review is complete."
            .to_string(),
    ];
    if matches!(format, "pdf" | "docx") {
        warnings.push(
            "Matter-level PDF/DOCX package currently references AST-backed WorkProduct exports; dedicated final renderer remains required."
                .to_string(),
        );
    }
    if open_findings > 0 {
        warnings.push(format!("{open_findings} open QC finding(s) remain."));
    }
    if work_product_ids.is_empty() {
        warnings.push("No WorkProducts are available to include yet.".to_string());
    }
    ExportPackage {
        export_package_id: generate_id(
            "export-package",
            &format!("{}:{format}:{now}", matter.summary.matter_id),
        ),
        id: generate_id(
            "export-package",
            &format!("{}:{format}:{now}", matter.summary.matter_id),
        ),
        matter_id: matter.summary.matter_id.clone(),
        format: format.to_string(),
        status: if open_findings > 0 {
            "blocked_review_needed".to_string()
        } else {
            "review_needed".to_string()
        },
        profile: "oregon-circuit-civil-matter-package".to_string(),
        created_at: now,
        artifact_count: work_product_ids.len() as u64,
        work_product_ids,
        warnings,
        download_url: None,
    }
}

fn add_case_graph_node(
    nodes: &mut Vec<CaseGraphNode>,
    seen: &mut HashSet<String>,
    node: CaseGraphNode,
) {
    if seen.insert(node.id.clone()) {
        nodes.push(node);
    }
}

fn add_case_graph_edge(
    edges: &mut Vec<CaseGraphEdge>,
    seen: &mut HashSet<String>,
    source: &str,
    target: &str,
    kind: &str,
    label: &str,
    status: Option<&str>,
) {
    if source.is_empty() || target.is_empty() {
        return;
    }
    let id = format!("{kind}:{source}:{target}");
    if seen.insert(id.clone()) {
        edges.push(CaseGraphEdge {
            id,
            source: source.to_string(),
            target: target.to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            status: status.map(str::to_string),
            metadata: BTreeMap::new(),
        });
    }
}

fn add_authority_node_and_edge(
    nodes: &mut Vec<CaseGraphNode>,
    edges: &mut Vec<CaseGraphEdge>,
    seen_nodes: &mut HashSet<String>,
    seen_edges: &mut HashSet<String>,
    source_id: &str,
    authority: &AuthorityRef,
) {
    add_case_graph_node(
        nodes,
        seen_nodes,
        CaseGraphNode {
            id: authority.canonical_id.clone(),
            kind: "authority".to_string(),
            label: authority.citation.clone(),
            subtitle: authority.reason.clone(),
            status: Some("attached".to_string()),
            risk: None,
            href: Some(format!(
                "/statutes/{}",
                authority.canonical_id.replace('/', "%2F")
            )),
            metadata: graph_metadata([("canonical_id", authority.canonical_id.clone())]),
        },
    );
    add_case_graph_edge(
        edges,
        seen_edges,
        source_id,
        &authority.canonical_id,
        "supported_by_authority",
        "authority",
        Some("attached"),
    );
}

fn graph_metadata<const N: usize>(pairs: [(&str, String); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn truncate_graph_label(value: &str) -> String {
    const LIMIT: usize = 120;
    let trimmed = value.trim();
    if trimmed.chars().count() <= LIMIT {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(LIMIT).collect::<String>())
    }
}

fn truncate_qc_text(value: &str) -> String {
    const LIMIT: usize = 280;
    let trimmed = value.trim();
    if trimmed.chars().count() <= LIMIT {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(LIMIT).collect::<String>())
    }
}

fn humanize_export_format(format: &str) -> String {
    match format {
        "docx" => "DOCX".to_string(),
        "pdf" => "PDF".to_string(),
        "filing_packet" => "Filing packet".to_string(),
        other => humanize_product_type(other),
    }
}

fn oregon_civil_motion_rule_pack() -> RulePack {
    RulePack {
        rule_pack_id: "oregon-circuit-civil-motion-orcp-utcr".to_string(),
        name: "Oregon Circuit Civil Motion - ORCP + UTCR".to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2025-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![
            rule_definition(
                "orcp-14-motion-writing-grounds-relief",
                "ORCP 14 A",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-14-motions/",
                "blocking",
                "work_product",
                "rules",
                "Motion must state grounds and relief sought.",
                "ORCP 14 A requires written motions to state grounds with particularity and set forth requested relief.",
                "Complete the relief requested and argument blocks.",
                false,
            ),
            rule_definition(
                "orcp-14-motion-form",
                "ORCP 14 B",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-14-motions/",
                "serious",
                "formatting",
                "formatting",
                "Motion form requires caption, signing, and other form review.",
                "ORCP 14 B applies pleading form rules, including signing requirements, to motions and other papers.",
                "Review caption, signature, and document form before export.",
                false,
            ),
            rule_definition(
                "orcp-17-motion-signature",
                "ORCP 17",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-17-signing-of-pleadings-motions-and-other-papers-sanctions/",
                "serious",
                "signature",
                "rules",
                "Motion signature and certification require review.",
                "ORCP 17 governs signing and certification obligations for pleadings, motions, and other papers.",
                "Complete and review the signature/certification block.",
                false,
            ),
            rule_definition(
                "utcr-5-010-conferral",
                "UTCR 5.010",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "warning",
                "work_product",
                "rules",
                "Motion may need conferral certificate.",
                "UTCR 5.010 describes conferral and certificate requirements for specified civil motions.",
                "Add a conferral certificate or mark why it is not required.",
                false,
            ),
            rule_definition(
                "utcr-5-020-authorities",
                "UTCR 5.020",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "warning",
                "authority",
                "authority",
                "Motion authorities require review.",
                "UTCR 5.020 covers authorities in motions and related requirements.",
                "Link controlling authority in the legal-standard and argument blocks.",
                false,
            ),
            rule_definition(
                "utcr-2-010-document-form",
                "UTCR 2.010",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "serious",
                "formatting",
                "formatting",
                "Motion document form requires review.",
                "UTCR 2.010 applies document form requirements to pleadings and motions.",
                "Use the Oregon court-paper formatting profile before export.",
                true,
            ),
        ],
    }
}

fn default_work_product_ai_commands(product_type: &str) -> Vec<WorkProductAiCommandState> {
    let common = [
        ("tighten", "Make concise"),
        ("find_missing_evidence", "Find missing evidence"),
        ("find_missing_authority", "Find missing authority"),
        ("citation_check", "Check citations"),
        ("export_check", "Check export readiness"),
    ];
    let profile = if product_type == "motion" {
        vec![
            ("draft_motion_argument", "Draft motion argument"),
            ("draft_legal_standard", "Draft legal standard"),
            ("draft_proposed_order", "Draft proposed order"),
        ]
    } else {
        vec![("draft_section", "Draft section")]
    };
    profile
        .into_iter()
        .chain(common)
        .map(|(command_id, label)| WorkProductAiCommandState {
            command_id: command_id.to_string(),
            label: label.to_string(),
            status: "disabled".to_string(),
            mode: "template".to_string(),
            description: "Provider-free template state; no live AI provider is configured."
                .to_string(),
            last_message: None,
        })
        .collect()
}

fn work_product_event(
    matter_id: &str,
    work_product_id: &str,
    event_type: &str,
    target_type: &str,
    target_id: &str,
    summary: &str,
) -> WorkProductHistoryEvent {
    let event_id = generate_id(
        "work-product-event",
        &format!("{work_product_id}:{event_type}"),
    );
    WorkProductHistoryEvent {
        id: event_id.clone(),
        event_id,
        matter_id: matter_id.to_string(),
        work_product_id: work_product_id.to_string(),
        event_type: event_type.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        summary: summary.to_string(),
        timestamp: now_string(),
    }
}

fn refresh_work_product_state(product: &mut WorkProduct) {
    if product.document_ast.blocks.is_empty() {
        rebuild_work_product_ast_from_projection(product);
    } else {
        normalize_work_product_ast(product);
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
    }
    product.blocks.sort_by_key(|block| block.ordinal);
    for (index, block) in product.blocks.iter_mut().enumerate() {
        block.ordinal = index as u64 + 1;
        block.updated_at = product.updated_at.clone();
        block
            .prosemirror_json
            .get_or_insert_with(|| prosemirror_doc_for_text(&block.text));
    }
    normalize_work_product_ast(product);
    product.review_status = if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "blocking")
    {
        "blocked".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        "needs_human_review".to_string()
    } else {
        "ready_for_review".to_string()
    };
}

fn summarize_work_product_for_list(product: &mut WorkProduct) {
    product.blocks.clear();
    product.marks.clear();
    product.anchors.clear();
    product.document_ast.blocks.clear();
    product.document_ast.links.clear();
    product.document_ast.citations.clear();
    product.document_ast.exhibits.clear();
    product.document_ast.rule_findings.clear();
    for artifact in &mut product.artifacts {
        artifact.content_preview = export_content_preview(&artifact.content_preview);
    }
}

fn summarize_version_snapshot_for_list(snapshot: &mut VersionSnapshot) {
    snapshot.full_state_inline = None;
}

fn latest_snapshot_id(snapshots: &[VersionSnapshot]) -> Option<String> {
    snapshots
        .iter()
        .max_by_key(|snapshot| snapshot.sequence_number)
        .map(|snapshot| snapshot.snapshot_id.clone())
}

fn validate_ast_patch_concurrency(
    patch: &AstPatch,
    route_work_product_id: &str,
    current_document_hash: &str,
    current_snapshot_id: Option<&str>,
) -> ApiResult<()> {
    if let Some(patch_work_product_id) = patch.work_product_id.as_deref() {
        if patch_work_product_id != route_work_product_id {
            return Err(ApiError::BadRequest(
                "AST patch work_product_id does not match route target.".to_string(),
            ));
        }
    }
    if patch.base_document_hash.is_none() && patch.base_snapshot_id.is_none() {
        return Err(ApiError::BadRequest(
            "AST patch requires base_document_hash or base_snapshot_id.".to_string(),
        ));
    }
    if let Some(base_hash) = patch.base_document_hash.as_deref() {
        if base_hash != current_document_hash {
            return Err(ApiError::Conflict(format!(
                "AST patch conflict: conflict_field=base_document_hash base_document_hash={base_hash} current_document_hash={current_document_hash}."
            )));
        }
    }
    if let Some(base_snapshot_id) = patch.base_snapshot_id.as_deref() {
        if current_snapshot_id != Some(base_snapshot_id) {
            return Err(ApiError::Conflict(
                "AST patch conflict: conflict_field=base_snapshot_id.".to_string(),
            ));
        }
    }
    Ok(())
}

fn rebuild_work_product_ast_from_projection(product: &mut WorkProduct) {
    let blocks = product.blocks.clone();
    product.document_ast = work_product_document_from_projection(product, blocks);
}

fn normalize_work_product_ast(product: &mut WorkProduct) {
    let now = if product.updated_at.is_empty() {
        now_string()
    } else {
        product.updated_at.clone()
    };
    let document_type_source = if product.document_ast.document_type.trim().is_empty()
        || product.document_ast.document_type == "custom"
    {
        product.product_type.as_str()
    } else {
        product.document_ast.document_type.as_str()
    };
    let document_type = normalize_work_product_type_lossy(document_type_source);
    product.product_type = document_type.clone();
    product.document_ast.schema_version = if product.document_ast.schema_version.is_empty() {
        default_work_product_schema_version()
    } else {
        product.document_ast.schema_version.clone()
    };
    product.document_ast.document_id = if product.document_ast.document_id.is_empty() {
        format!("{}:document", product.work_product_id)
    } else {
        product.document_ast.document_id.clone()
    };
    product.document_ast.work_product_id = product.work_product_id.clone();
    product.document_ast.matter_id = product.matter_id.clone();
    product.document_ast.draft_id = product.source_draft_id.clone();
    product.document_ast.document_type = document_type.clone();
    product.document_ast.product_type = document_type;
    product.document_ast.title = product.title.clone();
    product.document_ast.metadata.status = product.status.clone();
    product.document_ast.metadata.rule_pack_id = Some(product.rule_pack.rule_pack_id.clone());
    product.document_ast.metadata.formatting_profile_id =
        Some(product.formatting_profile.profile_id.clone());
    if product.document_ast.created_at.is_empty() {
        product.document_ast.created_at = product.created_at.clone();
    }
    product.document_ast.updated_at = now.clone();
    if product.document_ast.rule_findings.is_empty() && !product.findings.is_empty() {
        product.document_ast.rule_findings = product.findings.clone();
    }
    product.findings = product.document_ast.rule_findings.clone();
    for block in &mut product.document_ast.blocks {
        normalize_ast_block(block, &product.matter_id, &product.work_product_id, &now);
    }
}

fn normalize_ast_block(
    block: &mut WorkProductBlock,
    matter_id: &str,
    work_product_id: &str,
    now: &str,
) {
    if block.id.is_empty() {
        block.id = block.block_id.clone();
    }
    block.matter_id = matter_id.to_string();
    block.work_product_id = work_product_id.to_string();
    if block.created_at.is_empty() {
        block.created_at = now.to_string();
    }
    block.updated_at = now.to_string();
    if block.review_status.is_empty() {
        block.review_status = "needs_review".to_string();
    }
    if block.block_type.is_empty() {
        block.block_type = "paragraph".to_string();
    }
    for child in &mut block.children {
        child.parent_block_id = Some(block.block_id.clone());
        normalize_ast_block(child, matter_id, work_product_id, now);
    }
}

fn work_product_document_from_projection(
    product: &WorkProduct,
    blocks: Vec<WorkProductBlock>,
) -> WorkProductDocument {
    let now = if product.updated_at.is_empty() {
        product.created_at.clone()
    } else {
        product.updated_at.clone()
    };
    let mut flat_blocks = blocks
        .into_iter()
        .enumerate()
        .map(|(index, mut block)| {
            if block.id.is_empty() {
                block.id = block.block_id.clone();
            }
            if block.created_at.is_empty() {
                block.created_at = product.created_at.clone();
            }
            block.updated_at = now.clone();
            block.ordinal = if block.ordinal == 0 {
                index as u64 + 1
            } else {
                block.ordinal
            };
            block.children.clear();
            block.links.clear();
            block.citations.clear();
            block.exhibits.clear();
            block.rule_finding_ids = product
                .findings
                .iter()
                .filter(|finding| finding.target_id == block.block_id)
                .map(|finding| finding.finding_id.clone())
                .collect();
            block
        })
        .collect::<Vec<_>>();
    flat_blocks.sort_by_key(|block| block.ordinal);

    let mut links = Vec::new();
    let mut citations = Vec::new();
    let exhibits = Vec::new();
    for block in &mut flat_blocks {
        for fact_id in &block.fact_ids {
            let link_id = format!(
                "{}:link:fact:{}",
                block.block_id,
                sanitize_path_segment(fact_id)
            );
            block.links.push(link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "fact".to_string(),
                target_id: fact_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for evidence_id in &block.evidence_ids {
            let link_id = format!(
                "{}:link:evidence:{}",
                block.block_id,
                sanitize_path_segment(evidence_id)
            );
            block.links.push(link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "evidence".to_string(),
                target_id: evidence_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for authority in &block.authorities {
            let link_id = format!(
                "{}:link:authority:{}",
                block.block_id,
                sanitize_path_segment(&authority.canonical_id)
            );
            let citation_use_id = format!(
                "{}:citation:{}",
                block.block_id,
                sanitize_path_segment(&authority.citation)
            );
            block.links.push(link_id.clone());
            block.citations.push(citation_use_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "legal_authority".to_string(),
                target_id: authority.canonical_id.clone(),
                relation: "cites".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
            citations.push(WorkProductCitationUse {
                citation_use_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                raw_text: authority.citation.clone(),
                normalized_citation: Some(authority.citation.clone()),
                target_type: "provision".to_string(),
                target_id: Some(authority.canonical_id.clone()),
                pinpoint: authority.pinpoint.clone(),
                status: "resolved".to_string(),
                resolver_message: authority.reason.clone(),
                created_at: now.clone(),
            });
        }
    }

    for anchor in &product.anchors {
        let link_id = anchor.anchor_id.clone();
        links.push(WorkProductLink {
            link_id: link_id.clone(),
            source_block_id: anchor.block_id.clone(),
            source_text_range: anchor.quote.as_ref().map(|quote| TextRange {
                start_offset: 0,
                end_offset: quote.chars().count() as u64,
                quote: Some(quote.clone()),
            }),
            target_type: anchor.target_type.clone(),
            target_id: anchor.target_id.clone(),
            relation: anchor.relation.clone(),
            confidence: None,
            created_by: "user".to_string(),
            created_at: now.clone(),
        });
        if let Some(block) = flat_blocks
            .iter_mut()
            .find(|block| block.block_id == anchor.block_id)
        {
            push_unique(&mut block.links, link_id.clone());
            if anchor.anchor_type == "authority" || anchor.citation.is_some() {
                let citation_use_id = format!("{link_id}:citation");
                push_unique(&mut block.citations, citation_use_id.clone());
                citations.push(WorkProductCitationUse {
                    citation_use_id,
                    source_block_id: anchor.block_id.clone(),
                    source_text_range: None,
                    raw_text: anchor
                        .citation
                        .clone()
                        .unwrap_or_else(|| anchor.target_id.clone()),
                    normalized_citation: anchor.citation.clone(),
                    target_type: "provision".to_string(),
                    target_id: anchor
                        .canonical_id
                        .clone()
                        .or_else(|| Some(anchor.target_id.clone())),
                    pinpoint: anchor.pinpoint.clone(),
                    status: if anchor.status == "resolved" {
                        "resolved".to_string()
                    } else {
                        "needs_review".to_string()
                    },
                    resolver_message: None,
                    created_at: now.clone(),
                });
            }
        }
    }

    WorkProductDocument {
        schema_version: default_work_product_schema_version(),
        document_id: format!("{}:document", product.work_product_id),
        work_product_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        draft_id: product.source_draft_id.clone(),
        document_type: product.product_type.clone(),
        product_type: product.product_type.clone(),
        title: product.title.clone(),
        metadata: WorkProductMetadata {
            work_product_type: Some(product.product_type.clone()),
            document_title: Some(product.title.clone()),
            jurisdiction: Some(product.profile.jurisdiction.clone()),
            court: None,
            county: None,
            case_number: None,
            rule_pack_id: Some(product.rule_pack.rule_pack_id.clone()),
            template_id: None,
            formatting_profile_id: Some(product.formatting_profile.profile_id.clone()),
            parties: None,
            status: product.status.clone(),
            created_at: Some(product.created_at.clone()),
            updated_at: Some(now.clone()),
            created_by: None,
            last_modified_by: None,
        },
        blocks: build_work_product_block_tree(&flat_blocks),
        links,
        citations,
        exhibits,
        rule_findings: product.findings.clone(),
        tombstones: product.document_ast.tombstones.clone(),
        created_at: product.created_at.clone(),
        updated_at: now,
    }
}

fn build_work_product_block_tree(flat_blocks: &[WorkProductBlock]) -> Vec<WorkProductBlock> {
    let ids = flat_blocks
        .iter()
        .map(|block| block.block_id.clone())
        .collect::<HashSet<_>>();
    flat_blocks
        .iter()
        .filter(|block| {
            block
                .parent_block_id
                .as_ref()
                .map(|parent_id| !ids.contains(parent_id))
                .unwrap_or(true)
        })
        .cloned()
        .map(|mut block| {
            attach_work_product_children(&mut block, flat_blocks);
            block
        })
        .collect()
}

fn attach_work_product_children(block: &mut WorkProductBlock, flat_blocks: &[WorkProductBlock]) {
    block.children = flat_blocks
        .iter()
        .filter(|candidate| candidate.parent_block_id.as_deref() == Some(&block.block_id))
        .cloned()
        .map(|mut child| {
            attach_work_product_children(&mut child, flat_blocks);
            child
        })
        .collect();
}

fn flatten_work_product_blocks(blocks: &[WorkProductBlock]) -> Vec<WorkProductBlock> {
    let mut flattened = Vec::new();
    for block in blocks {
        flatten_work_product_block(block, &mut flattened);
    }
    flattened.sort_by_key(|block| block.ordinal);
    flattened
}

fn flatten_work_product_block(block: &WorkProductBlock, flattened: &mut Vec<WorkProductBlock>) {
    let mut current = block.clone();
    current.children.clear();
    flattened.push(current);
    for child in &block.children {
        flatten_work_product_block(child, flattened);
    }
}

fn canonical_work_product_blocks(product: &WorkProduct) -> Vec<WorkProductBlock> {
    flatten_work_product_blocks(&product.document_ast.blocks)
}

fn block_text_excerpt(text: &str, inline_limit: u64) -> String {
    if should_inline_payload(text.len(), inline_limit) {
        return text.to_string();
    }
    const GRAPH_EXCERPT_CHARS: usize = 4096;
    text.chars().take(GRAPH_EXCERPT_CHARS).collect()
}

fn work_product_block_graph_payload(
    block: &WorkProductBlock,
    inline_limit: u64,
) -> ApiResult<String> {
    let mut payload = json_value(block)?;
    if !should_inline_payload(block.text.len(), inline_limit) {
        if let Some(object) = payload.as_object_mut() {
            let excerpt = block_text_excerpt(&block.text, inline_limit);
            object.insert(
                "text".to_string(),
                serde_json::Value::String(excerpt.clone()),
            );
            object.insert(
                "text_excerpt".to_string(),
                serde_json::Value::String(excerpt),
            );
            object.insert(
                "text_hash".to_string(),
                serde_json::Value::String(sha256_hex(block.text.as_bytes())),
            );
            object.insert(
                "text_size_bytes".to_string(),
                serde_json::Value::Number(serde_json::Number::from(block.text.len() as u64)),
            );
            object.insert(
                "text_storage_status".to_string(),
                serde_json::Value::String("graph_excerpt".to_string()),
            );
        }
    }
    to_payload(&payload)
}

fn apply_ast_operation(
    document: &mut WorkProductDocument,
    operation: &AstOperation,
) -> ApiResult<()> {
    match operation {
        AstOperation::InsertBlock {
            parent_id,
            after_block_id,
            block,
        } => insert_ast_block(
            &mut document.blocks,
            parent_id.as_deref(),
            after_block_id.as_deref(),
            block.clone(),
        ),
        AstOperation::UpdateBlock {
            block_id, after, ..
        } => {
            let block = find_ast_block_mut(&mut document.blocks, block_id)
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
            merge_json_patch_into_block(block, after)
        }
        AstOperation::DeleteBlock { block_id, .. } => {
            delete_ast_block(&mut document.blocks, block_id)
                .map(|_| ())
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))
        }
        AstOperation::MoveBlock {
            block_id,
            parent_id,
            after_block_id,
        } => {
            let block = delete_ast_block(&mut document.blocks, block_id)
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
            insert_ast_block(
                &mut document.blocks,
                parent_id.as_deref(),
                after_block_id.as_deref(),
                block,
            )
        }
        AstOperation::SplitBlock {
            block_id,
            offset,
            new_block_id,
        } => split_ast_document_block(document, block_id, *offset, new_block_id),
        AstOperation::MergeBlocks {
            first_block_id,
            second_block_id,
        } => merge_ast_document_blocks(document, first_block_id, second_block_id),
        AstOperation::RenumberParagraphs => {
            let mut next = 1;
            renumber_ast_paragraphs(&mut document.blocks, &mut next);
            Ok(())
        }
        AstOperation::AddCitation { citation } => {
            let citation_id = citation.citation_use_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &citation.source_block_id)
                .ok_or_else(|| {
                    ApiError::NotFound("AST citation source block not found".to_string())
                })?;
            validate_optional_text_range(&block.text, citation.source_text_range.as_ref())?;
            push_unique(&mut block.citations, citation_id.clone());
            document
                .citations
                .retain(|item| item.citation_use_id != citation_id);
            document.citations.push(citation.clone());
            Ok(())
        }
        AstOperation::ResolveCitation {
            citation_use_id,
            normalized_citation,
            target_type,
            target_id,
            status,
        } => {
            let citation = document
                .citations
                .iter_mut()
                .find(|item| item.citation_use_id == *citation_use_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("CitationUse {citation_use_id} not found"))
                })?;
            if let Some(value) = normalized_citation {
                citation.normalized_citation = Some(value.clone());
            }
            if let Some(value) = target_type {
                citation.target_type = value.clone();
            }
            if let Some(value) = target_id {
                citation.target_id = Some(value.clone());
            }
            if let Some(value) = status {
                citation.status = value.clone();
            }
            Ok(())
        }
        AstOperation::RemoveCitation { citation_use_id } => {
            document
                .citations
                .retain(|item| item.citation_use_id != *citation_use_id);
            remove_block_ref(&mut document.blocks, citation_use_id, "citation");
            Ok(())
        }
        AstOperation::AddLink { link } => {
            let link_id = link.link_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &link.source_block_id)
                .ok_or_else(|| ApiError::NotFound("AST link source block not found".to_string()))?;
            validate_optional_text_range(&block.text, link.source_text_range.as_ref())?;
            push_unique(&mut block.links, link_id.clone());
            document.links.retain(|item| item.link_id != link_id);
            document.links.push(link.clone());
            Ok(())
        }
        AstOperation::RemoveLink { link_id } => {
            document.links.retain(|item| item.link_id != *link_id);
            remove_block_ref(&mut document.blocks, link_id, "link");
            Ok(())
        }
        AstOperation::AddExhibitReference { exhibit } => {
            let exhibit_id = exhibit.exhibit_reference_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &exhibit.source_block_id)
                .ok_or_else(|| {
                    ApiError::NotFound("AST exhibit source block not found".to_string())
                })?;
            validate_optional_text_range(&block.text, exhibit.source_text_range.as_ref())?;
            push_unique(&mut block.exhibits, exhibit_id.clone());
            document
                .exhibits
                .retain(|item| item.exhibit_reference_id != exhibit_id);
            document.exhibits.push(exhibit.clone());
            Ok(())
        }
        AstOperation::ResolveExhibitReference {
            exhibit_reference_id,
            exhibit_id,
            status,
        } => {
            let exhibit = document
                .exhibits
                .iter_mut()
                .find(|item| item.exhibit_reference_id == *exhibit_reference_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("ExhibitReference {exhibit_reference_id} not found"))
                })?;
            if let Some(value) = exhibit_id {
                exhibit.exhibit_id = Some(value.clone());
            }
            if let Some(value) = status {
                exhibit.status = value.clone();
            }
            Ok(())
        }
        AstOperation::AddRuleFinding { finding } => {
            let finding_id = finding.finding_id.clone();
            let block =
                find_ast_block_mut(&mut document.blocks, &finding.target_id).ok_or_else(|| {
                    ApiError::NotFound("AST rule finding target block not found".to_string())
                })?;
            push_unique(&mut block.rule_finding_ids, finding_id.clone());
            document
                .rule_findings
                .retain(|item| item.finding_id != finding_id);
            document.rule_findings.push(finding.clone());
            Ok(())
        }
        AstOperation::ResolveRuleFinding { finding_id, status } => {
            let finding = document
                .rule_findings
                .iter_mut()
                .find(|item| item.finding_id == *finding_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Rule finding {finding_id} not found"))
                })?;
            finding.status = status.clone();
            finding.updated_at = now_string();
            Ok(())
        }
        AstOperation::ApplyTemplate { template_id } => {
            document.metadata.template_id = Some(template_id.clone());
            Ok(())
        }
    }
}

fn insert_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    parent_id: Option<&str>,
    after_block_id: Option<&str>,
    mut block: WorkProductBlock,
) -> ApiResult<()> {
    block.parent_block_id = parent_id.map(str::to_string);
    let target_blocks = if let Some(parent_id) = parent_id {
        &mut find_ast_block_mut(blocks, parent_id)
            .ok_or_else(|| ApiError::NotFound(format!("Parent block {parent_id} not found")))?
            .children
    } else {
        blocks
    };
    let insert_index = after_block_id
        .and_then(|after_id| {
            target_blocks
                .iter()
                .position(|candidate| candidate.block_id == after_id)
                .map(|index| index + 1)
        })
        .unwrap_or_else(|| target_blocks.len());
    target_blocks.insert(insert_index, block);
    for (index, block) in target_blocks.iter_mut().enumerate() {
        block.ordinal = index as u64 + 1;
    }
    Ok(())
}

fn find_ast_block_mut<'a>(
    blocks: &'a mut [WorkProductBlock],
    block_id: &str,
) -> Option<&'a mut WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block_mut(&mut block.children, block_id) {
            return Some(found);
        }
    }
    None
}

fn delete_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    block_id: &str,
) -> Option<WorkProductBlock> {
    if let Some(index) = blocks.iter().position(|block| block.block_id == block_id) {
        return Some(blocks.remove(index));
    }
    for block in blocks {
        if let Some(deleted) = delete_ast_block(&mut block.children, block_id) {
            return Some(deleted);
        }
    }
    None
}

fn merge_json_patch_into_block(
    block: &mut WorkProductBlock,
    patch: &serde_json::Value,
) -> ApiResult<()> {
    let mut value = json_value(block)?;
    merge_json_objects(&mut value, patch);
    let mut updated: WorkProductBlock =
        serde_json::from_value(value).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    if updated.block_id.is_empty() {
        updated.block_id = block.block_id.clone();
    }
    *block = updated;
    Ok(())
}

fn merge_json_objects(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    base.remove(key);
                } else {
                    merge_json_objects(base.entry(key).or_insert(serde_json::Value::Null), value);
                }
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

fn split_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    block_id: &str,
    offset: u64,
    new_block_id: &str,
) -> ApiResult<()> {
    let (parent_id, new_block) = {
        let block = find_ast_block_mut(blocks, block_id)
            .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
        let text_len = block.text.chars().count() as u64;
        if offset > text_len {
            return Err(ApiError::BadRequest(
                "AST split offset extends past the source block.".to_string(),
            ));
        }
        let split_at = offset as usize;
        let left = block.text.chars().take(split_at).collect::<String>();
        let right = block.text.chars().skip(split_at).collect::<String>();
        block.text = left;
        let mut new_block = block.clone();
        new_block.block_id = new_block_id.to_string();
        new_block.id = new_block_id.to_string();
        new_block.text = right;
        new_block.ordinal = block.ordinal + 1;
        new_block.fact_ids.clear();
        new_block.evidence_ids.clear();
        new_block.authorities.clear();
        new_block.mark_ids.clear();
        new_block.links.clear();
        new_block.citations.clear();
        new_block.exhibits.clear();
        new_block.rule_finding_ids.clear();
        (block.parent_block_id.clone(), new_block)
    };
    insert_ast_block(blocks, parent_id.as_deref(), Some(block_id), new_block)
}

fn split_ast_document_block(
    document: &mut WorkProductDocument,
    block_id: &str,
    offset: u64,
    new_block_id: &str,
) -> ApiResult<()> {
    let source = find_ast_block(&document.blocks, block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
    if offset > source.text.chars().count() as u64 {
        return Err(ApiError::BadRequest(
            "AST split offset extends past the source block.".to_string(),
        ));
    }
    ensure_split_does_not_straddle_ranges(document, block_id, offset)?;
    split_ast_block(&mut document.blocks, block_id, offset, new_block_id)?;
    rehome_split_records(document, block_id, offset, new_block_id);
    rebuild_document_block_refs(document);
    Ok(())
}

fn merge_ast_blocks(
    blocks: &mut Vec<WorkProductBlock>,
    first_block_id: &str,
    second_block_id: &str,
) -> ApiResult<()> {
    let second = delete_ast_block(blocks, second_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {second_block_id} not found")))?;
    let first = find_ast_block_mut(blocks, first_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {first_block_id} not found")))?;
    if !first.text.is_empty() && !second.text.is_empty() {
        first.text.push_str("\n\n");
    }
    first.text.push_str(&second.text);
    for id in second.links {
        push_unique(&mut first.links, id);
    }
    for id in second.citations {
        push_unique(&mut first.citations, id);
    }
    for id in second.exhibits {
        push_unique(&mut first.exhibits, id);
    }
    for id in second.rule_finding_ids {
        push_unique(&mut first.rule_finding_ids, id);
    }
    Ok(())
}

fn merge_ast_document_blocks(
    document: &mut WorkProductDocument,
    first_block_id: &str,
    second_block_id: &str,
) -> ApiResult<()> {
    let first = find_ast_block(&document.blocks, first_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {first_block_id} not found")))?;
    let second = find_ast_block(&document.blocks, second_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {second_block_id} not found")))?;
    let separator_len = if !first.text.is_empty() && !second.text.is_empty() {
        2
    } else {
        0
    };
    let range_shift = first.text.chars().count() as u64 + separator_len;
    merge_ast_blocks(&mut document.blocks, first_block_id, second_block_id)?;
    rehome_merge_records(document, first_block_id, second_block_id, range_shift);
    rebuild_document_block_refs(document);
    Ok(())
}

fn ensure_split_does_not_straddle_ranges(
    document: &WorkProductDocument,
    block_id: &str,
    offset: u64,
) -> ApiResult<()> {
    for link in &document.links {
        ensure_range_does_not_straddle_split(
            &link.source_block_id,
            link.source_text_range.as_ref(),
            block_id,
            offset,
        )?;
    }
    for citation in &document.citations {
        ensure_range_does_not_straddle_split(
            &citation.source_block_id,
            citation.source_text_range.as_ref(),
            block_id,
            offset,
        )?;
    }
    for exhibit in &document.exhibits {
        ensure_range_does_not_straddle_split(
            &exhibit.source_block_id,
            exhibit.source_text_range.as_ref(),
            block_id,
            offset,
        )?;
    }
    Ok(())
}

fn ensure_range_does_not_straddle_split(
    source_block_id: &str,
    range: Option<&TextRange>,
    block_id: &str,
    offset: u64,
) -> ApiResult<()> {
    if source_block_id == block_id
        && range
            .map(|range| range.start_offset < offset && range.end_offset > offset)
            .unwrap_or(false)
    {
        return Err(ApiError::BadRequest(
            "AST split would divide an existing text-range reference.".to_string(),
        ));
    }
    Ok(())
}

fn rehome_split_records(
    document: &mut WorkProductDocument,
    block_id: &str,
    offset: u64,
    new_block_id: &str,
) {
    for link in &mut document.links {
        if link.source_block_id == block_id {
            if let Some(range) = link.source_text_range.as_mut() {
                if range.start_offset >= offset {
                    link.source_block_id = new_block_id.to_string();
                    shift_text_range_back(range, offset);
                }
            }
        }
    }
    for citation in &mut document.citations {
        if citation.source_block_id == block_id {
            if let Some(range) = citation.source_text_range.as_mut() {
                if range.start_offset >= offset {
                    citation.source_block_id = new_block_id.to_string();
                    shift_text_range_back(range, offset);
                }
            }
        }
    }
    for exhibit in &mut document.exhibits {
        if exhibit.source_block_id == block_id {
            if let Some(range) = exhibit.source_text_range.as_mut() {
                if range.start_offset >= offset {
                    exhibit.source_block_id = new_block_id.to_string();
                    shift_text_range_back(range, offset);
                }
            }
        }
    }
}

fn rehome_merge_records(
    document: &mut WorkProductDocument,
    first_block_id: &str,
    second_block_id: &str,
    range_shift: u64,
) {
    for link in &mut document.links {
        if link.source_block_id == second_block_id {
            link.source_block_id = first_block_id.to_string();
            if let Some(range) = link.source_text_range.as_mut() {
                shift_text_range_forward(range, range_shift);
            }
        }
    }
    for citation in &mut document.citations {
        if citation.source_block_id == second_block_id {
            citation.source_block_id = first_block_id.to_string();
            if let Some(range) = citation.source_text_range.as_mut() {
                shift_text_range_forward(range, range_shift);
            }
        }
    }
    for exhibit in &mut document.exhibits {
        if exhibit.source_block_id == second_block_id {
            exhibit.source_block_id = first_block_id.to_string();
            if let Some(range) = exhibit.source_text_range.as_mut() {
                shift_text_range_forward(range, range_shift);
            }
        }
    }
    for finding in &mut document.rule_findings {
        if finding.target_id == second_block_id {
            finding.target_id = first_block_id.to_string();
        }
    }
}

fn shift_text_range_back(range: &mut TextRange, amount: u64) {
    range.start_offset = range.start_offset.saturating_sub(amount);
    range.end_offset = range.end_offset.saturating_sub(amount);
}

fn shift_text_range_forward(range: &mut TextRange, amount: u64) {
    range.start_offset = range.start_offset.saturating_add(amount);
    range.end_offset = range.end_offset.saturating_add(amount);
}

fn rebuild_document_block_refs(document: &mut WorkProductDocument) {
    clear_document_block_refs(&mut document.blocks);
    for link in &document.links {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &link.source_block_id) {
            push_unique(&mut block.links, link.link_id.clone());
        }
    }
    for citation in &document.citations {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &citation.source_block_id) {
            push_unique(&mut block.citations, citation.citation_use_id.clone());
        }
    }
    for exhibit in &document.exhibits {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &exhibit.source_block_id) {
            push_unique(&mut block.exhibits, exhibit.exhibit_reference_id.clone());
        }
    }
    for finding in &document.rule_findings {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &finding.target_id) {
            push_unique(&mut block.rule_finding_ids, finding.finding_id.clone());
        }
    }
}

fn clear_document_block_refs(blocks: &mut [WorkProductBlock]) {
    for block in blocks {
        block.links.clear();
        block.citations.clear();
        block.exhibits.clear();
        block.rule_finding_ids.clear();
        clear_document_block_refs(&mut block.children);
    }
}

fn renumber_ast_paragraphs(blocks: &mut [WorkProductBlock], next: &mut u64) {
    for block in blocks {
        if matches!(
            block.block_type.as_str(),
            "numbered_paragraph" | "paragraph"
        ) && matches!(
            block.role.as_str(),
            "factual_allegation"
                | "legal_allegation"
                | "jurisdiction"
                | "venue"
                | "claim_element"
                | "relief"
                | "fact"
        ) {
            block.paragraph_number = Some(*next);
            *next += 1;
        }
        renumber_ast_paragraphs(&mut block.children, next);
    }
}

fn remove_block_ref(blocks: &mut [WorkProductBlock], id: &str, ref_kind: &str) {
    for block in blocks {
        match ref_kind {
            "citation" => block.citations.retain(|value| value != id),
            "link" => block.links.retain(|value| value != id),
            "exhibit" => block.exhibits.retain(|value| value != id),
            _ => {}
        }
        remove_block_ref(&mut block.children, id, ref_kind);
    }
}

fn export_content_preview(content: &str) -> String {
    const PREVIEW_CHARS: usize = 16 * 1024;
    content.chars().take(PREVIEW_CHARS).collect()
}

fn work_product_qc_status(product: &WorkProduct) -> String {
    if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "blocking")
    {
        "blocking".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "serious")
    {
        "serious".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        "warning".to_string()
    } else if product.findings.is_empty() {
        "not_run".to_string()
    } else {
        "clear".to_string()
    }
}

fn normalize_export_format(value: &str) -> ApiResult<String> {
    let format = value.to_ascii_lowercase();
    let supported = [
        "pdf",
        "docx",
        "html",
        "markdown",
        "text",
        "plain_text",
        "json",
    ];
    if supported.contains(&format.as_str()) {
        Ok(format)
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported work product export format {format}"
        )))
    }
}

fn count_work_product_words(product: &WorkProduct) -> u64 {
    canonical_work_product_blocks(product)
        .iter()
        .map(|block| block.text.split_whitespace().count() as u64)
        .sum()
}

fn work_product_to_draft(product: &WorkProduct) -> CaseDraft {
    let blocks = canonical_work_product_blocks(product);
    let sections = blocks
        .iter()
        .map(|block| DraftSection {
            section_id: block.block_id.clone(),
            heading: block.title.clone(),
            body: block.text.clone(),
            citations: block.authorities.clone(),
        })
        .collect::<Vec<_>>();
    let paragraphs = blocks
        .iter()
        .map(|block| DraftParagraph {
            paragraph_id: block.block_id.clone(),
            index: block.ordinal,
            role: block.role.clone(),
            text: block.text.clone(),
            fact_ids: block.fact_ids.clone(),
            evidence_ids: block.evidence_ids.clone(),
            authorities: block.authorities.clone(),
            factcheck_status: if block.fact_ids.is_empty() && block.evidence_ids.is_empty() {
                "needs_evidence".to_string()
            } else if block.authorities.is_empty()
                && matches!(
                    block.role.as_str(),
                    "legal_standard" | "argument" | "analysis"
                )
            {
                "needs_authority".to_string()
            } else {
                "supported".to_string()
            },
            factcheck_note: None,
        })
        .collect::<Vec<_>>();
    CaseDraft {
        id: product.work_product_id.clone(),
        draft_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        title: product.title.clone(),
        description: format!(
            "Migrated shared {} work product.",
            humanize_product_type(&product.product_type)
        ),
        draft_type: product.product_type.clone(),
        kind: product.product_type.clone(),
        status: product.status.clone(),
        created_at: product.created_at.clone(),
        updated_at: product.updated_at.clone(),
        word_count: count_work_product_words(product),
        sections,
        paragraphs,
    }
}

fn work_product_from_draft(draft: &CaseDraft) -> WorkProduct {
    let mut product = WorkProduct {
        id: draft.draft_id.clone(),
        work_product_id: draft.draft_id.clone(),
        matter_id: draft.matter_id.clone(),
        title: draft.title.clone(),
        product_type: draft.kind.clone(),
        status: draft.status.clone(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "migrated_from_draft".to_string(),
        source_draft_id: Some(draft.draft_id.clone()),
        source_complaint_id: None,
        created_at: draft.created_at.clone(),
        updated_at: draft.updated_at.clone(),
        profile: work_product_profile(&draft.kind),
        document_ast: WorkProductDocument::default(),
        blocks: work_product_blocks_from_draft(draft),
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: Vec::new(),
        artifacts: Vec::new(),
        history: vec![work_product_event(
            &draft.matter_id,
            &draft.draft_id,
            "migrated_from_draft",
            "draft",
            &draft.draft_id,
            "Legacy Draft node migrated into the shared WorkProduct AST.",
        )],
        ai_commands: default_work_product_ai_commands(&draft.kind),
        formatting_profile: default_work_product_formatting_profile(&draft.kind),
        rule_pack: work_product_rule_pack(&draft.kind),
    };
    refresh_work_product_state(&mut product);
    product
}

fn work_product_blocks_from_draft(draft: &CaseDraft) -> Vec<WorkProductBlock> {
    let section_blocks =
        draft
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| WorkProductBlock {
                id: section.section_id.clone(),
                block_id: section.section_id.clone(),
                matter_id: draft.matter_id.clone(),
                work_product_id: draft.draft_id.clone(),
                block_type: "section".to_string(),
                role: slug(&section.heading).replace('-', "_"),
                title: section.heading.clone(),
                text: section.body.clone(),
                ordinal: index as u64 + 1,
                parent_block_id: None,
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                authorities: section.citations.clone(),
                mark_ids: Vec::new(),
                locked: false,
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(&section.body)),
                ..WorkProductBlock::default()
            });
    let offset = draft.sections.len() as u64;
    let paragraph_blocks = draft
        .paragraphs
        .iter()
        .enumerate()
        .map(|(index, paragraph)| WorkProductBlock {
            id: paragraph.paragraph_id.clone(),
            block_id: paragraph.paragraph_id.clone(),
            matter_id: draft.matter_id.clone(),
            work_product_id: draft.draft_id.clone(),
            block_type: "paragraph".to_string(),
            role: paragraph.role.clone(),
            title: humanize_product_type(&paragraph.role),
            text: paragraph.text.clone(),
            ordinal: offset + index as u64 + 1,
            parent_block_id: None,
            fact_ids: paragraph.fact_ids.clone(),
            evidence_ids: paragraph.evidence_ids.clone(),
            authorities: paragraph.authorities.clone(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(&paragraph.text)),
            ..WorkProductBlock::default()
        });
    section_blocks.chain(paragraph_blocks).collect()
}

fn work_product_from_complaint(complaint: &ComplaintDraft) -> WorkProduct {
    let mut blocks = Vec::new();
    blocks.push(WorkProductBlock {
        id: format!("{}:block:caption", complaint.complaint_id),
        block_id: format!("{}:block:caption", complaint.complaint_id),
        matter_id: complaint.matter_id.clone(),
        work_product_id: complaint.complaint_id.clone(),
        block_type: "caption".to_string(),
        role: "caption".to_string(),
        title: "Caption".to_string(),
        text: format!(
            "{}\n{} v. {}",
            complaint.caption.document_title,
            complaint.caption.plaintiff_names.join(", "),
            complaint.caption.defendant_names.join(", ")
        ),
        ordinal: 1,
        parent_block_id: None,
        fact_ids: Vec::new(),
        evidence_ids: Vec::new(),
        authorities: Vec::new(),
        mark_ids: Vec::new(),
        locked: false,
        review_status: complaint.review_status.clone(),
        prosemirror_json: None,
        ..WorkProductBlock::default()
    });
    for section in &complaint.sections {
        blocks.push(WorkProductBlock {
            id: section.section_id.clone(),
            block_id: section.section_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "section".to_string(),
            role: section.section_type.clone(),
            title: section.title.clone(),
            text: String::new(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: None,
            fact_ids: Vec::new(),
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: section.review_status.clone(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
    }
    for count in &complaint.counts {
        blocks.push(WorkProductBlock {
            id: count.count_id.clone(),
            block_id: count.count_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "count".to_string(),
            role: "count".to_string(),
            title: count.title.clone(),
            text: count.legal_theory.clone(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: None,
            fact_ids: count.fact_ids.clone(),
            evidence_ids: count.evidence_ids.clone(),
            authorities: count.authorities.clone(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: count.health.clone(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
    }
    for paragraph in &complaint.paragraphs {
        let evidence_ids = paragraph
            .evidence_uses
            .iter()
            .filter_map(|use_ref| {
                use_ref
                    .evidence_id
                    .clone()
                    .or_else(|| use_ref.document_id.clone())
            })
            .collect::<Vec<_>>();
        let authorities = paragraph
            .citation_uses
            .iter()
            .filter_map(|citation| {
                citation
                    .canonical_id
                    .clone()
                    .map(|canonical_id| AuthorityRef {
                        citation: citation.citation.clone(),
                        canonical_id,
                        reason: Some("Complaint citation use".to_string()),
                        pinpoint: citation.pinpoint.clone(),
                    })
            })
            .collect::<Vec<_>>();
        blocks.push(WorkProductBlock {
            id: paragraph.paragraph_id.clone(),
            block_id: paragraph.paragraph_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "paragraph".to_string(),
            role: paragraph.role.clone(),
            title: format!("Paragraph {}", paragraph.number),
            text: paragraph.text.clone(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: paragraph
                .section_id
                .clone()
                .or_else(|| paragraph.count_id.clone()),
            fact_ids: paragraph.fact_ids.clone(),
            evidence_ids,
            authorities,
            mark_ids: Vec::new(),
            locked: paragraph.locked,
            review_status: paragraph.review_status.clone(),
            prosemirror_json: Some(prosemirror_doc_for_text(&paragraph.text)),
            ..WorkProductBlock::default()
        });
    }
    let mut product = WorkProduct {
        id: complaint.complaint_id.clone(),
        work_product_id: complaint.complaint_id.clone(),
        matter_id: complaint.matter_id.clone(),
        title: complaint.title.clone(),
        product_type: "complaint".to_string(),
        status: complaint.status.clone(),
        review_status: complaint.review_status.clone(),
        setup_stage: complaint.setup_stage.clone(),
        source_draft_id: None,
        source_complaint_id: Some(complaint.complaint_id.clone()),
        created_at: complaint.created_at.clone(),
        updated_at: complaint.updated_at.clone(),
        profile: work_product_profile("complaint"),
        document_ast: WorkProductDocument::default(),
        blocks,
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: complaint
            .findings
            .iter()
            .map(|finding| WorkProductFinding {
                id: finding.finding_id.clone(),
                finding_id: finding.finding_id.clone(),
                matter_id: finding.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                rule_id: finding.rule_id.clone(),
                rule_pack_id: Some(complaint.rule_pack.rule_pack_id.clone()),
                source_citation: None,
                source_url: None,
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                target_type: finding.target_type.clone(),
                target_id: finding.target_id.clone(),
                message: finding.message.clone(),
                explanation: finding.explanation.clone(),
                suggested_fix: finding.suggested_fix.clone(),
                auto_fix_available: false,
                primary_action: WorkProductAction {
                    action_id: finding.primary_action.action_id.clone(),
                    label: finding.primary_action.label.clone(),
                    action_type: finding.primary_action.action_type.clone(),
                    href: finding.primary_action.href.clone(),
                    target_type: finding.primary_action.target_type.clone(),
                    target_id: finding.primary_action.target_id.clone(),
                },
                status: finding.status.clone(),
                created_at: finding.created_at.clone(),
                updated_at: finding.updated_at.clone(),
            })
            .collect(),
        artifacts: complaint
            .export_artifacts
            .iter()
            .map(|artifact| WorkProductArtifact {
                id: artifact.artifact_id.clone(),
                artifact_id: artifact.artifact_id.clone(),
                matter_id: artifact.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                format: artifact.format.clone(),
                profile: artifact.profile.clone(),
                mode: artifact.mode.clone(),
                status: artifact.status.clone(),
                download_url: artifact.download_url.clone(),
                page_count: artifact.page_count,
                generated_at: artifact.generated_at.clone(),
                warnings: artifact.warnings.clone(),
                content_preview: artifact.content_preview.clone(),
                snapshot_id: None,
                artifact_hash: Some(sha256_hex(artifact.content_preview.as_bytes())),
                render_profile_hash: Some(sha256_hex(
                    format!("{}:{}:{}", artifact.format, artifact.profile, artifact.mode)
                        .as_bytes(),
                )),
                qc_status_at_export: None,
                changed_since_export: Some(false),
                immutable: Some(true),
                object_blob_id: None,
                size_bytes: Some(artifact.content_preview.len() as u64),
                mime_type: Some(export_mime_type(&artifact.format).to_string()),
                storage_status: Some("legacy_inline".to_string()),
            })
            .collect(),
        history: complaint
            .history
            .iter()
            .map(|event| WorkProductHistoryEvent {
                id: event.event_id.clone(),
                event_id: event.event_id.clone(),
                matter_id: event.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                event_type: event.event_type.clone(),
                target_type: event.target_type.clone(),
                target_id: event.target_id.clone(),
                summary: event.summary.clone(),
                timestamp: event.timestamp.clone(),
            })
            .collect(),
        ai_commands: default_work_product_ai_commands("complaint"),
        formatting_profile: complaint.formatting_profile.clone(),
        rule_pack: complaint.rule_pack.clone(),
    };
    refresh_work_product_state(&mut product);
    product
}

fn default_complaint_from_matter(
    matter: &MatterSummary,
    complaint_id: &str,
    title: &str,
    parties: &[CaseParty],
    claims: &[CaseClaim],
    facts: &[CaseFact],
    now: &str,
) -> ComplaintDraft {
    let matter_id = matter.matter_id.clone();
    let complaint_parties = if parties.is_empty() {
        vec![
            ComplaintParty {
                party_id: format!("{complaint_id}:party:plaintiff"),
                matter_party_id: None,
                name: "Plaintiff".to_string(),
                role: "plaintiff".to_string(),
                party_type: "individual".to_string(),
                represented_by: None,
            },
            ComplaintParty {
                party_id: format!("{complaint_id}:party:defendant"),
                matter_party_id: None,
                name: "Defendant".to_string(),
                role: "defendant".to_string(),
                party_type: "entity".to_string(),
                represented_by: None,
            },
        ]
    } else {
        parties
            .iter()
            .map(|party| ComplaintParty {
                party_id: format!(
                    "{complaint_id}:party:{}",
                    sanitize_path_segment(&party.party_id)
                ),
                matter_party_id: Some(party.party_id.clone()),
                name: party.name.clone(),
                role: party.role.clone(),
                party_type: party.party_type.clone(),
                represented_by: party.represented_by.clone(),
            })
            .collect()
    };
    let plaintiff_names = role_names(&complaint_parties, &["plaintiff", "petitioner"]);
    let defendant_names = role_names(&complaint_parties, &["defendant", "respondent"]);
    let sections = vec![
        complaint_section(
            &matter_id,
            complaint_id,
            "jurisdiction",
            "Jurisdiction and Venue",
            1,
        ),
        complaint_section(&matter_id, complaint_id, "facts", "Factual Allegations", 2),
        complaint_section(&matter_id, complaint_id, "counts", "Claims for Relief", 3),
        complaint_section(&matter_id, complaint_id, "relief", "Prayer for Relief", 4),
    ];
    let mut paragraphs = Vec::new();
    paragraphs.push(pleading_paragraph(
        &matter_id,
        complaint_id,
        &format!("{complaint_id}:paragraph:1"),
        Some(sections[0].section_id.clone()),
        None,
        "jurisdiction_venue",
        &format!(
            "This action is brought in {} and venue is proper in {}.",
            empty_as_review_needed(&matter.court),
            empty_as_review_needed(&matter.jurisdiction)
        ),
        1,
        Vec::new(),
        Vec::new(),
    ));
    if facts.is_empty() {
        paragraphs.push(pleading_paragraph(
            &matter_id,
            complaint_id,
            &format!("{complaint_id}:paragraph:2"),
            Some(sections[1].section_id.clone()),
            None,
            "factual_allegation",
            "Plaintiff alleges the following ultimate facts after human review and evidentiary support are added.",
            2,
            Vec::new(),
            Vec::new(),
        ));
    } else {
        for (index, fact) in facts.iter().take(12).enumerate() {
            paragraphs.push(pleading_paragraph(
                &matter_id,
                complaint_id,
                &format!("{complaint_id}:paragraph:{}", index + 2),
                Some(sections[1].section_id.clone()),
                None,
                "factual_allegation",
                &fact.statement,
                index as u64 + 2,
                vec![fact.fact_id.clone()],
                fact.source_evidence_ids.clone(),
            ));
        }
    }
    let mut counts = Vec::new();
    let complaint_claims = claims
        .iter()
        .filter(|claim| claim.kind != "defense")
        .take(8)
        .collect::<Vec<_>>();
    for claim in complaint_claims {
        let count_id = format!("{complaint_id}:count:{}", counts.len() + 1);
        let paragraph_id = format!("{complaint_id}:paragraph:{}", paragraphs.len() + 1);
        paragraphs.push(pleading_paragraph(
            &matter_id,
            complaint_id,
            &paragraph_id,
            Some(sections[2].section_id.clone()),
            Some(count_id.clone()),
            "count_heading",
            &format!("COUNT {} - {}", counts.len() + 1, claim.title),
            paragraphs.len() as u64 + 1,
            claim.fact_ids.clone(),
            claim.evidence_ids.clone(),
        ));
        counts.push(ComplaintCount {
            id: count_id.clone(),
            count_id,
            matter_id: matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            ordinal: counts.len() as u64 + 1,
            title: claim.title.clone(),
            claim_id: Some(claim.claim_id.clone()),
            legal_theory: claim.legal_theory.clone(),
            against_party_ids: Vec::new(),
            element_ids: claim
                .elements
                .iter()
                .map(|element| element.element_id.clone())
                .collect(),
            fact_ids: claim.fact_ids.clone(),
            evidence_ids: claim.evidence_ids.clone(),
            authorities: claim.authorities.clone(),
            relief_ids: Vec::new(),
            paragraph_ids: vec![paragraph_id],
            incorporation_range: Some("1 through preceding paragraph".to_string()),
            health: "needs_review".to_string(),
            weaknesses: Vec::new(),
        });
    }
    let relief_id = format!("{complaint_id}:relief:general");
    let relief = vec![ReliefRequest {
        id: relief_id.clone(),
        relief_id,
        matter_id: matter_id.clone(),
        complaint_id: complaint_id.to_string(),
        category: "general".to_string(),
        text: "Plaintiff requests relief determined by the court after human review, including any damages, costs, fees, or equitable relief supported by law and evidence.".to_string(),
        amount: None,
        authority_ids: Vec::new(),
        supported: false,
    }];
    let mut complaint = ComplaintDraft {
        complaint_id: complaint_id.to_string(),
        id: complaint_id.to_string(),
        matter_id: matter_id.clone(),
        title: title.to_string(),
        status: "draft".to_string(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "guided_setup".to_string(),
        active_profile_id: "oregon-circuit-civil-complaint".to_string(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
        caption: ComplaintCaption {
            court_name: matter.court.clone(),
            county: infer_county(&matter.court),
            case_number: matter.case_number.clone(),
            document_title: "Complaint".to_string(),
            plaintiff_names,
            defendant_names,
            jury_demand: false,
            jurisdiction: matter.jurisdiction.clone(),
            venue: matter.court.clone(),
        },
        parties: complaint_parties,
        sections,
        counts,
        paragraphs,
        relief,
        signature: SignatureBlock {
            name: String::new(),
            bar_number: None,
            firm: None,
            address: String::new(),
            phone: String::new(),
            email: String::new(),
            signature_date: None,
        },
        certificate_of_service: Some(CertificateOfService {
            certificate_id: format!("{complaint_id}:certificate:service"),
            method: "review_needed".to_string(),
            served_parties: Vec::new(),
            service_date: None,
            text: "Certificate of service requires human review before filing or service."
                .to_string(),
            review_status: "needs_review".to_string(),
        }),
        formatting_profile: default_formatting_profile(),
        rule_pack: oregon_civil_complaint_rule_pack(),
        findings: Vec::new(),
        export_artifacts: Vec::new(),
        history: vec![complaint_event(
            &matter_id,
            complaint_id,
            "complaint_created",
            "complaint",
            complaint_id,
            "Structured complaint AST created.",
        )],
        next_actions: Vec::new(),
        ai_commands: default_ai_commands(),
        filing_packet: FilingPacket {
            packet_id: format!("{complaint_id}:packet:filing"),
            matter_id: matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            status: "review_needed".to_string(),
            items: Vec::new(),
            warnings: Vec::new(),
        },
        import_provenance: None,
    };
    apply_matter_rule_profile(
        &mut complaint.rule_pack,
        matter,
        now.get(0..10).unwrap_or(now),
        "complaint",
    );
    refresh_complaint_state(&mut complaint);
    complaint
}

fn build_imported_complaint(
    matter: &MatterSummary,
    document: &CaseDocument,
    complaint_id: &str,
    title: &str,
    text: &str,
    parser_id: &str,
    context: &SourceContext,
    parties: &[CaseParty],
    claims: &[CaseClaim],
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
    now: &str,
) -> ComplaintDraft {
    let mut complaint =
        default_complaint_from_matter(matter, complaint_id, title, parties, claims, &[], now);
    complaint.status = "imported_draft".to_string();
    complaint.setup_stage = "editor".to_string();
    complaint.caption = imported_caption(matter, text, parties);
    complaint.import_provenance = Some(ComplaintImportProvenance {
        document_id: document.document_id.clone(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        source_span_id: None,
        parser_id: parser_id.to_string(),
        parser_version: PARSER_REGISTRY_VERSION.to_string(),
        byte_start: Some(0),
        byte_end: Some(text.len() as u64),
        char_start: Some(0),
        char_end: Some(text.chars().count() as u64),
    });

    let parsed = parse_complaint_structure(text);
    let section_rows = if parsed.sections.is_empty() {
        vec![(
            "imported".to_string(),
            "Imported Draft".to_string(),
            "imported".to_string(),
        )]
    } else {
        parsed.sections
    };
    complaint.sections = section_rows
        .iter()
        .enumerate()
        .map(|(index, (key, title, section_type))| ComplaintSection {
            id: format!("{complaint_id}:section:{}", sanitize_path_segment(key)),
            section_id: format!("{complaint_id}:section:{}", sanitize_path_segment(key)),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            title: title.clone(),
            section_type: section_type.clone(),
            ordinal: index as u64 + 1,
            paragraph_ids: Vec::new(),
            count_ids: Vec::new(),
            review_status: "needs_review".to_string(),
        })
        .collect();

    complaint.counts = parsed
        .counts
        .iter()
        .enumerate()
        .map(|(index, (count_key, title))| {
            let count_id = format!("{complaint_id}:count:{}", sanitize_path_segment(count_key));
            ComplaintCount {
                id: count_id.clone(),
                count_id,
                matter_id: matter.matter_id.clone(),
                complaint_id: complaint_id.to_string(),
                ordinal: index as u64 + 1,
                title: title.clone(),
                claim_id: match_claim_for_count(title, claims),
                legal_theory: title.clone(),
                against_party_ids: Vec::new(),
                element_ids: Vec::new(),
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                authorities: Vec::new(),
                relief_ids: Vec::new(),
                paragraph_ids: Vec::new(),
                incorporation_range: Some("1 through preceding paragraph".to_string()),
                health: "needs_review".to_string(),
                weaknesses: Vec::new(),
            }
        })
        .collect();

    let imported_paragraphs = if parsed.paragraphs.is_empty() {
        vec![ParsedComplaintParagraph {
            original_label: "1".to_string(),
            text: summarize_text(text),
            section_key: complaint
                .sections
                .first()
                .map(|section| section.section_type.clone())
                .unwrap_or_else(|| "imported".to_string()),
            count_key: None,
            byte_start: 0,
            byte_end: text.len() as u64,
            char_start: 0,
            char_end: text.chars().count() as u64,
        }]
    } else {
        parsed.paragraphs
    };
    complaint.paragraphs = imported_paragraphs
        .into_iter()
        .enumerate()
        .map(|(index, parsed)| {
            imported_pleading_paragraph(
                matter,
                complaint_id,
                document,
                parser_id,
                context,
                &complaint.sections,
                &complaint.counts,
                facts,
                evidence,
                index as u64 + 1,
                parsed,
            )
        })
        .collect();

    for count in &mut complaint.counts {
        count.paragraph_ids = complaint
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.count_id.as_deref() == Some(count.count_id.as_str()))
            .map(|paragraph| paragraph.paragraph_id.clone())
            .collect();
        let mut authorities = Vec::new();
        for paragraph in complaint
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.count_id.as_deref() == Some(count.count_id.as_str()))
        {
            for citation in &paragraph.citation_uses {
                if let Some(canonical_id) = &citation.canonical_id {
                    push_authority(
                        &mut authorities,
                        AuthorityRef {
                            citation: citation.citation.clone(),
                            canonical_id: canonical_id.clone(),
                            reason: Some("Imported citation from complaint count.".to_string()),
                            pinpoint: citation.pinpoint.clone(),
                        },
                    );
                }
            }
        }
        count.authorities = authorities;
    }

    complaint.relief = imported_relief_requests(matter, complaint_id, &complaint.paragraphs);
    complaint.history = vec![complaint_event(
        &matter.matter_id,
        complaint_id,
        "complaint_import_started",
        "document",
        &document.document_id,
        "Structured complaint AST created from uploaded draft.",
    )];
    complaint.findings = complaint_rule_findings(&complaint);
    refresh_complaint_state(&mut complaint);
    complaint
}

fn imported_pleading_paragraph(
    matter: &MatterSummary,
    complaint_id: &str,
    document: &CaseDocument,
    parser_id: &str,
    context: &SourceContext,
    sections: &[ComplaintSection],
    counts: &[ComplaintCount],
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
    ordinal: u64,
    parsed: ParsedComplaintParagraph,
) -> PleadingParagraph {
    let paragraph_id = format!("{complaint_id}:paragraph:{ordinal}");
    let section_id = sections
        .iter()
        .find(|section| {
            section
                .section_id
                .ends_with(&sanitize_path_segment(&parsed.section_key))
        })
        .or_else(|| sections.first())
        .map(|section| section.section_id.clone());
    let count_id = parsed.count_key.as_ref().and_then(|count_key| {
        let target = format!("{complaint_id}:count:{}", sanitize_path_segment(count_key));
        counts
            .iter()
            .find(|count| count.count_id == target)
            .map(|count| count.count_id.clone())
    });
    let role = role_for_imported_paragraph(section_id.as_deref(), sections, count_id.as_deref());
    let (fact_ids, evidence_uses) = exact_support_for_paragraph(
        matter,
        complaint_id,
        &paragraph_id,
        &parsed.text,
        facts,
        evidence,
    );
    let mut paragraph = pleading_paragraph(
        &matter.matter_id,
        complaint_id,
        &paragraph_id,
        section_id,
        count_id,
        &role,
        &parsed.text,
        ordinal,
        fact_ids,
        Vec::new(),
    );
    let source_span_id = source_span_id(&document.document_id, "pleading-paragraph", ordinal);
    paragraph.display_number = Some(parsed.original_label.clone());
    paragraph.original_label = Some(parsed.original_label);
    paragraph.source_span_id = Some(source_span_id.clone());
    paragraph.import_provenance = Some(ComplaintImportProvenance {
        document_id: document.document_id.clone(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        source_span_id: Some(source_span_id),
        parser_id: parser_id.to_string(),
        parser_version: PARSER_REGISTRY_VERSION.to_string(),
        byte_start: Some(parsed.byte_start),
        byte_end: Some(parsed.byte_end),
        char_start: Some(parsed.char_start),
        char_end: Some(parsed.char_end),
    });
    paragraph.citation_uses = citation_uses_for_text(
        &matter.matter_id,
        complaint_id,
        &paragraph_id,
        "paragraph",
        &parsed.text,
    );
    paragraph.exhibit_references =
        exhibit_references_for_text(&matter.matter_id, complaint_id, &paragraph_id, &parsed.text);
    paragraph.evidence_uses = evidence_uses;
    paragraph.review_status = "imported_needs_review".to_string();
    paragraph
}

fn parse_complaint_structure(text: &str) -> ParsedComplaintStructure {
    let mut structure = ParsedComplaintStructure::default();
    ensure_import_section(
        &mut structure.sections,
        "imported",
        "Imported Draft",
        "imported",
    );
    let mut current_section_key = "imported".to_string();
    let mut current_count_key: Option<String> = None;
    let mut current: Option<ParsedComplaintParagraph> = None;
    let mut cursor = 0usize;

    for raw_line in text.split_inclusive('\n') {
        let line_start = cursor;
        let line_end = cursor + raw_line.len();
        cursor = line_end;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if let Some(paragraph) = current.as_mut() {
                paragraph.text.push('\n');
                paragraph.byte_end = line_end as u64;
                paragraph.char_end = text[..line_end].chars().count() as u64;
            }
            continue;
        }

        if let Some(heading) = imported_heading(trimmed) {
            finish_import_paragraph(&mut structure.paragraphs, current.take());
            if CLAIM_HEADING_RE.is_match(&heading) {
                ensure_import_section(
                    &mut structure.sections,
                    "claims",
                    "Claims for Relief",
                    "counts",
                );
                current_section_key = "claims".to_string();
                let count_key = format!("count-{}", structure.counts.len() + 1);
                structure.counts.push((count_key.clone(), heading));
                current_count_key = Some(count_key);
            } else {
                let key = sanitize_path_segment(&heading.to_ascii_lowercase());
                let section_type = section_type_for_heading(&heading);
                ensure_import_section(&mut structure.sections, &key, &heading, &section_type);
                current_section_key = key;
                current_count_key = None;
            }
            continue;
        }

        if let Some(caps) = PLEADING_PARAGRAPH_RE.captures(trimmed) {
            finish_import_paragraph(&mut structure.paragraphs, current.take());
            let label = caps.get(1).map(|m| m.as_str()).unwrap_or("0").to_string();
            let body = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            current = Some(ParsedComplaintParagraph {
                original_label: label,
                text: body.to_string(),
                section_key: current_section_key.clone(),
                count_key: current_count_key.clone(),
                byte_start: line_start as u64,
                byte_end: line_end as u64,
                char_start: text[..line_start].chars().count() as u64,
                char_end: text[..line_end].chars().count() as u64,
            });
        } else if let Some(paragraph) = current.as_mut() {
            if !paragraph.text.ends_with('\n') && !paragraph.text.is_empty() {
                paragraph.text.push(' ');
            }
            paragraph.text.push_str(trimmed);
            paragraph.byte_end = line_end as u64;
            paragraph.char_end = text[..line_end].chars().count() as u64;
        }
    }
    finish_import_paragraph(&mut structure.paragraphs, current);
    structure
}

fn finish_import_paragraph(
    paragraphs: &mut Vec<ParsedComplaintParagraph>,
    paragraph: Option<ParsedComplaintParagraph>,
) {
    if let Some(mut paragraph) = paragraph {
        paragraph.text = paragraph
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !paragraph.text.is_empty() {
            paragraphs.push(paragraph);
        }
    }
}

fn ensure_import_section(
    sections: &mut Vec<(String, String, String)>,
    key: &str,
    title: &str,
    section_type: &str,
) {
    if !sections.iter().any(|(existing, _, _)| existing == key) {
        sections.push((key.to_string(), title.to_string(), section_type.to_string()));
    }
}

fn imported_heading(line: &str) -> Option<String> {
    let stripped = line.trim().trim_matches('*').trim();
    let markdown = stripped.strip_prefix('#').map(|_| {
        stripped
            .trim_start_matches('#')
            .trim()
            .trim_matches('*')
            .trim()
            .to_string()
    });
    if let Some(value) = markdown.filter(|value| !value.is_empty()) {
        return Some(value);
    }
    if CLAIM_HEADING_RE.is_match(stripped) {
        return Some(stripped.to_string());
    }
    None
}

fn section_type_for_heading(heading: &str) -> String {
    let value = heading.to_ascii_lowercase();
    if value.contains("caption") {
        "caption".to_string()
    } else if value.contains("jurisdiction") || value.contains("venue") {
        "jurisdiction".to_string()
    } else if value.contains("parties") {
        "parties".to_string()
    } else if value.contains("fact") || value.contains("allegation") {
        "facts".to_string()
    } else if value.contains("prayer") || value.contains("relief") {
        "relief".to_string()
    } else if value.contains("exhibit") {
        "exhibits".to_string()
    } else {
        "custom".to_string()
    }
}

fn role_for_imported_paragraph(
    section_id: Option<&str>,
    sections: &[ComplaintSection],
    count_id: Option<&str>,
) -> String {
    if count_id.is_some() {
        return "count_allegation".to_string();
    }
    let section_type = section_id
        .and_then(|id| sections.iter().find(|section| section.section_id == id))
        .map(|section| section.section_type.as_str())
        .unwrap_or("imported");
    match section_type {
        "jurisdiction" => "jurisdiction_venue",
        "relief" => "relief",
        "caption" | "parties" | "exhibits" => "procedural_notice",
        _ => "factual_allegation",
    }
    .to_string()
}

fn imported_caption(matter: &MatterSummary, text: &str, parties: &[CaseParty]) -> ComplaintCaption {
    let court_name = text
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_uppercase().contains("CIRCUIT COURT"))
        .map(clean_caption_line)
        .unwrap_or_else(|| matter.court.clone());
    let county = text
        .lines()
        .find_map(|line| {
            let upper = line.to_ascii_uppercase();
            upper
                .find("COUNTY OF ")
                .map(|idx| clean_caption_line(&line[idx + "COUNTY OF ".len()..]))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| infer_county(&matter.court));
    let plaintiffs = role_names_from_parties(parties, &["plaintiff", "petitioner"])
        .or_else(|| caption_names_before(text, "Plaintiffs,"))
        .unwrap_or_else(|| vec!["Plaintiff".to_string()]);
    let defendants = role_names_from_parties(parties, &["defendant", "respondent"])
        .or_else(|| caption_names_before(text, "Defendants."))
        .unwrap_or_else(|| vec!["Defendant".to_string()]);
    ComplaintCaption {
        court_name,
        county: county.clone(),
        case_number: matter.case_number.clone(),
        document_title: "Complaint".to_string(),
        plaintiff_names: plaintiffs,
        defendant_names: defendants,
        jury_demand: text.to_ascii_uppercase().contains("JURY TRIAL"),
        jurisdiction: matter.jurisdiction.clone(),
        venue: if county.is_empty() {
            matter.court.clone()
        } else {
            format!("{county} County")
        },
    }
}

fn role_names_from_parties(parties: &[CaseParty], roles: &[&str]) -> Option<Vec<String>> {
    let names = parties
        .iter()
        .filter(|party| {
            roles
                .iter()
                .any(|role| party.role.eq_ignore_ascii_case(role))
        })
        .map(|party| party.name.clone())
        .collect::<Vec<_>>();
    if names.is_empty() { None } else { Some(names) }
}

fn caption_names_before(text: &str, marker: &str) -> Option<Vec<String>> {
    let lines = text.lines().collect::<Vec<_>>();
    let marker_index = lines.iter().position(|line| {
        line.to_ascii_lowercase()
            .contains(&marker.to_ascii_lowercase())
    })?;
    let mut names = Vec::new();
    for line in lines[..marker_index].iter().rev().take(8).rev() {
        let cleaned = clean_caption_line(line);
        if cleaned.is_empty()
            || cleaned.contains("COURT")
            || cleaned.contains("COUNTY")
            || cleaned.eq_ignore_ascii_case("v.")
        {
            continue;
        }
        if cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .count()
            >= 3
        {
            names.push(cleaned.trim_matches(',').to_string());
        }
    }
    if names.is_empty() { None } else { Some(names) }
}

fn clean_caption_line(value: &str) -> String {
    value
        .trim()
        .trim_matches('*')
        .trim_matches(',')
        .trim()
        .to_string()
}

fn imported_complaint_title(document: &CaseDocument, text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| line.to_ascii_uppercase().contains("COMPLAINT"))
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .trim_matches('*')
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| format!("{} structured complaint", document.title))
}

fn looks_like_complaint(filename: &str, text: &str) -> bool {
    let lower_name = filename.to_ascii_lowercase();
    let upper = text.to_ascii_uppercase();
    let numbered = PLEADING_PARAGRAPH_RE.find_iter(text).take(12).count();
    let signals = [
        lower_name.contains("complaint"),
        upper.contains("CIRCUIT COURT"),
        upper.contains("COMPLAINT"),
        upper.contains("CLAIM FOR RELIEF"),
        upper.contains("PRAYER FOR RELIEF"),
        upper.contains("PLAINTIFF"),
        upper.contains("DEFENDANT"),
        ORS_CITATION_RE.is_match(text)
            || ORCP_CITATION_RE.is_match(text)
            || UTCR_CITATION_RE.is_match(text),
        numbered >= 3,
    ];
    signals.iter().filter(|signal| **signal).count() >= 3
}

fn parser_id_for_document(document: &CaseDocument) -> String {
    let filename = document.filename.to_ascii_lowercase();
    let mime = document
        .mime_type
        .clone()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if filename.ends_with(".md") || filename.ends_with(".markdown") {
        "casebuilder-markdown-v1"
    } else if filename.ends_with(".html") || filename.ends_with(".htm") || mime == "text/html" {
        "casebuilder-html-text-v1"
    } else if filename.ends_with(".csv") || mime == "text/csv" {
        "casebuilder-csv-text-v1"
    } else if filename.ends_with(".pdf") || mime == "application/pdf" {
        "casebuilder-pdf-embedded-text-v1"
    } else if filename.ends_with(".docx") {
        "casebuilder-docx-text-v1"
    } else {
        "casebuilder-plain-text-v1"
    }
    .to_string()
}

fn exact_support_for_paragraph(
    matter: &MatterSummary,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
) -> (Vec<String>, Vec<EvidenceUse>) {
    let normalized = normalize_for_match(text);
    let mut fact_ids = Vec::new();
    for fact in facts {
        let candidate = normalize_for_match(&fact.statement);
        if !candidate.is_empty()
            && (normalized.contains(&candidate) || candidate.contains(&normalized))
        {
            push_unique(&mut fact_ids, fact.fact_id.clone());
        }
    }
    let mut evidence_uses = Vec::new();
    for item in evidence {
        let quote = normalize_for_match(&item.quote);
        if quote.len() >= 24 && (normalized.contains(&quote) || quote.contains(&normalized)) {
            let id = format!("{paragraph_id}:evidence-use:{}", evidence_uses.len() + 1);
            evidence_uses.push(EvidenceUse {
                id: id.clone(),
                evidence_use_id: id,
                matter_id: matter.matter_id.clone(),
                complaint_id: complaint_id.to_string(),
                target_type: "paragraph".to_string(),
                target_id: paragraph_id.to_string(),
                fact_id: fact_ids.first().cloned(),
                evidence_id: Some(item.evidence_id.clone()),
                document_id: Some(item.document_id.clone()),
                source_span_id: item
                    .source_spans
                    .first()
                    .map(|span| span.source_span_id.clone()),
                relation: "supports".to_string(),
                quote: Some(item.quote.clone()),
                status: "exact_match_needs_review".to_string(),
            });
        }
    }
    (fact_ids, evidence_uses)
}

fn normalize_for_match(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn citation_uses_for_text(
    matter_id: &str,
    complaint_id: &str,
    target_id: &str,
    target_type: &str,
    text: &str,
) -> Vec<CitationUse> {
    let mut citations = Vec::new();
    for citation in ORS_CITATION_RE
        .find_iter(text)
        .chain(ORCP_CITATION_RE.find_iter(text))
        .chain(UTCR_CITATION_RE.find_iter(text))
        .chain(SESSION_LAW_CITATION_RE.find_iter(text))
        .map(|m| m.as_str().trim().trim_end_matches('.').to_string())
    {
        if citations
            .iter()
            .any(|existing: &CitationUse| existing.citation.eq_ignore_ascii_case(&citation))
        {
            continue;
        }
        let canonical_id = canonical_id_for_citation(&citation);
        let is_external = is_external_authority_citation(&citation);
        let currentness = authority_currentness_for_citation(&citation);
        let scope_warning = if is_external {
            Some("External rule or session-law authority is source-backed but not yet part of the full ORSGraph provision corpus.".to_string())
        } else {
            None
        };
        let id = format!("{target_id}:citation-use:{}", citations.len() + 1);
        citations.push(CitationUse {
            id: id.clone(),
            citation_use_id: id,
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            citation,
            pinpoint: None,
            quote: None,
            status: if canonical_id.is_some() {
                if is_external {
                    "resolved_external".to_string()
                } else {
                    "resolved".to_string()
                }
            } else {
                "unresolved".to_string()
            },
            currentness,
            scope_warning,
            canonical_id,
        });
    }
    citations
}

fn canonical_id_for_citation(citation: &str) -> Option<String> {
    self::citation_resolver::canonical_id_for_citation(citation)
}

fn is_external_authority_citation(citation: &str) -> bool {
    let upper = citation.to_ascii_uppercase();
    upper.starts_with("ORCP")
        || upper.starts_with("OR LAWS")
        || upper.starts_with("OR. LAWS")
        || upper.starts_with("OREGON LAWS")
        || upper.starts_with("OREGON LAW")
}

fn authority_type_for_citation(citation: &str) -> &'static str {
    let upper = citation.to_ascii_uppercase();
    if upper.starts_with("ORS ") {
        "ors"
    } else if upper.starts_with("ORCP") {
        "orcp"
    } else if upper.starts_with("UTCR") {
        "utcr"
    } else if is_external_authority_citation(citation) {
        "session_law"
    } else {
        "unknown"
    }
}

fn authority_source_url_for_citation(citation: &str) -> &'static str {
    match authority_type_for_citation(citation) {
        "ors" => ORS_2025_SOURCE_URL,
        "orcp" => ORCP_2025_SOURCE_URL,
        "utcr" => UTCR_CURRENT_SOURCE_URL,
        "session_law" => ORS_2025_SOURCE_URL,
        _ => ORS_2025_SOURCE_URL,
    }
}

fn authority_edition_for_citation(citation: &str) -> &'static str {
    match authority_type_for_citation(citation) {
        "ors" => "2025 ORS",
        "orcp" => "2025 ORCP",
        "utcr" => "Current UTCR",
        "session_law" => "Oregon session law source-backed",
        _ => "Source-backed authority",
    }
}

fn authority_currentness_for_citation(citation: &str) -> String {
    match authority_type_for_citation(citation) {
        "ors" => "2025_ors_needs_review",
        "orcp" => "2025_orcp_needs_review",
        "utcr" => "current_utcr_needs_review",
        "session_law" => "source_backed_needs_review",
        _ => "source_backed_needs_review",
    }
    .to_string()
}

fn exhibit_references_for_text(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
) -> Vec<ExhibitReference> {
    let mut out = Vec::new();
    for caps in EXHIBIT_LABEL_RE.captures_iter(text) {
        let Some(label) = caps.get(0).map(|m| m.as_str().trim().to_string()) else {
            continue;
        };
        if out
            .iter()
            .any(|existing: &ExhibitReference| existing.exhibit_label.eq_ignore_ascii_case(&label))
        {
            continue;
        }
        let id = format!("{paragraph_id}:exhibit-reference:{}", out.len() + 1);
        out.push(ExhibitReference {
            id: id.clone(),
            exhibit_reference_id: id,
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            target_type: "paragraph".to_string(),
            target_id: paragraph_id.to_string(),
            exhibit_label: label,
            document_id: None,
            evidence_id: None,
            status: "missing".to_string(),
        });
    }
    out
}

fn imported_relief_requests(
    matter: &MatterSummary,
    complaint_id: &str,
    paragraphs: &[PleadingParagraph],
) -> Vec<ReliefRequest> {
    let mut relief = paragraphs
        .iter()
        .filter(|paragraph| paragraph.role == "relief")
        .take(12)
        .enumerate()
        .map(|(index, paragraph)| ReliefRequest {
            id: format!("{complaint_id}:relief:{}", index + 1),
            relief_id: format!("{complaint_id}:relief:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            category: "imported".to_string(),
            text: paragraph.text.clone(),
            amount: None,
            authority_ids: paragraph
                .citation_uses
                .iter()
                .filter_map(|citation| citation.canonical_id.clone())
                .collect(),
            supported: false,
        })
        .collect::<Vec<_>>();
    if relief.is_empty() {
        relief.push(ReliefRequest {
            id: format!("{complaint_id}:relief:general"),
            relief_id: format!("{complaint_id}:relief:general"),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            category: "general".to_string(),
            text: "Imported complaint contains no clearly parsed prayer paragraph; relief requires human review.".to_string(),
            amount: None,
            authority_ids: Vec::new(),
            supported: false,
        });
    }
    relief
}

fn match_claim_for_count(title: &str, claims: &[CaseClaim]) -> Option<String> {
    let normalized = normalize_for_match(title);
    claims
        .iter()
        .find(|claim| {
            let claim_text =
                normalize_for_match(&format!("{} {}", claim.title, claim.legal_theory));
            !claim_text.is_empty()
                && (normalized.contains(&claim_text) || claim_text.contains(&normalized))
        })
        .map(|claim| claim.claim_id.clone())
}

fn complaint_import_node_ids(complaint: &ComplaintDraft, spans: &[SourceSpan]) -> Vec<String> {
    let mut ids = vec![complaint.complaint_id.clone()];
    ids.extend(
        complaint
            .sections
            .iter()
            .map(|section| section.section_id.clone()),
    );
    ids.extend(complaint.counts.iter().map(|count| count.count_id.clone()));
    ids.extend(
        complaint
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.paragraph_id.clone()),
    );
    ids.extend(spans.iter().map(|span| span.source_span_id.clone()));
    ids
}

fn complaint_section(
    matter_id: &str,
    complaint_id: &str,
    section_type: &str,
    title: &str,
    ordinal: u64,
) -> ComplaintSection {
    let section_id = format!("{complaint_id}:section:{section_type}");
    ComplaintSection {
        id: section_id.clone(),
        section_id,
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        title: title.to_string(),
        section_type: section_type.to_string(),
        ordinal,
        paragraph_ids: Vec::new(),
        count_ids: Vec::new(),
        review_status: "needs_review".to_string(),
    }
}

fn pleading_paragraph(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    section_id: Option<String>,
    count_id: Option<String>,
    role: &str,
    text: &str,
    ordinal: u64,
    fact_ids: Vec<String>,
    evidence_ids: Vec<String>,
) -> PleadingParagraph {
    let evidence_uses = evidence_ids
        .iter()
        .enumerate()
        .map(|(index, evidence_id)| {
            let id = format!("{paragraph_id}:evidence-use:{}", index + 1);
            EvidenceUse {
                id: id.clone(),
                evidence_use_id: id,
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                target_type: "paragraph".to_string(),
                target_id: paragraph_id.to_string(),
                fact_id: fact_ids.first().cloned(),
                evidence_id: Some(evidence_id.clone()),
                document_id: None,
                source_span_id: None,
                relation: "supports".to_string(),
                quote: None,
                status: "needs_review".to_string(),
            }
        })
        .collect::<Vec<_>>();
    PleadingParagraph {
        paragraph_id: paragraph_id.to_string(),
        id: paragraph_id.to_string(),
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        section_id,
        count_id,
        number: ordinal,
        ordinal,
        display_number: Some(ordinal.to_string()),
        original_label: None,
        source_span_id: None,
        import_provenance: None,
        role: role.to_string(),
        text: text.to_string(),
        sentences: pleading_sentences(matter_id, complaint_id, paragraph_id, text, &fact_ids),
        fact_ids,
        evidence_uses,
        citation_uses: Vec::new(),
        exhibit_references: Vec::new(),
        rule_finding_ids: Vec::new(),
        locked: false,
        review_status: "needs_review".to_string(),
    }
}

fn pleading_sentences(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
    fact_ids: &[String],
) -> Vec<PleadingSentence> {
    let parts = text
        .split_terminator(['.', '?', '!'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let sentences = if parts.is_empty() {
        vec![text.trim()]
    } else {
        parts
    };
    sentences
        .into_iter()
        .enumerate()
        .map(|(index, sentence)| {
            let id = format!("{paragraph_id}:sentence:{}", index + 1);
            PleadingSentence {
                sentence_id: id.clone(),
                id,
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                paragraph_id: paragraph_id.to_string(),
                ordinal: index as u64 + 1,
                text: sentence.to_string(),
                fact_ids: fact_ids.to_vec(),
                evidence_use_ids: Vec::new(),
                citation_use_ids: Vec::new(),
                review_status: "needs_review".to_string(),
            }
        })
        .collect()
}

fn refresh_complaint_state(complaint: &mut ComplaintDraft) {
    renumber_paragraphs(&mut complaint.paragraphs);
    for section in &mut complaint.sections {
        section.paragraph_ids = complaint
            .paragraphs
            .iter()
            .filter(|paragraph| {
                paragraph.section_id.as_deref() == Some(section.section_id.as_str())
            })
            .map(|paragraph| paragraph.paragraph_id.clone())
            .collect();
        section.count_ids = complaint
            .counts
            .iter()
            .filter(|count| {
                count.paragraph_ids.iter().any(|paragraph_id| {
                    section
                        .paragraph_ids
                        .iter()
                        .any(|section_paragraph_id| section_paragraph_id == paragraph_id)
                })
            })
            .map(|count| count.count_id.clone())
            .collect();
    }
    for count in &mut complaint.counts {
        let mut weaknesses = Vec::new();
        if count.fact_ids.is_empty() {
            weaknesses.push("missing fact support".to_string());
        }
        if count.evidence_ids.is_empty() {
            weaknesses.push("missing evidence support".to_string());
        }
        if count.authorities.is_empty() {
            weaknesses.push("missing authority".to_string());
        }
        if count.relief_ids.is_empty() {
            weaknesses.push("relief not mapped".to_string());
        }
        count.health = if weaknesses.is_empty() {
            "supported_needs_review".to_string()
        } else {
            "needs_work".to_string()
        };
        count.weaknesses = weaknesses;
    }
    let open_findings = complaint
        .findings
        .iter()
        .filter(|finding| finding.status == "open")
        .cloned()
        .collect::<Vec<_>>();
    complaint.next_actions = derive_next_actions(complaint, &open_findings);
    complaint.filing_packet = derive_filing_packet(complaint);
}

fn renumber_paragraphs(paragraphs: &mut [PleadingParagraph]) {
    paragraphs.sort_by_key(|paragraph| paragraph.ordinal);
    for (index, paragraph) in paragraphs.iter_mut().enumerate() {
        paragraph.number = index as u64 + 1;
        paragraph.ordinal = index as u64 + 1;
        if paragraph.original_label.is_none() {
            paragraph.display_number = Some(paragraph.number.to_string());
        }
    }
}

fn complaint_rule_findings(complaint: &ComplaintDraft) -> Vec<RuleCheckFinding> {
    let now = now_string();
    let mut findings = Vec::new();
    let mut add = |rule_id: &str,
                   category: &str,
                   severity: &str,
                   target_type: &str,
                   target_id: &str,
                   message: &str,
                   explanation: &str,
                   suggested_fix: &str,
                   action_label: &str,
                   action_type: &str| {
        findings.push(RuleCheckFinding {
            finding_id: format!(
                "finding:{}:{}:{}",
                sanitize_path_segment(&complaint.complaint_id),
                sanitize_path_segment(rule_id),
                sanitize_path_segment(target_id)
            ),
            id: format!(
                "finding:{}:{}:{}",
                sanitize_path_segment(&complaint.complaint_id),
                sanitize_path_segment(rule_id),
                sanitize_path_segment(target_id)
            ),
            matter_id: complaint.matter_id.clone(),
            complaint_id: complaint.complaint_id.clone(),
            rule_id: rule_id.to_string(),
            category: category.to_string(),
            severity: severity.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            message: message.to_string(),
            explanation: explanation.to_string(),
            suggested_fix: suggested_fix.to_string(),
            primary_action: ComplaintAction {
                action_id: format!("action:{rule_id}:{}", sanitize_path_segment(target_id)),
                label: action_label.to_string(),
                action_type: action_type.to_string(),
                href: None,
                target_type: target_type.to_string(),
                target_id: target_id.to_string(),
            },
            status: "open".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    };

    if complaint.caption.court_name.trim().is_empty()
        || complaint.caption.court_name == "Unassigned"
    {
        add(
            "orcp-16-caption-court",
            "rules",
            "blocking",
            "caption",
            &complaint.complaint_id,
            "Caption is missing the court name.",
            "ORCP 16 requires a caption setting forth the name of the court.",
            "Add the court name to the caption.",
            "Complete caption",
            "edit_caption",
        );
    }
    if complaint.caption.plaintiff_names.is_empty() || complaint.caption.defendant_names.is_empty()
    {
        add(
            "orcp-16-complaint-title-parties",
            "rules",
            "blocking",
            "caption",
            &complaint.complaint_id,
            "Complaint title must identify all parties.",
            "ORCP 16 requires the complaint title to include the names of all parties.",
            "Confirm plaintiff and defendant names from the matter parties.",
            "Review parties",
            "edit_parties",
        );
    }
    if complaint.counts.is_empty() {
        add(
            "orcp-16-separate-counts",
            "structure",
            "blocking",
            "complaint",
            &complaint.complaint_id,
            "No separately stated count exists.",
            "ORCP 16 requires separate claims or defenses to be separately stated.",
            "Create at least one count linked to a claim or custom legal theory.",
            "Create count",
            "create_count",
        );
    }
    if complaint.paragraphs.is_empty() {
        add(
            "orcp-16-numbered-paragraphs",
            "structure",
            "blocking",
            "complaint",
            &complaint.complaint_id,
            "No numbered pleading paragraphs exist.",
            "ORCP 16 requires plain and concise statements in consecutively numbered paragraphs.",
            "Add numbered pleading paragraphs.",
            "Add paragraph",
            "create_paragraph",
        );
    }
    for (index, paragraph) in complaint.paragraphs.iter().enumerate() {
        if paragraph.number != index as u64 + 1 {
            add(
                "orcp-16-consecutive-numbering",
                "structure",
                "serious",
                "paragraph",
                &paragraph.paragraph_id,
                "Paragraph numbering is not consecutive.",
                "ORCP 16 expects paragraphs to be consecutively numbered throughout the pleading.",
                "Run renumbering to restore consecutive Arabic numerals.",
                "Renumber paragraphs",
                "renumber",
            );
        }
        if paragraph.role == "factual_allegation"
            && paragraph.fact_ids.is_empty()
            && paragraph.evidence_uses.is_empty()
        {
            add(
                "cb-support-factual-allegation",
                "evidence",
                "warning",
                "paragraph",
                &paragraph.paragraph_id,
                "Factual allegation has no linked fact or evidence.",
                "Complaint Editor flags unsupported factual allegations for human review.",
                "Link a fact, evidence record, document span, or mark the paragraph reviewed.",
                "Link evidence",
                "link_evidence",
            );
        }
        if paragraph.role == "factual_allegation" && paragraph.text.split_whitespace().count() > 90
        {
            add(
                "orcp-18-plain-concise-ultimate-facts",
                "rules",
                "warning",
                "paragraph",
                &paragraph.paragraph_id,
                "Factual paragraph may be too long for plain and concise pleading.",
                "ORCP 18 calls for a plain and concise statement of ultimate facts without unnecessary repetition.",
                "Split or tighten the paragraph after human review.",
                "Split paragraph",
                "split_paragraph",
            );
        }
        for citation in &paragraph.citation_uses {
            if citation.status != "resolved" {
                add(
                    "cb-citation-unresolved",
                    "citations",
                    "warning",
                    "citation",
                    &citation.citation_use_id,
                    "Citation is unresolved or needs review.",
                    "Citation uses should be linked to ORSGraph authority when possible and reviewed for currentness.",
                    "Resolve the citation against ORSGraph authority.",
                    "Resolve citation",
                    "resolve_citation",
                );
            }
        }
        for exhibit in &paragraph.exhibit_references {
            if exhibit.status == "missing" {
                add(
                    "cb-exhibit-missing",
                    "exhibits",
                    "warning",
                    "exhibit",
                    &exhibit.exhibit_reference_id,
                    "Exhibit reference is not linked to an exhibit document or evidence record.",
                    "Exhibit labels should remain stable and link to the supporting record.",
                    "Attach the referenced exhibit.",
                    "Attach exhibit",
                    "attach_exhibit",
                );
            }
        }
    }
    if complaint.relief.is_empty()
        || complaint
            .relief
            .iter()
            .all(|relief| relief.text.trim().is_empty())
    {
        add(
            "orcp-18-demand-relief",
            "relief",
            "blocking",
            "relief",
            &complaint.complaint_id,
            "Demand for relief is missing.",
            "ORCP 18 requires a demand of the relief claimed, including amounts when money or damages are demanded.",
            "Add requested relief and any amount after human review.",
            "Add relief",
            "edit_relief",
        );
    }
    if complaint.signature.name.trim().is_empty()
        || complaint.signature.address.trim().is_empty()
        || complaint.signature.email.trim().is_empty()
    {
        add(
            "orcp-17-signature-contact",
            "rules",
            "serious",
            "signature",
            &complaint.complaint_id,
            "Signature/contact block is incomplete.",
            "Pleadings require a human-reviewed signature and contact block before filing or service.",
            "Complete the signature block.",
            "Complete signature",
            "edit_signature",
        );
    }
    if !complaint.formatting_profile.double_spaced {
        add(
            "utcr-2-010-double-spacing",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Formatting profile is not double-spaced.",
            "UTCR 2.010 provides that pleadings must be double-spaced unless another rule applies.",
            "Set the complaint formatting profile to double-spaced.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if !complaint.formatting_profile.line_numbers {
        add(
            "utcr-2-010-numbered-lines",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Formatting profile does not include numbered lines.",
            "UTCR 2.010 provides that pleadings must be prepared with numbered lines unless another rule applies.",
            "Enable numbered lines.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if complaint.formatting_profile.first_page_top_blank_inches < 2.0 {
        add(
            "utcr-2-010-first-page-blank",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "First-page top blank area is less than two inches.",
            "UTCR 2.010 calls for two inches blank at the top of the first page of pleadings or similar documents.",
            "Set the first-page top blank area to two inches.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if complaint.formatting_profile.margin_left_inches < 1.0
        || complaint.formatting_profile.margin_right_inches < 1.0
    {
        add(
            "utcr-2-010-side-margins",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Side margins are less than one inch.",
            "UTCR 2.010 calls for one-inch side margins except where a different form applies.",
            "Set side margins to at least one inch.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    findings
}

fn derive_next_actions(
    complaint: &ComplaintDraft,
    open_findings: &[RuleCheckFinding],
) -> Vec<ComplaintNextAction> {
    let mut actions = Vec::new();
    for finding in open_findings.iter().take(5) {
        actions.push(ComplaintNextAction {
            action_id: format!("next:{}", finding.finding_id),
            priority: finding.severity.clone(),
            label: finding.primary_action.label.clone(),
            detail: finding.message.clone(),
            action_type: finding.primary_action.action_type.clone(),
            target_type: finding.target_type.clone(),
            target_id: finding.target_id.clone(),
            href: finding.primary_action.href.clone(),
        });
    }
    if actions.is_empty() {
        if complaint.paragraphs.iter().any(|paragraph| {
            paragraph.role == "factual_allegation"
                && paragraph.fact_ids.is_empty()
                && paragraph.evidence_uses.is_empty()
        }) {
            actions.push(ComplaintNextAction {
                action_id: "next:link-evidence".to_string(),
                priority: "warning".to_string(),
                label: "Link evidence".to_string(),
                detail: "Some factual allegations still need support links.".to_string(),
                action_type: "link_evidence".to_string(),
                target_type: "complaint".to_string(),
                target_id: complaint.complaint_id.clone(),
                href: None,
            });
        } else {
            actions.push(ComplaintNextAction {
                action_id: "next:run-qc".to_string(),
                priority: "info".to_string(),
                label: "Run QC".to_string(),
                detail: "Run deterministic complaint QC before preview or export.".to_string(),
                action_type: "run_qc".to_string(),
                target_type: "complaint".to_string(),
                target_id: complaint.complaint_id.clone(),
                href: None,
            });
        }
    }
    actions
}

fn derive_filing_packet(complaint: &ComplaintDraft) -> FilingPacket {
    let mut items = Vec::new();
    items.push(FilingPacketItem {
        item_id: format!("{}:packet:item:complaint", complaint.complaint_id),
        label: "Complaint".to_string(),
        item_type: "complaint_pdf".to_string(),
        status: if complaint
            .export_artifacts
            .iter()
            .any(|artifact| artifact.format == "pdf")
        {
            "generated_review_needed".to_string()
        } else {
            "missing".to_string()
        },
        artifact_id: complaint
            .export_artifacts
            .iter()
            .find(|artifact| artifact.format == "pdf")
            .map(|artifact| artifact.artifact_id.clone()),
        warning: Some("Review before filing; no e-filing is performed.".to_string()),
    });
    items.push(FilingPacketItem {
        item_id: format!("{}:packet:item:certificate", complaint.complaint_id),
        label: "Certificate of service".to_string(),
        item_type: "certificate".to_string(),
        status: complaint
            .certificate_of_service
            .as_ref()
            .map(|certificate| certificate.review_status.clone())
            .unwrap_or_else(|| "missing".to_string()),
        artifact_id: None,
        warning: Some("Service details require human review.".to_string()),
    });
    for exhibit in complaint
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.exhibit_references.iter())
    {
        items.push(FilingPacketItem {
            item_id: format!(
                "{}:packet:item:{}",
                complaint.complaint_id, exhibit.exhibit_label
            ),
            label: format!("Exhibit {}", exhibit.exhibit_label),
            item_type: "exhibit".to_string(),
            status: exhibit.status.clone(),
            artifact_id: None,
            warning: if exhibit.status == "missing" {
                Some("Referenced exhibit is not attached.".to_string())
            } else {
                None
            },
        });
    }
    let warnings = items
        .iter()
        .filter_map(|item| item.warning.clone())
        .collect::<Vec<_>>();
    FilingPacket {
        packet_id: format!("{}:packet:filing", complaint.complaint_id),
        matter_id: complaint.matter_id.clone(),
        complaint_id: complaint.complaint_id.clone(),
        status: "review_needed".to_string(),
        items,
        warnings,
    }
}

fn render_complaint_preview(complaint: &ComplaintDraft) -> ComplaintPreviewResponse {
    let mut html = String::new();
    html.push_str("<article class=\"court-paper review-needed\">");
    html.push_str("<header class=\"caption\">");
    html.push_str(&format!(
        "<p>{}</p>",
        escape_html(&complaint.caption.court_name)
    ));
    html.push_str(&format!(
        "<h1>{}</h1>",
        escape_html(&complaint.caption.document_title)
    ));
    html.push_str(&format!(
        "<p>{} v. {}</p>",
        escape_html(&complaint.caption.plaintiff_names.join(", ")),
        escape_html(&complaint.caption.defendant_names.join(", "))
    ));
    html.push_str("</header>");
    for paragraph in &complaint.paragraphs {
        html.push_str(&format!(
            "<p id=\"{}\"><span class=\"line-number\">{}</span> {}</p>",
            escape_html(&paragraph.paragraph_id),
            paragraph.number,
            escape_html(&paragraph.text)
        ));
    }
    html.push_str("<section class=\"signature\">");
    html.push_str(&format!(
        "<p>{}</p><p>{}</p><p>{}</p>",
        escape_html(&complaint.signature.name),
        escape_html(&complaint.signature.address),
        escape_html(&complaint.signature.email)
    ));
    html.push_str("</section>");
    html.push_str("</article>");
    let plain_text = complaint_plain_text(complaint);
    let page_count = (plain_text.lines().count() as u64 / 28).max(1);
    ComplaintPreviewResponse {
        complaint_id: complaint.complaint_id.clone(),
        matter_id: complaint.matter_id.clone(),
        html,
        plain_text,
        page_count,
        warnings: export_warnings(complaint, "preview"),
        generated_at: now_string(),
        review_label: "Review needed; not legal advice or filing-ready status.".to_string(),
    }
}

fn export_complaint_content(
    complaint: &ComplaintDraft,
    format: &str,
    include_exhibits: bool,
    include_qc_report: bool,
) -> ApiResult<String> {
    let mut text = match format {
        "html" => render_complaint_preview(complaint).html,
        "json" => to_payload(complaint)?,
        "markdown" => complaint_markdown(complaint),
        "text" | "plain_text" => complaint_plain_text(complaint),
        "pdf" => format!(
            "PDF skeleton for human review\n\n{}",
            complaint_plain_text(complaint)
        ),
        "docx" => format!(
            "DOCX skeleton for human review\n\n{}",
            complaint_plain_text(complaint)
        ),
        _ => complaint_plain_text(complaint),
    };
    if include_exhibits {
        let exhibits = complaint
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.exhibit_references.iter())
            .map(|exhibit| format!("Exhibit {} - {}", exhibit.exhibit_label, exhibit.status))
            .collect::<Vec<_>>();
        if !exhibits.is_empty() {
            text.push_str("\n\nEXHIBITS\n");
            text.push_str(&exhibits.join("\n"));
        }
    }
    if include_qc_report {
        text.push_str("\n\nQC REVIEW ITEMS\n");
        if complaint.findings.is_empty() {
            text.push_str("No persisted findings. Run complaint QC before relying on this export.");
        } else {
            for finding in &complaint.findings {
                text.push_str(&format!(
                    "\n- [{}] {}: {}",
                    finding.severity, finding.status, finding.message
                ));
            }
        }
    }
    Ok(text)
}

fn complaint_plain_text(complaint: &ComplaintDraft) -> String {
    let mut lines = Vec::new();
    lines.push(complaint.caption.court_name.clone());
    lines.push(format!(
        "{} v. {}",
        complaint.caption.plaintiff_names.join(", "),
        complaint.caption.defendant_names.join(", ")
    ));
    lines.push(complaint.caption.document_title.clone());
    lines.push(String::new());
    for paragraph in &complaint.paragraphs {
        lines.push(format!("{}. {}", paragraph.number, paragraph.text));
    }
    lines.push(String::new());
    lines.push("PRAYER FOR RELIEF".to_string());
    for relief in &complaint.relief {
        lines.push(format!("- {}", relief.text));
    }
    lines.push(String::new());
    lines.push("Review needed; not legal advice or filing-ready status.".to_string());
    lines.join("\n")
}

fn complaint_markdown(complaint: &ComplaintDraft) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {}", complaint.caption.document_title));
    lines.push(format!(
        "**{} v. {}**",
        complaint.caption.plaintiff_names.join(", "),
        complaint.caption.defendant_names.join(", ")
    ));
    for section in &complaint.sections {
        lines.push(format!("\n## {}", section.title));
        for paragraph in complaint.paragraphs.iter().filter(|paragraph| {
            paragraph.section_id.as_deref() == Some(section.section_id.as_str())
        }) {
            lines.push(format!("{}. {}", paragraph.number, paragraph.text));
        }
    }
    lines.push("\n> Review needed; not legal advice or filing-ready status.".to_string());
    lines.join("\n")
}

fn export_warnings(complaint: &ComplaintDraft, format: &str) -> Vec<String> {
    let mut warnings =
        vec!["Review needed; generated checks and exports are not legal advice.".to_string()];
    if complaint
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        warnings.push("Open QC findings remain.".to_string());
    }
    if matches!(format, "pdf" | "docx") {
        warnings.push(
            "PDF/DOCX output is a deterministic skeleton until the dedicated renderer is enabled."
                .to_string(),
        );
    }
    warnings
}

fn export_mime_type(format: &str) -> &'static str {
    match format {
        "html" => "text/html",
        "json" => "application/json",
        "markdown" => "text/markdown",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "text/plain",
    }
}

fn default_formatting_profile() -> FormattingProfile {
    FormattingProfile {
        profile_id: "oregon-circuit-civil-complaint".to_string(),
        name: "Oregon Circuit Civil Complaint".to_string(),
        jurisdiction: "Oregon".to_string(),
        line_numbers: true,
        double_spaced: true,
        first_page_top_blank_inches: 2.0,
        margin_top_inches: 1.0,
        margin_bottom_inches: 1.0,
        margin_left_inches: 1.0,
        margin_right_inches: 1.0,
        font_family: "Times New Roman".to_string(),
        font_size_pt: 12,
    }
}

fn oregon_civil_complaint_rule_pack() -> RulePack {
    RulePack {
        rule_pack_id: "oregon-circuit-civil-complaint-orcp-utcr".to_string(),
        name: "Oregon Circuit Civil Complaint - ORCP + UTCR".to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2024-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![
            rule_definition(
                "orcp-16-caption-court",
                "ORCP 16 A",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/",
                "blocking",
                "caption",
                "rules",
                "Caption must include court name.",
                "ORCP 16 describes caption requirements.",
                "Complete the court name.",
                false,
            ),
            rule_definition(
                "orcp-16-complaint-title-parties",
                "ORCP 16 A",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/",
                "blocking",
                "caption",
                "rules",
                "Complaint title must include all parties.",
                "ORCP 16 distinguishes complaint title requirements from later pleadings.",
                "Confirm all party names.",
                false,
            ),
            rule_definition(
                "orcp-16-numbered-paragraphs",
                "ORCP 16 C",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/",
                "blocking",
                "paragraph",
                "structure",
                "Paragraphs must be consecutively numbered.",
                "ORCP 16 calls for consecutively numbered paragraphs.",
                "Run renumbering.",
                true,
            ),
            rule_definition(
                "orcp-16-separate-counts",
                "ORCP 16 C",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/",
                "blocking",
                "count",
                "structure",
                "Separate claims must be separately stated.",
                "ORCP 16 requires separate claims or defenses to be separately stated.",
                "Create separate counts.",
                false,
            ),
            rule_definition(
                "orcp-18-plain-concise-ultimate-facts",
                "ORCP 18 A",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-18-claims-for-relief/",
                "warning",
                "paragraph",
                "rules",
                "Claims should plead plain and concise ultimate facts.",
                "ORCP 18 calls for a plain and concise statement of ultimate facts.",
                "Tighten or split long factual allegations.",
                false,
            ),
            rule_definition(
                "orcp-18-demand-relief",
                "ORCP 18 B",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-18-claims-for-relief/",
                "blocking",
                "relief",
                "relief",
                "Demand for relief is required.",
                "ORCP 18 requires a demand for relief and amount when money or damages are demanded.",
                "Add requested relief.",
                false,
            ),
            rule_definition(
                "orcp-17-signature-contact",
                "ORCP 17",
                "https://oregon.public.law/rules-of-civil-procedure/orcp-17-signing-of-pleadings-motions-and-other-papers-sanctions/",
                "serious",
                "signature",
                "rules",
                "Signature and contact block require review.",
                "Pleadings must be signed by a responsible person subject to Rule 17 obligations.",
                "Complete signature details.",
                false,
            ),
            rule_definition(
                "utcr-2-010-double-spacing",
                "UTCR 2.010(4)(a)",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "serious",
                "formatting",
                "formatting",
                "Pleadings should be double-spaced.",
                "UTCR 2.010 includes spacing standards for pleadings.",
                "Enable double spacing.",
                true,
            ),
            rule_definition(
                "utcr-2-010-numbered-lines",
                "UTCR 2.010(4)(a)",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "serious",
                "formatting",
                "formatting",
                "Pleadings should have numbered lines.",
                "UTCR 2.010 includes numbered-line standards for pleadings.",
                "Enable numbered lines.",
                true,
            ),
            rule_definition(
                "utcr-2-010-first-page-blank",
                "UTCR 2.010(4)(c)",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "serious",
                "formatting",
                "formatting",
                "First page top blank area should be two inches.",
                "UTCR 2.010 includes a first-page blank area standard.",
                "Set first-page blank area to two inches.",
                true,
            ),
            rule_definition(
                "utcr-2-010-side-margins",
                "UTCR 2.010(4)(d)",
                "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf",
                "serious",
                "formatting",
                "formatting",
                "Side margins should be at least one inch.",
                "UTCR 2.010 includes side-margin standards.",
                "Set one-inch side margins.",
                true,
            ),
        ],
    }
}

fn rule_definition(
    rule_id: &str,
    source_citation: &str,
    source_url: &str,
    severity: &str,
    target_type: &str,
    category: &str,
    message: &str,
    explanation: &str,
    suggested_fix: &str,
    auto_fix_available: bool,
) -> RuleDefinition {
    RuleDefinition {
        rule_id: rule_id.to_string(),
        source_citation: source_citation.to_string(),
        source_url: source_url.to_string(),
        severity: severity.to_string(),
        target_type: target_type.to_string(),
        category: category.to_string(),
        message: message.to_string(),
        explanation: explanation.to_string(),
        suggested_fix: suggested_fix.to_string(),
        auto_fix_available,
    }
}

fn default_ai_commands() -> Vec<ComplaintAiCommandState> {
    [
        ("draft_factual_background", "Draft factual background"),
        ("draft_count", "Draft count"),
        ("rewrite_ultimate_facts", "Rewrite as ultimate facts"),
        ("split_paragraph", "Split paragraph"),
        ("make_concise", "Make concise"),
        ("find_missing_evidence", "Find missing evidence"),
        ("find_missing_authority", "Find missing authority"),
        ("generate_prayer", "Generate prayer"),
        ("generate_exhibit_list", "Generate exhibit list"),
        ("generate_certificate", "Generate certificate"),
        ("fact_check", "Fact-check"),
        ("citation_check", "Citation-check"),
    ]
    .into_iter()
    .map(|(command_id, label)| ComplaintAiCommandState {
        command_id: command_id.to_string(),
        label: label.to_string(),
        status: "template_available".to_string(),
        mode: "provider_free".to_string(),
        description: "Provider-free command state; no unsupported text is inserted automatically."
            .to_string(),
        last_message: None,
    })
    .collect()
}

fn complaint_event(
    matter_id: &str,
    complaint_id: &str,
    event_type: &str,
    target_type: &str,
    target_id: &str,
    summary: &str,
) -> ComplaintHistoryEvent {
    let id = format!(
        "event:{}:{}:{}",
        sanitize_path_segment(complaint_id),
        sanitize_path_segment(event_type),
        now_secs()
    );
    ComplaintHistoryEvent {
        id: id.clone(),
        event_id: id,
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        event_type: event_type.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        summary: summary.to_string(),
        timestamp: now_string(),
    }
}

fn json_value<T: serde::Serialize>(value: &T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn json_hash(value: &serde_json::Value) -> ApiResult<String> {
    let payload =
        serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(sha256_hex(payload.as_bytes()))
}

fn hash_json<T: serde::Serialize>(value: &T) -> ApiResult<String> {
    json_hash(&json_value(value)?)
}

fn version_change_state_summary(
    value: Option<serde_json::Value>,
) -> ApiResult<(Option<String>, Option<serde_json::Value>)> {
    let Some(value) = value else {
        return Ok((None, None));
    };
    let state_hash = json_hash(&value)?;
    let size_bytes = serde_json::to_vec(&value)
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .len() as u64;
    Ok((
        Some(state_hash.clone()),
        Some(serde_json::json!({
            "state_hash": state_hash,
            "size_bytes": size_bytes,
            "state_storage": "version_snapshot",
            "inline_payload": false,
        })),
    ))
}

fn work_product_hashes(product: &WorkProduct) -> ApiResult<WorkProductHashes> {
    let document_state = serde_json::json!({
        "title": product.title,
        "product_type": product.product_type,
        "profile": product.profile,
        "document_ast": product.document_ast,
    });
    let support_state = serde_json::json!({
        "blocks": flatten_work_product_blocks(&product.document_ast.blocks).iter().map(|block| serde_json::json!({
            "block_id": block.block_id,
            "links": block.links,
            "citations": block.citations,
            "exhibits": block.exhibits,
        })).collect::<Vec<_>>(),
        "links": product.document_ast.links,
        "citations": product.document_ast.citations,
        "exhibits": product.document_ast.exhibits,
    });
    let qc_state = serde_json::json!({
        "findings": product.document_ast.rule_findings,
        "review_status": product.review_status,
    });
    Ok(WorkProductHashes {
        document_hash: json_hash(&document_state)?,
        support_graph_hash: json_hash(&support_state)?,
        qc_state_hash: json_hash(&qc_state)?,
        formatting_hash: hash_json(&product.formatting_profile)?,
    })
}

fn snapshot_manifest_for_product(
    matter_id: &str,
    snapshot_id: &str,
    product: &WorkProduct,
    created_at: &str,
) -> ApiResult<(SnapshotManifest, Vec<SnapshotEntityState>)> {
    let manifest_id = format!("{snapshot_id}:manifest");
    let mut states = Vec::new();
    push_entity_state(
        &mut states,
        &manifest_id,
        snapshot_id,
        matter_id,
        &product.work_product_id,
        "work_product",
        &product.work_product_id,
        json_value(product)?,
    )?;
    push_entity_state(
        &mut states,
        &manifest_id,
        snapshot_id,
        matter_id,
        &product.work_product_id,
        "document_ast",
        &product.document_ast.document_id,
        json_value(&product.document_ast)?,
    )?;
    for block in flatten_work_product_blocks(&product.document_ast.blocks) {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "block",
            &block.block_id,
            json_value(&block)?,
        )?;
    }
    for link in &product.document_ast.links {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "support_use",
            &link.link_id,
            json_value(link)?,
        )?;
    }
    for citation in &product.document_ast.citations {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "citation",
            &citation.citation_use_id,
            json_value(citation)?,
        )?;
    }
    for exhibit in &product.document_ast.exhibits {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "exhibit_reference",
            &exhibit.exhibit_reference_id,
            json_value(exhibit)?,
        )?;
    }
    for finding in &product.document_ast.rule_findings {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "rule_finding",
            &finding.finding_id,
            json_value(finding)?,
        )?;
    }
    for artifact in &product.artifacts {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "export_artifact",
            &artifact.artifact_id,
            json_value(artifact)?,
        )?;
    }
    let manifest_hash = snapshot_manifest_hash_for_states(&states)?;
    Ok((
        SnapshotManifest {
            id: manifest_id.clone(),
            manifest_id,
            snapshot_id: snapshot_id.to_string(),
            matter_id: matter_id.to_string(),
            subject_id: product.work_product_id.clone(),
            manifest_hash,
            entry_count: states.len() as u64,
            storage_ref: None,
            created_at: created_at.to_string(),
        },
        states,
    ))
}

fn snapshot_manifest_hash_for_states(states: &[SnapshotEntityState]) -> ApiResult<String> {
    hash_json(
        &states
            .iter()
            .map(|state| (&state.entity_type, &state.entity_id, &state.entity_hash))
            .collect::<Vec<_>>(),
    )
}

fn push_entity_state(
    states: &mut Vec<SnapshotEntityState>,
    manifest_id: &str,
    snapshot_id: &str,
    matter_id: &str,
    subject_id: &str,
    entity_type: &str,
    entity_id: &str,
    state: serde_json::Value,
) -> ApiResult<()> {
    let entity_hash = json_hash(&state)?;
    let entity_state_id = format!(
        "{snapshot_id}:state:{}:{}",
        sanitize_path_segment(entity_type),
        sanitize_path_segment(entity_id)
    );
    states.push(SnapshotEntityState {
        id: entity_state_id.clone(),
        entity_state_id,
        manifest_id: manifest_id.to_string(),
        snapshot_id: snapshot_id.to_string(),
        matter_id: matter_id.to_string(),
        subject_id: subject_id.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        entity_hash,
        state_ref: None,
        state_inline: Some(state),
    });
    Ok(())
}

fn diff_work_product_blocks(from: &WorkProduct, to: &WorkProduct) -> Vec<VersionTextDiff> {
    let mut diffs = Vec::new();
    let from_blocks = flatten_work_product_blocks(&from.document_ast.blocks);
    let to_blocks = flatten_work_product_blocks(&to.document_ast.blocks);
    for before in &from_blocks {
        match to_blocks
            .iter()
            .find(|block| block.block_id == before.block_id)
        {
            Some(after) => {
                let status = if before.text == after.text && before.title == after.title {
                    "unchanged"
                } else {
                    "modified"
                };
                diffs.push(VersionTextDiff {
                    target_type: "block".to_string(),
                    target_id: before.block_id.clone(),
                    title: after.title.clone(),
                    status: status.to_string(),
                    before: Some(before.text.clone()),
                    after: Some(after.text.clone()),
                });
            }
            None => diffs.push(VersionTextDiff {
                target_type: "block".to_string(),
                target_id: before.block_id.clone(),
                title: before.title.clone(),
                status: "removed".to_string(),
                before: Some(before.text.clone()),
                after: None,
            }),
        }
    }
    for after in &to_blocks {
        if !from_blocks
            .iter()
            .any(|block| block.block_id == after.block_id)
        {
            diffs.push(VersionTextDiff {
                target_type: "block".to_string(),
                target_id: after.block_id.clone(),
                title: after.title.clone(),
                status: "added".to_string(),
                before: None,
                after: Some(after.text.clone()),
            });
        }
    }
    diffs
}

fn normalize_compare_layers(layers: Vec<String>) -> Vec<String> {
    let defaults = [
        "text",
        "support",
        "citations",
        "exhibits",
        "rule_findings",
        "formatting",
        "exports",
    ];
    let requested = if layers.is_empty() {
        defaults.iter().map(|layer| layer.to_string()).collect()
    } else {
        layers
    };
    let mut normalized = Vec::new();
    for layer in requested {
        let layer = layer.trim().to_ascii_lowercase();
        let expanded = match layer.as_str() {
            "all" | "legal" | "ast" => defaults.to_vec(),
            "text" | "blocks" => vec!["text"],
            "support" | "links" | "support_links" => vec!["support"],
            "citation" | "citations" => vec!["citations"],
            "exhibit" | "exhibits" => vec!["exhibits"],
            "rule_finding" | "rule_findings" | "qc" | "findings" | "rules" => {
                vec!["rule_findings"]
            }
            "format" | "formatting" => vec!["formatting"],
            "export" | "exports" | "artifacts" => vec!["exports"],
            _ => Vec::new(),
        };
        for value in expanded {
            if !normalized.iter().any(|existing| existing == value) {
                normalized.push(value.to_string());
            }
        }
    }
    if normalized.is_empty() {
        normalized = defaults.iter().map(|layer| layer.to_string()).collect();
    }
    normalized
}

fn layer_change_count(diffs: &[VersionLayerDiff], layer: &str) -> u64 {
    diffs
        .iter()
        .filter(|diff| diff.layer == layer && diff.status != "unchanged")
        .count() as u64
}

fn restore_work_product_scope(
    current: &WorkProduct,
    snapshot: &WorkProduct,
    scope: &str,
    target_ids: &[String],
) -> ApiResult<(WorkProduct, Vec<String>)> {
    let normalized_scope = scope.trim().to_ascii_lowercase();
    let mut restored = current.clone();
    let mut warnings = Vec::new();
    if !target_ids.is_empty() {
        warnings.push(
            "Targeted restore will only apply matching targets in the selected scope.".to_string(),
        );
    }
    match normalized_scope.as_str() {
        "all" | "work_product" | "complaint" => {
            warnings.push(
                "This will replace the current work product with the selected snapshot."
                    .to_string(),
            );
            restored = snapshot.clone();
        }
        "text" | "blocks" | "block" | "paragraph" => {
            restore_blocks_from_snapshot(&mut restored, snapshot, target_ids, &mut warnings)?;
        }
        "metadata" | "document_metadata" => {
            restored.title = snapshot.title.clone();
            restored.status = snapshot.status.clone();
            restored.review_status = snapshot.review_status.clone();
            restored.setup_stage = snapshot.setup_stage.clone();
            restored.document_ast.metadata = snapshot.document_ast.metadata.clone();
            restored.document_ast.title = snapshot.document_ast.title.clone();
        }
        "support" | "links" | "support_links" => {
            restore_links_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "citations" | "citation" => {
            restore_citations_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "exhibits" | "exhibit" => {
            restore_exhibits_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "rule_findings" | "rule_finding" | "qc" => {
            restore_rule_findings_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "formatting" | "format" => {
            restored.formatting_profile = snapshot.formatting_profile.clone();
        }
        "exports" | "export" | "export_state" | "artifacts" => {
            merge_export_state_from_snapshot(&mut restored, snapshot);
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported restore scope.".to_string(),
            ));
        }
    }
    refresh_work_product_state(&mut restored);
    Ok((restored, warnings))
}

fn restore_blocks_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
    warnings: &mut Vec<String>,
) -> ApiResult<()> {
    if target_ids.is_empty() {
        restored.document_ast.blocks = snapshot.document_ast.blocks.clone();
        return Ok(());
    }
    for target_id in target_ids {
        let snapshot_block = find_ast_block(&snapshot.document_ast.blocks, target_id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound("Snapshot block not found".to_string()))?;
        let current_block = find_ast_block(&restored.document_ast.blocks, target_id).cloned();
        match replace_ast_block(&mut restored.document_ast.blocks, &snapshot_block) {
            true => {
                if let Some(current_block) = current_block.as_ref() {
                    if current_block.evidence_ids.len() > snapshot_block.evidence_ids.len()
                        || current_block.links.len() > snapshot_block.links.len()
                    {
                        warnings.push(
                            "Restoring a block may remove current support references on that block."
                                .to_string(),
                        );
                    }
                }
            }
            false => restored.document_ast.blocks.push(snapshot_block),
        }
    }
    Ok(())
}

fn restore_links_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.links = snapshot.document_ast.links.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "links",
            None,
        );
        return;
    }
    restored
        .document_ast
        .links
        .retain(|link| !target_ids.contains(&link.source_block_id));
    restored.document_ast.links.extend(
        snapshot
            .document_ast
            .links
            .iter()
            .filter(|link| target_ids.contains(&link.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "links",
        Some(target_ids),
    );
}

fn restore_citations_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.citations = snapshot.document_ast.citations.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "citations",
            None,
        );
        return;
    }
    restored
        .document_ast
        .citations
        .retain(|citation| !target_ids.contains(&citation.source_block_id));
    restored.document_ast.citations.extend(
        snapshot
            .document_ast
            .citations
            .iter()
            .filter(|citation| target_ids.contains(&citation.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "citations",
        Some(target_ids),
    );
}

fn restore_exhibits_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.exhibits = snapshot.document_ast.exhibits.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "exhibits",
            None,
        );
        return;
    }
    restored
        .document_ast
        .exhibits
        .retain(|exhibit| !target_ids.contains(&exhibit.source_block_id));
    restored.document_ast.exhibits.extend(
        snapshot
            .document_ast
            .exhibits
            .iter()
            .filter(|exhibit| target_ids.contains(&exhibit.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "exhibits",
        Some(target_ids),
    );
}

fn restore_rule_findings_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.rule_findings = snapshot.document_ast.rule_findings.clone();
        restored.findings = snapshot.findings.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "rule_findings",
            None,
        );
        return;
    }
    restored
        .document_ast
        .rule_findings
        .retain(|finding| !target_ids.contains(&finding.target_id));
    restored.document_ast.rule_findings.extend(
        snapshot
            .document_ast
            .rule_findings
            .iter()
            .filter(|finding| target_ids.contains(&finding.target_id))
            .cloned(),
    );
    restored.findings = restored.document_ast.rule_findings.clone();
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "rule_findings",
        Some(target_ids),
    );
}

fn merge_export_state_from_snapshot(restored: &mut WorkProduct, snapshot: &WorkProduct) {
    for artifact in &snapshot.artifacts {
        if !restored
            .artifacts
            .iter()
            .any(|current| current.artifact_id == artifact.artifact_id)
        {
            restored.artifacts.push(artifact.clone());
        }
    }
}

fn find_ast_block<'a>(
    blocks: &'a [WorkProductBlock],
    block_id: &str,
) -> Option<&'a WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block(&block.children, block_id) {
            return Some(found);
        }
    }
    None
}

fn replace_ast_block(blocks: &mut [WorkProductBlock], replacement: &WorkProductBlock) -> bool {
    for block in blocks {
        if block.block_id == replacement.block_id {
            *block = replacement.clone();
            return true;
        }
        if replace_ast_block(&mut block.children, replacement) {
            return true;
        }
    }
    false
}

fn copy_block_refs(
    blocks: &mut [WorkProductBlock],
    snapshot_blocks: &[WorkProductBlock],
    ref_kind: &str,
    only_block_ids: Option<&[String]>,
) {
    for block in blocks {
        if only_block_ids
            .map(|ids| ids.contains(&block.block_id))
            .unwrap_or(true)
        {
            if let Some(snapshot_block) = find_ast_block(snapshot_blocks, &block.block_id) {
                match ref_kind {
                    "links" => block.links = snapshot_block.links.clone(),
                    "citations" => block.citations = snapshot_block.citations.clone(),
                    "exhibits" => block.exhibits = snapshot_block.exhibits.clone(),
                    "rule_findings" => {
                        block.rule_finding_ids = snapshot_block.rule_finding_ids.clone()
                    }
                    _ => {}
                }
            }
        }
        copy_block_refs(
            &mut block.children,
            snapshot_blocks,
            ref_kind,
            only_block_ids,
        );
    }
}

fn work_product_facade_change_inputs(
    before: Option<&WorkProduct>,
    after: &WorkProduct,
) -> Vec<VersionChangeInput> {
    let Some(before) = before else {
        return vec![VersionChangeInput {
            target_type: "work_product".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "create".to_string(),
            before: None,
            after: json_value(after).ok(),
            summary: "Complaint-profile work product created.".to_string(),
            legal_impact: LegalImpactSummary::default(),
            ai_audit_id: None,
        }];
    };

    let mut changes = Vec::new();
    if before.title != after.title
        || before.status != after.status
        || before.review_status != after.review_status
        || before.setup_stage != after.setup_stage
        || before.formatting_profile.profile_id != after.formatting_profile.profile_id
    {
        changes.push(VersionChangeInput {
            target_type: "work_product".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "update".to_string(),
            before: json_value(before).ok(),
            after: json_value(after).ok(),
            summary: "Complaint metadata changed.".to_string(),
            legal_impact: LegalImpactSummary::default(),
            ai_audit_id: None,
        });
    }

    for after_block in &after.blocks {
        let target_type = if after_block.block_type == "paragraph" {
            "paragraph"
        } else {
            "block"
        };
        match before
            .blocks
            .iter()
            .find(|block| block.block_id == after_block.block_id)
        {
            Some(before_block) => {
                if before_block.text != after_block.text
                    || before_block.title != after_block.title
                    || before_block.ordinal != after_block.ordinal
                    || before_block.parent_block_id != after_block.parent_block_id
                    || before_block.fact_ids != after_block.fact_ids
                    || before_block.evidence_ids != after_block.evidence_ids
                    || json_value(&before_block.authorities).ok()
                        != json_value(&after_block.authorities).ok()
                    || before_block.review_status != after_block.review_status
                {
                    let before_authority_ids = before_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>();
                    let after_authority_ids = after_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>();
                    changes.push(VersionChangeInput {
                        target_type: target_type.to_string(),
                        target_id: after_block.block_id.clone(),
                        operation: "update".to_string(),
                        before: json_value(before_block).ok(),
                        after: json_value(after_block).ok(),
                        summary: format!("{} changed.", after_block.title),
                        legal_impact: LegalImpactSummary {
                            affected_counts: Vec::new(),
                            affected_elements: Vec::new(),
                            affected_facts: union_string_ids(
                                &before_block.fact_ids,
                                &after_block.fact_ids,
                            ),
                            affected_evidence: union_string_ids(
                                &before_block.evidence_ids,
                                &after_block.evidence_ids,
                            ),
                            affected_authorities: union_string_ids(
                                &before_authority_ids,
                                &after_authority_ids,
                            ),
                            affected_exhibits: Vec::new(),
                            support_status_before: None,
                            support_status_after: None,
                            qc_warnings_added: Vec::new(),
                            qc_warnings_resolved: Vec::new(),
                            blocking_issues_added: Vec::new(),
                            blocking_issues_resolved: Vec::new(),
                        },
                        ai_audit_id: None,
                    });
                }
            }
            None => changes.push(VersionChangeInput {
                target_type: target_type.to_string(),
                target_id: after_block.block_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(after_block).ok(),
                summary: format!("{} created.", after_block.title),
                legal_impact: LegalImpactSummary {
                    affected_counts: Vec::new(),
                    affected_elements: Vec::new(),
                    affected_facts: after_block.fact_ids.clone(),
                    affected_evidence: after_block.evidence_ids.clone(),
                    affected_authorities: after_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect(),
                    affected_exhibits: Vec::new(),
                    support_status_before: None,
                    support_status_after: None,
                    qc_warnings_added: Vec::new(),
                    qc_warnings_resolved: Vec::new(),
                    blocking_issues_added: Vec::new(),
                    blocking_issues_resolved: Vec::new(),
                },
                ai_audit_id: None,
            }),
        }
    }
    for before_block in &before.blocks {
        if !after
            .blocks
            .iter()
            .any(|block| block.block_id == before_block.block_id)
        {
            changes.push(VersionChangeInput {
                target_type: if before_block.block_type == "paragraph" {
                    "paragraph"
                } else {
                    "block"
                }
                .to_string(),
                target_id: before_block.block_id.clone(),
                operation: "delete".to_string(),
                before: json_value(before_block).ok(),
                after: None,
                summary: format!("{} removed.", before_block.title),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            });
        }
    }

    if json_value(&before.findings).ok() != json_value(&after.findings).ok() {
        let before_open = before
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect::<Vec<_>>();
        let after_open = after
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect::<Vec<_>>();
        changes.push(VersionChangeInput {
            target_type: "rule_finding".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "update".to_string(),
            before: json_value(&before.findings).ok(),
            after: json_value(&after.findings).ok(),
            summary: "QC findings changed.".to_string(),
            legal_impact: LegalImpactSummary {
                qc_warnings_added: after_open
                    .iter()
                    .filter(|id| !before_open.contains(id))
                    .cloned()
                    .collect(),
                qc_warnings_resolved: before_open
                    .iter()
                    .filter(|id| !after_open.contains(id))
                    .cloned()
                    .collect(),
                ..LegalImpactSummary::default()
            },
            ai_audit_id: None,
        });
    }

    for artifact in &after.artifacts {
        if !before
            .artifacts
            .iter()
            .any(|item| item.artifact_id == artifact.artifact_id)
        {
            changes.push(VersionChangeInput {
                target_type: "export".to_string(),
                target_id: artifact.artifact_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(artifact).ok(),
                summary: format!("{} export generated.", artifact.format.to_uppercase()),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            });
        }
    }

    changes
}

fn union_string_ids(left: &[String], right: &[String]) -> Vec<String> {
    let mut values = left.to_vec();
    for value in right {
        if !values.contains(value) {
            values.push(value.clone());
        }
    }
    values
}

fn version_summary_for_changes(summary: &str, changes: &[VersionChange]) -> VersionChangeSummary {
    VersionChangeSummary {
        text_changes: changes
            .iter()
            .filter(|change| {
                matches!(
                    change.target_type.as_str(),
                    "block" | "paragraph" | "work_product"
                )
            })
            .count() as u64,
        support_changes: changes
            .iter()
            .filter(|change| {
                change.target_type.contains("support") || change.target_type.contains("link")
            })
            .count() as u64,
        citation_changes: changes
            .iter()
            .filter(|change| change.target_type.contains("citation"))
            .count() as u64,
        authority_changes: changes
            .iter()
            .filter(|change| change.target_type.contains("authority"))
            .count() as u64,
        qc_changes: changes
            .iter()
            .filter(|change| {
                change.target_type.contains("finding") || change.target_type.contains("qc")
            })
            .count() as u64,
        export_changes: changes
            .iter()
            .filter(|change| change.target_type == "export" || change.target_type == "artifact")
            .count() as u64,
        ai_changes: changes
            .iter()
            .filter(|change| change.ai_audit_id.is_some() || change.target_type == "ai_edit")
            .count() as u64,
        targets_changed: changes
            .iter()
            .map(|change| VersionTargetSummary {
                target_type: change.target_type.clone(),
                target_id: change.target_id.clone(),
                label: Some(change.summary.clone()),
            })
            .collect(),
        risk_level: if changes
            .iter()
            .any(|change| !change.legal_impact.blocking_issues_added.is_empty())
        {
            "high"
        } else {
            "low"
        }
        .to_string(),
        user_summary: summary.to_string(),
    }
}

fn merge_legal_impacts<'a>(
    impacts: impl Iterator<Item = &'a LegalImpactSummary>,
) -> LegalImpactSummary {
    let mut merged = LegalImpactSummary::default();
    for impact in impacts {
        merged
            .affected_counts
            .extend(impact.affected_counts.clone());
        merged
            .affected_elements
            .extend(impact.affected_elements.clone());
        merged.affected_facts.extend(impact.affected_facts.clone());
        merged
            .affected_evidence
            .extend(impact.affected_evidence.clone());
        merged
            .affected_authorities
            .extend(impact.affected_authorities.clone());
        merged
            .affected_exhibits
            .extend(impact.affected_exhibits.clone());
        merged
            .qc_warnings_added
            .extend(impact.qc_warnings_added.clone());
        merged
            .qc_warnings_resolved
            .extend(impact.qc_warnings_resolved.clone());
        merged
            .blocking_issues_added
            .extend(impact.blocking_issues_added.clone());
        merged
            .blocking_issues_resolved
            .extend(impact.blocking_issues_resolved.clone());
    }
    merged
}

fn role_names(parties: &[ComplaintParty], roles: &[&str]) -> Vec<String> {
    let values = parties
        .iter()
        .filter(|party| {
            roles
                .iter()
                .any(|role| party.role.eq_ignore_ascii_case(role))
        })
        .map(|party| party.name.clone())
        .collect::<Vec<_>>();
    if values.is_empty() {
        roles
            .first()
            .map(|role| vec![title_case(role)])
            .unwrap_or_default()
    } else {
        values
    }
}

fn infer_county(court: &str) -> String {
    court
        .split_whitespace()
        .take_while(|part| !part.eq_ignore_ascii_case("county"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn empty_as_review_needed(value: &str) -> String {
    if value.trim().is_empty() {
        "[review needed]".to_string()
    } else {
        value.to_string()
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn put_options(mime_type: Option<String>, sha256: Option<String>) -> PutOptions {
    let mut metadata = BTreeMap::new();
    if let Some(sha256) = sha256 {
        metadata.insert("sha256".to_string(), sha256);
    }
    PutOptions {
        content_type: mime_type,
        metadata,
    }
}

fn build_original_provenance(
    matter_id: &str,
    document: &CaseDocument,
    object: &StoredObject,
    status: &str,
) -> DocumentProvenance {
    let now = now_string();
    let blob = ObjectBlob {
        object_blob_id: object_blob_id_for_document(document),
        id: object_blob_id_for_document(document),
        sha256: document.file_hash.clone(),
        size_bytes: object.content_length,
        mime_type: document
            .mime_type
            .clone()
            .or_else(|| object.content_type.clone()),
        storage_provider: document.storage_provider.clone(),
        storage_bucket: object
            .bucket
            .clone()
            .or_else(|| document.storage_bucket.clone()),
        storage_key: object.key.clone(),
        etag: object
            .etag
            .clone()
            .or_else(|| document.content_etag.clone()),
        storage_class: None,
        created_at: now.clone(),
        retention_state: "active".to_string(),
    };
    let version_id = original_version_id(&document.document_id);
    let version = DocumentVersion {
        document_version_id: version_id.clone(),
        id: version_id.clone(),
        matter_id: matter_id.to_string(),
        document_id: document.document_id.clone(),
        object_blob_id: blob.object_blob_id.clone(),
        role: "original".to_string(),
        artifact_kind: "original_upload".to_string(),
        source_version_id: None,
        created_by: "casebuilder_upload".to_string(),
        current: true,
        created_at: now.clone(),
        storage_provider: document.storage_provider.clone(),
        storage_bucket: blob.storage_bucket.clone(),
        storage_key: object.key.clone(),
        sha256: document.file_hash.clone(),
        size_bytes: object.content_length,
        mime_type: document
            .mime_type
            .clone()
            .or_else(|| object.content_type.clone()),
    };
    let run_id = primary_ingestion_run_id(&document.document_id);
    let run = IngestionRun {
        ingestion_run_id: run_id.clone(),
        id: run_id,
        matter_id: matter_id.to_string(),
        document_id: document.document_id.clone(),
        document_version_id: Some(version.document_version_id.clone()),
        object_blob_id: Some(blob.object_blob_id.clone()),
        input_sha256: document.file_hash.clone(),
        status: status.to_string(),
        stage: status.to_string(),
        mode: "deterministic".to_string(),
        started_at: now,
        completed_at: None,
        error_code: None,
        error_message: None,
        retryable: false,
        produced_node_ids: Vec::new(),
        produced_object_keys: vec![object.key.clone()],
        parser_id: Some("casebuilder-parser-registry".to_string()),
        parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
        chunker_version: Some(CHUNKER_VERSION.to_string()),
        citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
        index_version: Some(CASE_INDEX_VERSION.to_string()),
    };
    DocumentProvenance {
        object_blob: blob,
        document_version: version,
        ingestion_run: run,
    }
}

fn apply_document_provenance(document: &mut CaseDocument, provenance: &DocumentProvenance) {
    document.object_blob_id = Some(provenance.object_blob.object_blob_id.clone());
    document.current_version_id = Some(provenance.document_version.document_version_id.clone());
    push_unique(
        &mut document.ingestion_run_ids,
        provenance.ingestion_run.ingestion_run_id.clone(),
    );
}

fn source_context_from_provenance(provenance: Option<&DocumentProvenance>) -> SourceContext {
    SourceContext {
        document_version_id: provenance
            .map(|value| value.document_version.document_version_id.clone()),
        object_blob_id: provenance.map(|value| value.object_blob.object_blob_id.clone()),
        ingestion_run_id: provenance.map(|value| value.ingestion_run.ingestion_run_id.clone()),
    }
}

fn object_blob_id_for_document(document: &CaseDocument) -> String {
    if let Some(sha256) = document.file_hash.as_deref() {
        return object_blob_id_for_hash(sha256);
    }
    let seed = format!(
        "{}:{}:{}",
        document.storage_provider,
        document.storage_bucket.clone().unwrap_or_default(),
        document.storage_key.clone().unwrap_or_default()
    );
    format!("blob:object:{}", hex_prefix(seed.as_bytes(), 24))
}

fn object_blob_id_for_hash(sha256: &str) -> String {
    let raw = sha256
        .trim()
        .strip_prefix("sha256:")
        .unwrap_or(sha256.trim());
    format!("blob:sha256:{}", raw.to_ascii_lowercase())
}

fn should_inline_payload(byte_len: usize, limit: u64) -> bool {
    byte_len as u64 <= limit
}

fn storage_hash_segment(hash: &str) -> String {
    sanitize_path_segment(hash.trim().strip_prefix("sha256:").unwrap_or(hash.trim()))
}

fn work_product_storage_prefix(matter_id: &str, work_product_id: &str) -> String {
    format!(
        "casebuilder/matters/{}/work-products/{}",
        hex_prefix(matter_id.as_bytes(), 24),
        hex_prefix(work_product_id.as_bytes(), 24)
    )
}

fn snapshot_full_state_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    state_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/full-state.{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        storage_hash_segment(state_hash)
    )
}

fn snapshot_manifest_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    manifest_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/manifest.{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        storage_hash_segment(manifest_hash)
    )
}

fn snapshot_entity_state_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    entity_type: &str,
    entity_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/states/{}/{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        sanitize_path_segment(entity_type),
        storage_hash_segment(entity_hash)
    )
}

fn work_product_export_key(
    matter_id: &str,
    work_product_id: &str,
    artifact_id: &str,
    artifact_hash: &str,
    ext: &str,
) -> String {
    format!(
        "{}/exports/{}/{}.{}",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(artifact_id.as_bytes(), 24),
        storage_hash_segment(artifact_hash),
        sanitize_path_segment(ext)
    )
}

fn safe_work_product_download_filename(artifact: &WorkProductArtifact) -> String {
    let seed = artifact
        .artifact_hash
        .as_deref()
        .unwrap_or(artifact.artifact_id.as_str());
    format!(
        "work-product-export-{}.{}",
        hex_prefix(seed.as_bytes(), 16),
        sanitize_path_segment(&artifact.format)
    )
}

fn original_version_id(document_id: &str) -> String {
    format!("version:{}:original", sanitize_path_segment(document_id))
}

fn edited_version_id(document_id: &str, sha256: &str, now_secs: u64) -> String {
    format!(
        "version:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(format!("{sha256}:{now_secs}").as_bytes(), 16)
    )
}

fn edited_ingestion_run_id(document_id: &str, document_version_id: &str) -> String {
    format!(
        "ingestion:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(document_version_id.as_bytes(), 16)
    )
}

fn document_version_object_key(
    matter_id: &str,
    document_id: &str,
    document_version_id: &str,
    sha256: &str,
    extension: &str,
) -> String {
    format!(
        "casebuilder/matters/{}/documents/{}/versions/{}/{}.{}",
        hex_prefix(matter_id.as_bytes(), 24),
        hex_prefix(document_id.as_bytes(), 24),
        hex_prefix(document_version_id.as_bytes(), 24),
        storage_hash_segment(sha256),
        sanitize_path_segment(extension)
    )
}

fn artifact_version_id(document_id: &str, artifact_kind: &str, sha256: &str) -> String {
    format!(
        "version:{}:{}:{}",
        sanitize_path_segment(document_id),
        sanitize_path_segment(artifact_kind),
        hex_prefix(sha256.as_bytes(), 16)
    )
}

fn primary_ingestion_run_id(document_id: &str) -> String {
    format!("ingestion:{}:primary", sanitize_path_segment(document_id))
}

fn transcription_job_id(document_id: &str, now_secs: u64) -> String {
    format!(
        "transcription:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(format!("{document_id}:{now_secs}").as_bytes(), 16)
    )
}

fn transcript_segment_id(transcription_job_id: &str, ordinal: u64) -> String {
    format!(
        "segment:{}:{}",
        sanitize_path_segment(transcription_job_id),
        ordinal
    )
}

fn transcript_speaker_id(transcription_job_id: &str, speaker_label: &str) -> String {
    format!(
        "speaker:{}:{}",
        sanitize_path_segment(transcription_job_id),
        sanitize_path_segment(speaker_label)
    )
}

fn transcript_review_change_id(
    transcription_job_id: &str,
    target_id: &str,
    field: &str,
    now_secs: u64,
) -> String {
    format!(
        "transcript-change:{}:{}",
        sanitize_path_segment(transcription_job_id),
        hex_prefix(format!("{target_id}:{field}:{now_secs}").as_bytes(), 16)
    )
}

fn source_span_id(document_id: &str, kind: &str, index: u64) -> String {
    format!(
        "span:{}:{}:{}",
        sanitize_path_segment(document_id),
        sanitize_path_segment(kind),
        index
    )
}

fn index_run_id(document_id: &str, document_version_id: Option<&str>, text_sha256: &str) -> String {
    format!(
        "index-run:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(
            format!(
                "{}:{}",
                document_version_id.unwrap_or("unversioned"),
                text_sha256
            )
            .as_bytes(),
            20,
        )
    )
}

fn page_id(document_id: &str, page_number: u64) -> String {
    format!(
        "page:{}:{}",
        sanitize_path_segment(document_id),
        page_number
    )
}

fn evidence_span_id_for_chunk(document_id: &str, chunk_id: &str) -> String {
    format!(
        "evidence-span:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(chunk_id.as_bytes(), 20)
    )
}

fn search_index_record_id(document_id: &str, text_chunk_id: &str, index_version: &str) -> String {
    format!(
        "search-index:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(format!("{text_chunk_id}:{index_version}").as_bytes(), 20)
    )
}

fn extraction_manifest_id(
    document_id: &str,
    document_version_id: Option<&str>,
    text_sha256: &str,
) -> String {
    format!(
        "extraction-manifest:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(
            format!(
                "{}:{}",
                document_version_id.unwrap_or("unversioned"),
                text_sha256
            )
            .as_bytes(),
            20,
        )
    )
}

fn text_excerpt(value: &str, limit: usize) -> String {
    let cleaned = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() <= limit {
        cleaned
    } else {
        let mut excerpt = cleaned.chars().take(limit).collect::<String>();
        excerpt.push_str("...");
        excerpt
    }
}

fn approximate_token_count(value: &str) -> u64 {
    value.split_whitespace().count().max(1) as u64
}

fn source_spans_for_chunks(
    matter_id: &str,
    document_id: &str,
    chunks: &[ExtractedTextChunk],
    context: &SourceContext,
) -> Vec<SourceSpan> {
    chunks
        .iter()
        .map(|chunk| SourceSpan {
            source_span_id: chunk
                .source_span_id
                .clone()
                .unwrap_or_else(|| source_span_id(document_id, "chunk", chunk.page)),
            id: chunk
                .source_span_id
                .clone()
                .unwrap_or_else(|| source_span_id(document_id, "chunk", chunk.page)),
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: context.document_version_id.clone(),
            object_blob_id: context.object_blob_id.clone(),
            ingestion_run_id: context.ingestion_run_id.clone(),
            page: Some(chunk.page),
            chunk_id: Some(chunk.chunk_id.clone()),
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            char_start: chunk.char_start,
            char_end: chunk.char_end,
            time_start_ms: None,
            time_end_ms: None,
            speaker_label: None,
            quote: Some(chunk.text.clone()),
            extraction_method: "deterministic_text_chunk".to_string(),
            confidence: 1.0,
            review_status: "unreviewed".to_string(),
            unavailable_reason: None,
        })
        .collect()
}

fn source_span_for_sentence(
    matter_id: &str,
    document_id: &str,
    index: u64,
    sentence: &SentenceCandidate,
    context: &SourceContext,
) -> SourceSpan {
    let id = source_span_id(document_id, "fact", index);
    SourceSpan {
        source_span_id: id.clone(),
        id,
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        page: Some(1),
        chunk_id: None,
        byte_start: Some(sentence.byte_start),
        byte_end: Some(sentence.byte_end),
        char_start: Some(sentence.char_start),
        char_end: Some(sentence.char_end),
        time_start_ms: None,
        time_end_ms: None,
        speaker_label: None,
        quote: Some(sentence.text.clone()),
        extraction_method: "deterministic_sentence".to_string(),
        confidence: 0.55,
        review_status: "unreviewed".to_string(),
        unavailable_reason: None,
    }
}

fn manual_evidence_source_span(
    matter_id: &str,
    document_id: &str,
    evidence_id: &str,
    source_span: Option<&str>,
    quote: &str,
    context: &SourceContext,
) -> SourceSpan {
    let id = format!("span:{}:evidence", sanitize_path_segment(evidence_id));
    SourceSpan {
        source_span_id: id.clone(),
        id,
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        page: None,
        chunk_id: source_span.map(str::to_string),
        byte_start: None,
        byte_end: None,
        char_start: None,
        char_end: None,
        time_start_ms: None,
        time_end_ms: None,
        speaker_label: None,
        quote: Some(quote.to_string()),
        extraction_method: "manual_evidence_quote".to_string(),
        confidence: 0.75,
        review_status: "unreviewed".to_string(),
        unavailable_reason: None,
    }
}

fn assemblyai_speech_models() -> Vec<String> {
    vec!["universal-3-pro".to_string(), "universal-2".to_string()]
}

fn validate_assemblyai_transcription_request(
    request: &CreateTranscriptionRequest,
) -> ApiResult<()> {
    let prompt = assemblyai_effective_prompt(request)?;
    let keyterms_prompt =
        sanitize_assemblyai_keyterms(request.keyterms_prompt.iter().map(String::as_str));
    let speaker_labels = request.speaker_labels.unwrap_or(true);
    if prompt.is_some() && !keyterms_prompt.is_empty() {
        return Err(ApiError::BadRequest(
            "AssemblyAI prompt and keyterms_prompt cannot be used in the same request.".to_string(),
        ));
    }
    if let Some(prompt) = prompt.as_deref() {
        let word_count = prompt.split_whitespace().count();
        if word_count > ASSEMBLYAI_PROMPT_MAX_WORDS {
            return Err(ApiError::BadRequest(format!(
                "AssemblyAI prompt must be {ASSEMBLYAI_PROMPT_MAX_WORDS} words or fewer."
            )));
        }
    }
    if assemblyai_keyterms_word_count(&keyterms_prompt) > ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL {
        return Err(ApiError::BadRequest(format!(
            "AssemblyAI keyterms_prompt must include {ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL} words or fewer across all terms."
        )));
    }
    normalize_assemblyai_speaker_config(
        speaker_labels,
        request.speakers_expected,
        request.speaker_options.as_ref(),
    )?;
    normalize_assemblyai_remove_audio_tags(request.remove_audio_tags.as_deref())?;
    Ok(())
}

fn assemblyai_effective_prompt(request: &CreateTranscriptionRequest) -> ApiResult<Option<String>> {
    if let Some(prompt) = sanitize_assemblyai_prompt(request.prompt.as_deref()) {
        return Ok(Some(prompt));
    }
    if let Some(preset) = normalize_assemblyai_prompt_preset(request.prompt_preset.as_deref())? {
        return Ok(Some(assemblyai_prompt_preset_text(&preset).to_string()));
    }
    Ok(None)
}

fn normalize_assemblyai_prompt_preset(value: Option<&str>) -> ApiResult<Option<String>> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('-', "_").to_ascii_lowercase())
    {
        Some(value) if matches!(value.as_str(), "none" | "default" | "off") => Ok(None),
        Some(value) if ASSEMBLYAI_PROMPT_PRESETS.contains(&value.as_str()) => Ok(Some(value)),
        Some(value) => Err(ApiError::BadRequest(format!(
            "AssemblyAI prompt_preset is not supported: {value}."
        ))),
        None => Ok(None),
    }
}

fn assemblyai_prompt_preset_text(preset: &str) -> &'static str {
    match preset {
        "verbatim_multilingual" => {
            "Required: Preserve the original language and script as spoken, including code-switching and mixed-language phrases.\n\nMandatory: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language.\n\nAlways: Transcribe speech with your best guess based on context in all possible scenarios where speech is present in the audio."
        }
        "unclear_masked" => {
            "Always: Transcribe speech exactly as heard. If uncertain or audio is unclear, mark as [masked]. After the first output, review the transcript again. Pay close attention to hallucinations, misspellings, or errors, and revise them like a computer performing spell and grammar checks. Ensure words and phrases make grammatical sense in sentences."
        }
        "unclear" => {
            "Always: Transcribe speech exactly as heard. If uncertain or audio is unclear, mark as [unclear]. After the first output, review the transcript again. Pay close attention to hallucinations, misspellings, or errors, and revise them like a computer performing spell and grammar checks. Ensure words and phrases make grammatical sense in sentences."
        }
        "legal" => {
            "Mandatory: Transcribe legal proceedings and legal recordings with precise terminology intact.\n\nRequired: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language.\n\nNon-negotiable: Preserve legal entity names, party names, exhibit references, citations, acronyms, dates, and monetary amounts exactly when clear in the audio."
        }
        "medical" => {
            "Mandatory: Preserve clinical terminology exactly as spoken, including drug names, dosages, conditions, procedures, and diagnostic terms.\n\nRequired: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language.\n\nNon-negotiable: Use the most contextually correct spelling for medical terms and proper nouns."
        }
        "financial" => {
            "Mandatory: Transcribe financial discussions with precise financial terminology.\n\nRequired: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language.\n\nNon-negotiable: Preserve financial terms, acronyms, company names, and industry-standard phrases. Format numerical data with standard notation."
        }
        "technical" => {
            "Mandatory: Transcribe technical discussions with precise terminology.\n\nRequired: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language.\n\nNon-negotiable: Preserve software names, framework names, code terms, acronyms, command names, and technical proper nouns exactly when clear in the audio."
        }
        "code_switching" => {
            "Mandatory: Transcribe verbatim, preserving natural code-switching between languages.\n\nRequired: Retain spoken language as-is without translation. Preserve words in the language they are spoken.\n\nNon-negotiable: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language."
        }
        "customer_support" => {
            "Context: a customer support call. Prioritize accurately transcribing names, account details, balance amounts, and organization names.\n\nMandatory: Transcribe overlapping speech across channels including crosstalk when audible.\n\nNon-negotiable: Preserve linguistic speech patterns including disfluencies, filler words, hesitations, repetitions, stutters, false starts, and colloquialisms in the spoken language."
        }
        _ => unreachable!("prompt preset must be validated before lookup"),
    }
}

fn sanitize_assemblyai_prompt(prompt: Option<&str>) -> Option<String> {
    prompt
        .map(str::trim)
        .filter(|prompt| !prompt.is_empty())
        .map(str::to_string)
}

fn sanitize_assemblyai_keyterms<'a>(terms: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    let mut word_budget_used = 0usize;
    for term in terms {
        let words = term
            .split_whitespace()
            .map(str::trim)
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        if words.is_empty() || words.len() > ASSEMBLYAI_KEYTERM_MAX_WORDS {
            continue;
        }
        if word_budget_used + words.len() > ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL {
            continue;
        }
        let normalized = words.join(" ");
        if out
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&normalized))
        {
            continue;
        }
        word_budget_used += words.len();
        out.push(normalized);
        if word_budget_used >= ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL {
            break;
        }
    }
    out
}

fn assemblyai_keyterms_word_count(terms: &[String]) -> usize {
    terms
        .iter()
        .map(|term| term.split_whitespace().count())
        .sum()
}

fn normalize_assemblyai_speaker_config(
    speaker_labels: bool,
    speakers_expected: Option<u64>,
    speaker_options: Option<&AssemblyAiSpeakerOptions>,
) -> ApiResult<(Option<u64>, Option<AssemblyAiSpeakerOptions>)> {
    let speaker_options = speaker_options.and_then(normalize_assemblyai_speaker_options);
    if !speaker_labels && (speakers_expected.is_some() || speaker_options.is_some()) {
        return Err(ApiError::BadRequest(
            "AssemblyAI speaker counts require speaker_labels to be enabled.".to_string(),
        ));
    }
    if speakers_expected.is_some() && speaker_options.is_some() {
        return Err(ApiError::BadRequest(
            "AssemblyAI speakers_expected and speaker_options cannot be used together.".to_string(),
        ));
    }
    if matches!(speakers_expected, Some(0)) {
        return Err(ApiError::BadRequest(
            "AssemblyAI speakers_expected must be greater than zero.".to_string(),
        ));
    }
    if let Some(options) = speaker_options.as_ref() {
        match (options.min_speakers_expected, options.max_speakers_expected) {
            (Some(0), _) | (_, Some(0)) => {
                return Err(ApiError::BadRequest(
                    "AssemblyAI speaker_options values must be greater than zero.".to_string(),
                ));
            }
            (Some(min), Some(max)) if min > max => {
                return Err(ApiError::BadRequest(
                    "AssemblyAI min_speakers_expected cannot exceed max_speakers_expected."
                        .to_string(),
                ));
            }
            (None, None) => {
                return Err(ApiError::BadRequest(
                    "AssemblyAI speaker_options requires min_speakers_expected or max_speakers_expected."
                        .to_string(),
                ));
            }
            _ => {}
        }
    }
    Ok((speakers_expected, speaker_options))
}

fn normalize_assemblyai_speaker_options(
    options: &AssemblyAiSpeakerOptions,
) -> Option<AssemblyAiSpeakerOptions> {
    if options.min_speakers_expected.is_none() && options.max_speakers_expected.is_none() {
        return None;
    }
    Some(options.clone())
}

fn normalize_assemblyai_remove_audio_tags(value: Option<&str>) -> ApiResult<Option<String>> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
    {
        Some(value) if value == ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL => Ok(Some(value)),
        Some(_) => Err(ApiError::BadRequest(
            "AssemblyAI remove_audio_tags must be \"all\" when provided.".to_string(),
        )),
        None => Ok(Some(ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL.to_string())),
    }
}

fn assemblyai_redact_pii_policies() -> Vec<String> {
    [
        "account_number",
        "banking_information",
        "credit_card_cvv",
        "credit_card_expiration",
        "credit_card_number",
        "date_of_birth",
        "drivers_license",
        "email_address",
        "healthcare_number",
        "ip_address",
        "location",
        "passport_number",
        "password",
        "person_name",
        "phone_number",
        "us_social_security_number",
        "username",
        "vehicle_id",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn assemblyai_webhook_header_name() -> &'static str {
    "x-casebuilder-assemblyai-secret"
}

fn assemblyai_transcript_create_request(
    audio_url: &str,
    request: &CreateTranscriptionRequest,
    job: &TranscriptionJob,
    webhook_url: Option<String>,
    webhook_secret: Option<String>,
) -> AssemblyAiTranscriptCreateRequest {
    let webhook_enabled = webhook_url.is_some() && webhook_secret.is_some();
    let redact_pii = request.redact_pii.unwrap_or(true);
    let speaker_labels = request.speaker_labels.unwrap_or(true);
    let (speakers_expected, speaker_options) = normalize_assemblyai_speaker_config(
        speaker_labels,
        request.speakers_expected,
        request.speaker_options.as_ref(),
    )
    .unwrap_or((None, None));
    let prompt = assemblyai_effective_prompt(request).unwrap_or_else(|_| None);
    let keyterms_prompt =
        sanitize_assemblyai_keyterms(request.keyterms_prompt.iter().map(String::as_str));
    AssemblyAiTranscriptCreateRequest {
        audio_url: audio_url.to_string(),
        speech_models: job.speech_models.clone(),
        language_detection: request.language_code.is_none(),
        speaker_labels,
        speakers_expected,
        speaker_options,
        redact_pii: redact_pii.then_some(true),
        redact_pii_policies: redact_pii.then(assemblyai_redact_pii_policies),
        redact_pii_sub: redact_pii.then_some("entity_name".to_string()),
        redact_pii_return_unredacted: redact_pii.then_some(true),
        redact_pii_audio: redact_pii.then_some(true),
        redact_pii_audio_quality: redact_pii
            .then_some(ASSEMBLYAI_REDACTED_AUDIO_QUALITY.to_string()),
        language_code: request.language_code.clone(),
        prompt,
        keyterms_prompt: (!keyterms_prompt.is_empty()).then_some(keyterms_prompt),
        remove_audio_tags: normalize_assemblyai_remove_audio_tags(
            request.remove_audio_tags.as_deref(),
        )
        .unwrap_or_else(|_| Some(ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL.to_string())),
        webhook_url: webhook_url.filter(|_| webhook_enabled),
        webhook_auth_header_name: if webhook_enabled {
            Some(assemblyai_webhook_header_name().to_string())
        } else {
            None
        },
        webhook_auth_header_value: webhook_secret.filter(|_| webhook_enabled),
    }
}

fn assemblyai_provider_has_redaction(provider: &AssemblyAiTranscriptResponse) -> bool {
    provider.redact_pii.unwrap_or(false)
        || provider.redact_pii_return_unredacted.unwrap_or(false)
        || provider.unredacted_text.is_some()
        || !provider.unredacted_utterances.is_empty()
        || !provider.unredacted_words.is_empty()
}

fn assemblyai_raw_utterances(provider: &AssemblyAiTranscriptResponse) -> &[AssemblyAiUtterance] {
    if provider.unredacted_utterances.is_empty() {
        &provider.utterances
    } else {
        &provider.unredacted_utterances
    }
}

fn assemblyai_raw_words(provider: &AssemblyAiTranscriptResponse) -> &[AssemblyAiWord] {
    if provider.unredacted_words.is_empty() {
        &provider.words
    } else {
        &provider.unredacted_words
    }
}

fn assemblyai_raw_text(provider: &AssemblyAiTranscriptResponse) -> Option<&str> {
    provider
        .unredacted_text
        .as_deref()
        .or(provider.text.as_deref())
        .filter(|text| !text.trim().is_empty())
}

fn assemblyai_word_count(
    provider: &AssemblyAiTranscriptResponse,
    sentences: &AssemblyAiSentencesResponse,
    paragraphs: &AssemblyAiParagraphsResponse,
) -> usize {
    let raw_words = assemblyai_raw_words(provider);
    if raw_words.is_empty() {
        let sentence_words = sentences
            .sentences
            .iter()
            .map(|sentence| sentence.words.len())
            .sum::<usize>();
        if sentence_words == 0 {
            paragraphs
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.words.len())
                .sum()
        } else {
            sentence_words
        }
    } else {
        raw_words.len()
    }
}

fn assemblyai_redacted_utterance_text(
    provider: &AssemblyAiTranscriptResponse,
    index: usize,
    raw_text: &str,
) -> String {
    if assemblyai_provider_has_redaction(provider) {
        if let Some(text) = provider
            .utterances
            .get(index)
            .map(|utterance| utterance.text.trim())
            .filter(|text| !text.is_empty())
        {
            return text.to_string();
        }
    }
    redact_transcript_text(raw_text)
}

fn assemblyai_redacted_sentence_text(
    provider: &AssemblyAiTranscriptResponse,
    sentence: &AssemblyAiSentence,
    raw_text: &str,
) -> String {
    if assemblyai_provider_has_redaction(provider) {
        let text = sentence.text.trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    redact_transcript_text(raw_text)
}

fn assemblyai_unredacted_sentence_text(
    provider: &AssemblyAiTranscriptResponse,
    sentence: &AssemblyAiSentence,
) -> Option<String> {
    assemblyai_unredacted_timed_text(provider, sentence.start, sentence.end)
}

fn assemblyai_unredacted_paragraph_text(
    provider: &AssemblyAiTranscriptResponse,
    paragraph: &AssemblyAiParagraph,
) -> Option<String> {
    assemblyai_unredacted_timed_text(provider, paragraph.start, paragraph.end)
}

fn assemblyai_unredacted_timed_text(
    provider: &AssemblyAiTranscriptResponse,
    start: u64,
    end: u64,
) -> Option<String> {
    if !assemblyai_provider_has_redaction(provider) || provider.unredacted_words.is_empty() {
        return None;
    }
    let words = provider
        .unredacted_words
        .iter()
        .filter(|word| transcript_word_overlaps(word, start, end))
        .collect::<Vec<_>>();
    if words.is_empty() {
        None
    } else {
        Some(join_transcript_words(
            words.iter().map(|word| word.text.as_str()),
        ))
    }
}

fn assemblyai_redacted_paragraph_text(
    provider: &AssemblyAiTranscriptResponse,
    paragraph: &AssemblyAiParagraph,
    raw_text: &str,
) -> String {
    if assemblyai_provider_has_redaction(provider) {
        let text = paragraph.text.trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    redact_transcript_text(raw_text)
}

fn assemblyai_sentence_speaker(sentence: &AssemblyAiSentence) -> Option<String> {
    sentence.speaker.clone().or_else(|| {
        dominant_transcript_label(
            sentence
                .words
                .iter()
                .filter_map(|word| word.speaker.as_deref()),
        )
    })
}

fn assemblyai_sentence_channel(sentence: &AssemblyAiSentence) -> Option<String> {
    sentence.channel.clone().or_else(|| {
        dominant_transcript_label(
            sentence
                .words
                .iter()
                .filter_map(|word| word.channel.as_deref()),
        )
    })
}

fn assemblyai_paragraph_speaker(paragraph: &AssemblyAiParagraph) -> Option<String> {
    dominant_transcript_label(
        paragraph
            .words
            .iter()
            .filter_map(|word| word.speaker.as_deref()),
    )
}

fn assemblyai_paragraph_channel(paragraph: &AssemblyAiParagraph) -> Option<String> {
    dominant_transcript_label(
        paragraph
            .words
            .iter()
            .filter_map(|word| word.channel.as_deref()),
    )
}

fn dominant_transcript_label<'a>(labels: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for label in labels {
        let label = label.trim();
        if !label.is_empty() {
            *counts.entry(label).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(left.0)))
        .map(|(label, _)| label.to_string())
}

fn transcript_word_overlaps(word: &AssemblyAiWord, start: u64, end: u64) -> bool {
    word.start < end && word.end > start
}

fn transcript_time_overlap(start: u64, end: u64, other_start: u64, other_end: u64) -> u64 {
    end.min(other_end).saturating_sub(start.max(other_start))
}

fn join_transcript_words<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for word in words {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        if out.is_empty() || transcript_word_attaches_to_previous(word) {
            out.push_str(word);
        } else {
            out.push(' ');
            out.push_str(word);
        }
    }
    out
}

fn transcript_word_attaches_to_previous(word: &str) -> bool {
    matches!(
        word.chars().next(),
        Some('.' | ',' | '?' | '!' | ':' | ';' | ')' | ']' | '}')
    )
}

fn assemblyai_redacted_document_text(
    provider: &AssemblyAiTranscriptResponse,
    raw_text: &str,
) -> String {
    if assemblyai_provider_has_redaction(provider) {
        if let Some(text) = provider
            .text
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            return text.to_string();
        }
    }
    redact_transcript_text(raw_text)
}

fn transcript_segments_from_provider(
    matter_id: &str,
    document: &CaseDocument,
    job: &TranscriptionJob,
    provider: &AssemblyAiTranscriptResponse,
    sentences: &AssemblyAiSentencesResponse,
    paragraphs: &AssemblyAiParagraphsResponse,
    now: &str,
) -> (Vec<TranscriptSegment>, Vec<TranscriptSpeaker>) {
    let mut segments = Vec::new();
    if !sentences.sentences.is_empty() {
        for sentence in &sentences.sentences {
            let sentence_text = sentence.text.trim().to_string();
            if sentence_text.is_empty() {
                continue;
            }
            let ordinal = segments.len() as u64 + 1;
            let raw_text = if job.redact_pii {
                assemblyai_unredacted_sentence_text(provider, sentence)
                    .filter(|text| !text.trim().is_empty())
                    .unwrap_or_else(|| sentence_text.clone())
            } else {
                sentence_text.clone()
            };
            let redacted_text = if job.redact_pii {
                assemblyai_redacted_sentence_text(provider, sentence, &raw_text)
            } else {
                raw_text.clone()
            };
            let segment_id = transcript_segment_id(&job.transcription_job_id, ordinal);
            segments.push(TranscriptSegment {
                segment_id: segment_id.clone(),
                id: segment_id,
                matter_id: matter_id.to_string(),
                document_id: document.document_id.clone(),
                transcription_job_id: job.transcription_job_id.clone(),
                source_span_id: None,
                ordinal,
                paragraph_ordinal: None,
                speaker_label: assemblyai_sentence_speaker(sentence),
                speaker_name: None,
                channel: assemblyai_sentence_channel(sentence),
                redacted_text: Some(redacted_text),
                text: raw_text,
                time_start_ms: sentence.start,
                time_end_ms: sentence.end,
                confidence: sentence
                    .confidence
                    .or(sentences.confidence)
                    .or(provider.confidence)
                    .unwrap_or(0.0),
                review_status: "unreviewed".to_string(),
                edited: false,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
    }
    let utterances = assemblyai_raw_utterances(provider);
    let words = assemblyai_raw_words(provider);
    if segments.is_empty() && !utterances.is_empty() {
        for (index, utterance) in utterances.iter().enumerate() {
            let ordinal = index as u64 + 1;
            let speaker_label = utterance.speaker.clone();
            let text = utterance.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let redacted_text = if job.redact_pii {
                assemblyai_redacted_utterance_text(provider, index, &text)
            } else {
                text.clone()
            };
            let segment_id = transcript_segment_id(&job.transcription_job_id, ordinal);
            segments.push(TranscriptSegment {
                segment_id: segment_id.clone(),
                id: segment_id,
                matter_id: matter_id.to_string(),
                document_id: document.document_id.clone(),
                transcription_job_id: job.transcription_job_id.clone(),
                source_span_id: None,
                ordinal,
                paragraph_ordinal: None,
                speaker_label,
                speaker_name: None,
                channel: utterance.channel.clone(),
                redacted_text: Some(redacted_text),
                text,
                time_start_ms: utterance.start,
                time_end_ms: utterance.end,
                confidence: utterance
                    .confidence
                    .unwrap_or(provider.confidence.unwrap_or(0.0)),
                review_status: "unreviewed".to_string(),
                edited: false,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
    } else if segments.is_empty() {
        if let Some(text) = assemblyai_raw_text(provider) {
            let end = provider
                .audio_duration
                .map(|seconds| (seconds * 1000.0) as u64)
                .or_else(|| words.last().map(|word| word.end))
                .unwrap_or(0);
            let text = text.trim().to_string();
            if text.is_empty() {
                return (segments, Vec::new());
            }
            let redacted_text = if job.redact_pii {
                assemblyai_redacted_document_text(provider, &text)
            } else {
                text.clone()
            };
            let segment_id = transcript_segment_id(&job.transcription_job_id, 1);
            segments.push(TranscriptSegment {
                segment_id: segment_id.clone(),
                id: segment_id,
                matter_id: matter_id.to_string(),
                document_id: document.document_id.clone(),
                transcription_job_id: job.transcription_job_id.clone(),
                source_span_id: None,
                ordinal: 1,
                paragraph_ordinal: None,
                speaker_label: None,
                speaker_name: None,
                channel: None,
                text,
                redacted_text: Some(redacted_text),
                time_start_ms: words.first().map(|word| word.start).unwrap_or(0),
                time_end_ms: end,
                confidence: provider.confidence.unwrap_or(0.0),
                review_status: "unreviewed".to_string(),
                edited: false,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
    }
    apply_assemblyai_paragraphs_to_segments(&mut segments, paragraphs);

    let mut speaker_counts: BTreeMap<String, u64> = BTreeMap::new();
    for segment in &segments {
        if let Some(label) = segment.speaker_label.as_deref() {
            *speaker_counts.entry(label.to_string()).or_insert(0) += 1;
        }
    }
    let speakers = speaker_counts
        .into_iter()
        .map(|(label, count)| TranscriptSpeaker {
            speaker_id: transcript_speaker_id(&job.transcription_job_id, &label),
            id: transcript_speaker_id(&job.transcription_job_id, &label),
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            transcription_job_id: job.transcription_job_id.clone(),
            speaker_label: label,
            display_name: None,
            role: None,
            confidence: provider.confidence,
            segment_count: count,
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
        .collect();
    (segments, speakers)
}

fn apply_assemblyai_paragraphs_to_segments(
    segments: &mut [TranscriptSegment],
    paragraphs: &AssemblyAiParagraphsResponse,
) {
    if paragraphs.paragraphs.is_empty() {
        return;
    }
    for segment in segments {
        let paragraph_index = paragraphs
            .paragraphs
            .iter()
            .enumerate()
            .max_by_key(|(_, paragraph)| {
                transcript_time_overlap(
                    segment.time_start_ms,
                    segment.time_end_ms,
                    paragraph.start,
                    paragraph.end,
                )
            })
            .and_then(|(index, paragraph)| {
                let overlap = transcript_time_overlap(
                    segment.time_start_ms,
                    segment.time_end_ms,
                    paragraph.start,
                    paragraph.end,
                );
                (overlap > 0).then_some(index as u64 + 1)
            });
        segment.paragraph_ordinal = paragraph_index;
    }
}

fn transcript_paragraph_payloads(
    provider: &AssemblyAiTranscriptResponse,
    paragraphs: &AssemblyAiParagraphsResponse,
    redact_pii: bool,
) -> Vec<serde_json::Value> {
    paragraphs
        .paragraphs
        .iter()
        .enumerate()
        .filter_map(|(index, paragraph)| {
            let paragraph_text = paragraph.text.trim().to_string();
            if paragraph_text.is_empty() {
                return None;
            }
            let raw_text = if redact_pii {
                assemblyai_unredacted_paragraph_text(provider, paragraph)
                    .filter(|text| !text.trim().is_empty())
                    .unwrap_or_else(|| paragraph_text.clone())
            } else {
                paragraph_text.clone()
            };
            let redacted_text = if redact_pii {
                assemblyai_redacted_paragraph_text(provider, paragraph, &raw_text)
            } else {
                raw_text.clone()
            };
            Some(serde_json::json!({
                "ordinal": index + 1,
                "text": raw_text,
                "redacted_text": redacted_text,
                "time_start_ms": paragraph.start,
                "time_end_ms": paragraph.end,
                "confidence": paragraph.confidence.or(paragraphs.confidence),
                "speaker_label": assemblyai_paragraph_speaker(paragraph),
                "channel": assemblyai_paragraph_channel(paragraph),
                "word_count": paragraph.words.len(),
            }))
        })
        .collect()
}

fn should_use_assemblyai_subtitles(
    provider: &AssemblyAiTranscriptResponse,
    redact_pii: bool,
) -> bool {
    !redact_pii || assemblyai_provider_has_redaction(provider)
}

fn provider_subtitle_or_local(
    provider_subtitle: Option<String>,
    local_subtitle: String,
) -> (String, &'static str) {
    match provider_subtitle
        .map(|subtitle| subtitle.trim().to_string())
        .filter(|subtitle| !subtitle.is_empty())
    {
        Some(subtitle) => (subtitle, "assemblyai_subtitles"),
        None => (local_subtitle, "casebuilder_local"),
    }
}

fn normalize_assemblyai_transcript_list_query(
    mut query: AssemblyAiTranscriptListQuery,
) -> ApiResult<AssemblyAiTranscriptListQuery> {
    let limit = query
        .limit
        .unwrap_or(ASSEMBLYAI_TRANSCRIPT_LIST_DEFAULT_LIMIT);
    if limit == 0 || limit > ASSEMBLYAI_TRANSCRIPT_LIST_MAX_LIMIT {
        return Err(ApiError::BadRequest(format!(
            "AssemblyAI transcript list limit must be between 1 and {ASSEMBLYAI_TRANSCRIPT_LIST_MAX_LIMIT}."
        )));
    }
    query.limit = Some(limit);
    query.status = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .map(str::to_ascii_lowercase);
    if let Some(status) = query.status.as_deref() {
        if !matches!(status, "queued" | "processing" | "completed" | "error") {
            return Err(ApiError::BadRequest(
                "AssemblyAI transcript status must be queued, processing, completed, or error."
                    .to_string(),
            ));
        }
    }
    query.created_on = query
        .created_on
        .as_deref()
        .map(str::trim)
        .filter(|created_on| !created_on.is_empty())
        .map(str::to_string);
    if let Some(created_on) = query.created_on.as_deref() {
        if !assemblyai_created_on_date_is_valid(created_on) {
            return Err(ApiError::BadRequest(
                "AssemblyAI created_on must use YYYY-MM-DD format.".to_string(),
            ));
        }
    }
    query.before_id = sanitize_optional_assemblyai_transcript_id(query.before_id.as_deref());
    query.after_id = sanitize_optional_assemblyai_transcript_id(query.after_id.as_deref());
    Ok(query)
}

fn assemblyai_transcript_list_query_pairs(
    query: &AssemblyAiTranscriptListQuery,
) -> Vec<(&'static str, String)> {
    let mut pairs = vec![(
        "limit",
        query
            .limit
            .unwrap_or(ASSEMBLYAI_TRANSCRIPT_LIST_DEFAULT_LIMIT)
            .to_string(),
    )];
    if let Some(status) = query.status.as_deref().filter(|status| !status.is_empty()) {
        pairs.push(("status", status.to_string()));
    }
    if let Some(created_on) = query
        .created_on
        .as_deref()
        .filter(|created_on| !created_on.is_empty())
    {
        pairs.push(("created_on", created_on.to_string()));
    }
    if let Some(before_id) = query
        .before_id
        .as_deref()
        .filter(|before_id| !before_id.is_empty())
    {
        pairs.push(("before_id", before_id.to_string()));
    }
    if let Some(after_id) = query
        .after_id
        .as_deref()
        .filter(|after_id| !after_id.is_empty())
    {
        pairs.push(("after_id", after_id.to_string()));
    }
    if let Some(throttled_only) = query.throttled_only {
        pairs.push(("throttled_only", throttled_only.to_string()));
    }
    pairs
}

fn assemblyai_created_on_date_is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn sanitize_optional_assemblyai_transcript_id(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_path_segment)
}

fn normalize_assemblyai_transcript_id(value: &str) -> ApiResult<String> {
    let transcript_id = sanitize_path_segment(value.trim());
    if transcript_id.is_empty() {
        return Err(ApiError::BadRequest(
            "AssemblyAI transcript_id is required.".to_string(),
        ));
    }
    Ok(transcript_id)
}

fn assemblyai_transcript_delete_response(
    requested_transcript_id: &str,
    provider_response: serde_json::Value,
) -> AssemblyAiTranscriptDeleteResponse {
    let id = provider_response
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(requested_transcript_id)
        .to_string();
    let status = provider_response
        .get("status")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("deleted")
        .to_string();
    let deleted = status == "deleted"
        || provider_response
            .get("text")
            .map(|value| value.is_null())
            .unwrap_or(false);
    AssemblyAiTranscriptDeleteResponse {
        id,
        status,
        deleted,
        provider_response,
    }
}

fn assemblyai_default_word_search_terms(
    provider: &AssemblyAiTranscriptResponse,
    sentences: &AssemblyAiSentencesResponse,
    paragraphs: &AssemblyAiParagraphsResponse,
) -> Vec<String> {
    let mut text_parts = Vec::new();
    if let Some(text) = provider
        .text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        text_parts.push(text.to_string());
    } else if let Some(text) = assemblyai_raw_text(provider) {
        text_parts.push(text.to_string());
    }
    if text_parts.is_empty() {
        text_parts.extend(
            sentences
                .sentences
                .iter()
                .map(|sentence| sentence.text.trim())
                .filter(|text| !text.is_empty())
                .map(str::to_string),
        );
    }
    if text_parts.is_empty() {
        text_parts.extend(
            paragraphs
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.text.trim())
                .filter(|text| !text.is_empty())
                .map(str::to_string),
        );
    }
    let text = text_parts.join(" ");
    if text.trim().is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let lower_text = text.to_lowercase();
    for term in [
        "agreement",
        "attorney",
        "contract",
        "court",
        "damage",
        "deadline",
        "deposit",
        "email",
        "evidence",
        "hearing",
        "invoice",
        "lease",
        "notice",
        "payment",
        "receipt",
        "repair",
        "tenant",
    ] {
        if lower_text.contains(term) {
            candidates.push(term.to_string());
        }
    }

    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for token in text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 4)
    {
        let token = token.to_ascii_lowercase();
        if !assemblyai_word_search_stopword(&token) {
            *counts.entry(token).or_insert(0) += 1;
        }
    }
    let mut counted_terms = counts.into_iter().collect::<Vec<_>>();
    counted_terms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates.extend(
        counted_terms
            .into_iter()
            .take(ASSEMBLYAI_WORD_SEARCH_MAX_TERMS)
            .map(|(term, _)| term),
    );
    sanitize_assemblyai_word_search_terms(candidates.iter().map(String::as_str))
}

fn sanitize_assemblyai_word_search_terms<'a>(
    terms: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let mut out = Vec::new();
    for term in terms {
        let words = term
            .split_whitespace()
            .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        if words.is_empty() || words.len() > ASSEMBLYAI_WORD_SEARCH_MAX_WORDS_PER_TERM {
            continue;
        }
        let normalized = words.join(" ").to_lowercase();
        if normalized.is_empty()
            || assemblyai_word_search_stopword(&normalized)
            || out.iter().any(|existing| existing == &normalized)
        {
            continue;
        }
        out.push(normalized);
        if out.len() >= ASSEMBLYAI_WORD_SEARCH_MAX_TERMS {
            break;
        }
    }
    out
}

fn assemblyai_word_search_stopword(term: &str) -> bool {
    matches!(
        term,
        "about"
            | "after"
            | "again"
            | "also"
            | "because"
            | "been"
            | "being"
            | "could"
            | "from"
            | "have"
            | "into"
            | "just"
            | "like"
            | "more"
            | "only"
            | "other"
            | "over"
            | "said"
            | "should"
            | "that"
            | "their"
            | "there"
            | "they"
            | "this"
            | "through"
            | "were"
            | "what"
            | "when"
            | "where"
            | "which"
            | "with"
            | "would"
            | "your"
    )
}

fn assemblyai_word_search_payload(
    terms: &[String],
    response: Option<&AssemblyAiWordSearchResponse>,
) -> serde_json::Value {
    serde_json::json!({
        "terms": terms,
        "total_count": response.map(|response| response.total_count).unwrap_or(0),
        "match_count": response.map(|response| response.matches.len()).unwrap_or(0),
        "matches": response
            .map(|response| {
                response
                    .matches
                    .iter()
                    .map(|search_match| {
                        serde_json::json!({
                            "text": search_match.text,
                            "count": search_match.count,
                            "timestamps": search_match.timestamps,
                            "indexes": search_match.indexes,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

fn transcript_source_spans(
    matter_id: &str,
    document_id: &str,
    transcription_job_id: &str,
    segments: &[TranscriptSegment],
    document_version_id: Option<String>,
    object_blob_id: Option<String>,
    review_status: &str,
    extraction_method: &str,
) -> Vec<SourceSpan> {
    segments
        .iter()
        .map(|segment| {
            let id = format!(
                "span:{}:{}",
                sanitize_path_segment(transcription_job_id),
                segment.ordinal
            );
            SourceSpan {
                source_span_id: id.clone(),
                id,
                matter_id: matter_id.to_string(),
                document_id: document_id.to_string(),
                document_version_id: document_version_id.clone(),
                object_blob_id: object_blob_id.clone(),
                ingestion_run_id: None,
                page: None,
                chunk_id: Some(segment.segment_id.clone()),
                byte_start: None,
                byte_end: None,
                char_start: None,
                char_end: None,
                time_start_ms: Some(segment.time_start_ms),
                time_end_ms: Some(segment.time_end_ms),
                speaker_label: segment.speaker_label.clone(),
                quote: Some(if review_status == "approved" {
                    segment.text.clone()
                } else {
                    segment
                        .redacted_text
                        .clone()
                        .unwrap_or_else(|| redact_transcript_text(&segment.text))
                }),
                extraction_method: extraction_method.to_string(),
                confidence: segment.confidence,
                review_status: review_status.to_string(),
                unavailable_reason: None,
            }
        })
        .collect()
}

fn transcript_segments_to_text(segments: &[TranscriptSegment], redacted: bool) -> String {
    let mut paragraphs = Vec::new();
    let mut current_paragraph: Option<u64> = None;
    let mut current_lines = Vec::new();
    for segment in segments {
        if !current_lines.is_empty()
            && segment.paragraph_ordinal.is_some()
            && segment.paragraph_ordinal != current_paragraph
        {
            paragraphs.push(current_lines.join("\n"));
            current_lines.clear();
        }
        current_paragraph = segment.paragraph_ordinal;
        let speaker = segment
            .speaker_name
            .as_deref()
            .or(segment.speaker_label.as_deref())
            .unwrap_or("Speaker");
        let text = if redacted {
            segment
                .redacted_text
                .as_deref()
                .unwrap_or(segment.text.as_str())
        } else {
            segment.text.as_str()
        };
        current_lines.push(format!("{speaker}: {text}"));
    }
    if !current_lines.is_empty() {
        paragraphs.push(current_lines.join("\n"));
    }
    paragraphs.join("\n\n")
}

fn transcript_segments_to_vtt(segments: &[TranscriptSegment], redacted: bool) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for segment in segments {
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            caption_timestamp(segment.time_start_ms, '.'),
            caption_timestamp(segment.time_end_ms, '.'),
            caption_text(segment, redacted)
        ));
    }
    out
}

fn transcript_segments_to_srt(segments: &[TranscriptSegment], redacted: bool) -> String {
    let mut out = String::new();
    for (index, segment) in segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            caption_timestamp(segment.time_start_ms, ','),
            caption_timestamp(segment.time_end_ms, ','),
            caption_text(segment, redacted)
        ));
    }
    out
}

fn caption_text(segment: &TranscriptSegment, redacted: bool) -> String {
    let speaker = segment
        .speaker_name
        .as_deref()
        .or(segment.speaker_label.as_deref())
        .unwrap_or("Speaker");
    let text = if redacted {
        segment
            .redacted_text
            .as_deref()
            .unwrap_or(segment.text.as_str())
    } else {
        segment.text.as_str()
    };
    format!("{speaker}: {text}")
}

fn caption_timestamp(ms: u64, millis_separator: char) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{millis_separator}{millis:03}")
}

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap());
static PHONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)\d{3}[-.\s]?\d{4}\b").unwrap()
});
static SSN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());

fn redact_transcript_text(text: &str) -> String {
    let text = EMAIL_RE.replace_all(text, "[redacted email]");
    let text = PHONE_RE.replace_all(&text, "[redacted phone]");
    let text = SSN_RE.replace_all(&text, "[redacted ssn]");
    text.to_string()
}

fn assemblyai_http_error(action: &str, status: reqwest::StatusCode) -> ApiError {
    ApiError::External(format!("AssemblyAI {action} failed with HTTP {status}."))
}

fn assemblyai_transcript_error_message(provider: &AssemblyAiTranscriptResponse) -> String {
    match provider
        .error
        .as_deref()
        .and_then(sanitized_provider_error_fragment)
    {
        Some(error) => format!("AssemblyAI returned an error status: {error}"),
        None => "AssemblyAI returned an error status.".to_string(),
    }
}

fn sanitized_provider_error_fragment(message: &str) -> Option<String> {
    let normalized = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = normalized.trim();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized.chars().take(240).collect())
}

fn sanitized_external_error(error: &ApiError) -> String {
    match error {
        ApiError::External(message) if message.starts_with("AssemblyAI ") => message.clone(),
        ApiError::HttpClient(_) | ApiError::External(_) => {
            "External transcription provider request failed.".to_string()
        }
        _ => "Transcription request failed.".to_string(),
    }
}

fn transcription_warnings(job: &TranscriptionJob) -> Vec<String> {
    let mut warnings = Vec::new();
    if job.provider_mode == "disabled" {
        warnings.push("AssemblyAI transcription is disabled for this API instance.".to_string());
    }
    if matches!(job.status.as_str(), "review_ready") {
        warnings.push(
            "Transcript review is required before transcript-derived facts or evidence are created."
                .to_string(),
        );
    }
    if job.redact_pii {
        warnings.push(
            "Raw transcript artifacts are private; redacted transcript text is used for review display."
                .to_string(),
        );
    }
    warnings
}

fn failed_ingestion_run(
    run: &IngestionRun,
    stage: &str,
    error_code: &str,
    error_message: &str,
    retryable: bool,
) -> IngestionRun {
    let mut next = run.clone();
    next.status = "failed".to_string();
    next.stage = stage.to_string();
    next.completed_at = Some(now_string());
    next.error_code = Some(error_code.to_string());
    next.error_message = Some(error_message.to_string());
    next.retryable = retryable;
    next
}

fn completed_ingestion_run(
    run: &IngestionRun,
    status: &str,
    stage: &str,
    produced_node_ids: Vec<String>,
) -> IngestionRun {
    let mut next = run.clone();
    next.status = status.to_string();
    next.stage = stage.to_string();
    next.completed_at = Some(now_string());
    next.error_code = None;
    next.error_message = None;
    next.retryable = false;
    next.produced_node_ids = produced_node_ids;
    next
}

fn completed_ingestion_run_with_objects(
    run: &IngestionRun,
    status: &str,
    stage: &str,
    produced_node_ids: Vec<String>,
    produced_object_keys: Vec<String>,
) -> IngestionRun {
    let mut next = completed_ingestion_run(run, status, stage, produced_node_ids);
    next.produced_object_keys = produced_object_keys;
    next
}

fn produced_node_ids(
    chunks: &[ExtractedTextChunk],
    spans: &[SourceSpan],
    facts: &[CaseFact],
) -> Vec<String> {
    let mut ids = Vec::new();
    for chunk in chunks {
        push_unique(&mut ids, chunk.chunk_id.clone());
    }
    for span in spans {
        push_unique(&mut ids, span.source_span_id.clone());
    }
    for fact in facts {
        push_unique(&mut ids, fact.fact_id.clone());
    }
    ids
}

fn upload_id_for_document(document_id: &str) -> String {
    format!("upload:{}", sanitize_path_segment(document_id))
}

fn to_payload<T: serde::Serialize>(value: &T) -> ApiResult<String> {
    serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn from_payload<T: serde::de::DeserializeOwned>(payload: &str) -> ApiResult<T> {
    serde_json::from_str(payload).map_err(|error| ApiError::Internal(error.to_string()))
}

fn row_u64(row: &neo4rs::Row, key: &str) -> u64 {
    row.get::<i64>(key).ok().unwrap_or(0).max(0) as u64
}

fn document_file_extension(document: &CaseDocument) -> Option<String> {
    Path::new(&document.filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| sanitize_path_segment(ext).to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
}

fn document_lower_parts(document: &CaseDocument) -> (String, String) {
    (
        document.filename.to_ascii_lowercase(),
        document
            .mime_type
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase(),
    )
}

fn document_is_docx(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    lower.ends_with(".docx")
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
}

fn document_is_pdf(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    lower.ends_with(".pdf") || mime == "application/pdf"
}

fn document_is_markdown(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    lower.ends_with(".md") || lower.ends_with(".markdown") || mime == "text/markdown"
}

fn document_is_text(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    mime.starts_with("text/")
        || matches!(
            mime.as_str(),
            "application/json" | "application/xml" | "application/x-ndjson"
        )
        || [".txt", ".csv", ".html", ".htm", ".json", ".log", ".xml"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn document_is_image(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    is_image_file(&lower, &mime)
}

fn document_is_media(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    is_audio_video_file(&lower, &mime)
}

fn document_is_spreadsheet(document: &CaseDocument) -> bool {
    let (lower, mime) = document_lower_parts(document);
    mime.contains("spreadsheet")
        || mime.contains("ms-excel")
        || [".xlsx", ".xls", ".ods", ".csv"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn workspace_text_content(document: &CaseDocument, bytes: Option<&Bytes>) -> Option<String> {
    if let Some(text) = document
        .extracted_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        return Some(text.to_string());
    }
    if !(document_is_docx(document)
        || document_is_markdown(document)
        || document_is_text(document)
        || document_is_pdf(document))
    {
        return None;
    }
    bytes.and_then(|bytes| {
        parse_document_bytes(&document.filename, document.mime_type.as_deref(), bytes)
            .text
            .filter(|text| !text.trim().is_empty())
    })
}

fn document_capabilities(
    document: &CaseDocument,
    docx_manifest: Option<&DocxPackageManifest>,
) -> Vec<DocumentCapability> {
    if document_is_docx(document) {
        let editable = docx_manifest
            .map(|manifest| manifest.editable)
            .unwrap_or(true);
        return vec![
            capability("view", true, "custom_docx", None),
            capability(
                "edit",
                editable,
                "ooxml_round_trip_text",
                (!editable).then(|| {
                    "Complex DOCX objects are read-only until they are mapped safely.".to_string()
                }),
            ),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "deterministic_docx_text", None),
            capability("promote", editable, "work_product_ast", None),
        ];
    }
    if document_is_pdf(document) {
        return vec![
            capability("view", true, "pdfjs", None),
            capability("edit", false, "immutable_pdf_bytes", Some("PDF v1 keeps original bytes immutable and stores redactions/notes as CaseBuilder sidecar annotations.".to_string())),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "embedded_text_or_ocr", None),
            capability("promote", false, "pdf_text_review_required", None),
        ];
    }
    if document_is_markdown(document) {
        return vec![
            capability("view", true, "markdown_source", None),
            capability("edit", true, "markdown_ast_source", None),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "markdown_text", None),
            capability("promote", true, "work_product_ast", None),
        ];
    }
    if document_is_text(document) {
        return vec![
            capability("view", true, "plain_text", None),
            capability("edit", true, "plain_text", None),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "deterministic_text", None),
            capability("promote", true, "work_product_ast", None),
        ];
    }
    if document_is_image(document) {
        return vec![
            capability("view", true, "image_preview", None),
            capability("edit", false, "immutable_source", None),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "ocr_deferred", None),
            capability("promote", false, "ocr_required", None),
        ];
    }
    if document_is_media(document) {
        return vec![
            capability("view", true, "media_preview", None),
            capability("edit", false, "immutable_source", None),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", true, "transcription_deferred", None),
            capability("promote", false, "transcript_required", None),
        ];
    }
    if document_is_spreadsheet(document) {
        return vec![
            capability("view", false, "spreadsheet_preview_pending", None),
            capability("edit", false, "unsupported_binary", None),
            capability("annotate", true, "graph_sidecar", None),
            capability("extract", false, "spreadsheet_parser_pending", None),
            capability("promote", false, "unsupported_binary", None),
        ];
    }
    vec![
        capability("view", false, "unsupported_binary", None),
        capability("edit", false, "unsupported_binary", None),
        capability("annotate", true, "graph_sidecar", None),
        capability("extract", false, "unsupported_binary", None),
        capability("promote", false, "unsupported_binary", None),
    ]
}

fn capability(
    capability: &str,
    enabled: bool,
    mode: &str,
    reason: Option<String>,
) -> DocumentCapability {
    DocumentCapability {
        capability: capability.to_string(),
        enabled,
        mode: mode.to_string(),
        reason,
    }
}

fn parse_document_bytes(filename: &str, mime_type: Option<&str>, bytes: &[u8]) -> ParserOutcome {
    let lower = filename.to_ascii_lowercase();
    let mime = mime_type.unwrap_or_default().to_ascii_lowercase();
    if is_image_file(&lower, &mime) {
        return ParserOutcome {
            parser_id: "casebuilder-image-ocr-deferred".to_string(),
            status: "ocr_required".to_string(),
            message: "Stored privately. OCR is required before text indexing.".to_string(),
            text: None,
        };
    }
    if is_audio_video_file(&lower, &mime) {
        return ParserOutcome {
            parser_id: "casebuilder-media-transcription-deferred".to_string(),
            status: "transcription_deferred".to_string(),
            message: "Stored privately. Transcription is deferred for this media file.".to_string(),
            text: None,
        };
    }
    if lower.ends_with(".pdf") || mime == "application/pdf" || bytes.starts_with(b"%PDF") {
        let text = extract_pdf_embedded_text(bytes);
        return ParserOutcome {
            parser_id: "casebuilder-pdf-embedded-text-v1".to_string(),
            status: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "processed".to_string()
            } else {
                "ocr_required".to_string()
            },
            message: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "Stored privately. Embedded PDF text is ready for deterministic extraction."
                    .to_string()
            } else {
                "Stored privately. No embedded PDF text was detected; OCR is required.".to_string()
            },
            text,
        };
    }
    if lower.ends_with(".docx")
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    {
        let text = extract_docx_like_text(bytes);
        return ParserOutcome {
            parser_id: "casebuilder-docx-text-v1".to_string(),
            status: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "processed".to_string()
            } else {
                "unsupported".to_string()
            },
            message: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "Stored privately. DOCX XML text is ready for deterministic extraction.".to_string()
            } else {
                "Stored privately. Compressed DOCX parsing is deferred until the document parser worker is enabled."
                    .to_string()
            },
            text,
        };
    }
    let lower = filename.to_ascii_lowercase();
    let text_like_mime = mime_type
        .map(|value| {
            value.starts_with("text/")
                || matches!(
                    value,
                    "application/json" | "application/xml" | "application/x-ndjson"
                )
        })
        .unwrap_or(false);
    let text_like_extension = lower.ends_with(".txt")
        || lower.ends_with(".md")
        || lower.ends_with(".markdown")
        || lower.ends_with(".csv")
        || lower.ends_with(".html")
        || lower.ends_with(".htm")
        || lower.ends_with(".json")
        || lower.ends_with(".log");
    if !(text_like_mime || text_like_extension) {
        return ParserOutcome {
            parser_id: "casebuilder-binary-unsupported-v1".to_string(),
            status: "unsupported".to_string(),
            message: "Stored privately. This file type is not parseable in the deterministic V0 importer.".to_string(),
            text: None,
        };
    }
    let text = String::from_utf8(bytes.to_vec())
        .ok()
        .map(|text| {
            if lower.ends_with(".html") || lower.ends_with(".htm") || mime_type == Some("text/html")
            {
                strip_html_text(&text)
            } else {
                text
            }
        })
        .filter(|text| !text.trim().is_empty());
    ParserOutcome {
        parser_id: if lower.ends_with(".md") || lower.ends_with(".markdown") {
            "casebuilder-markdown-v1".to_string()
        } else if lower.ends_with(".csv") || mime_type == Some("text/csv") {
            "casebuilder-csv-text-v1".to_string()
        } else if lower.ends_with(".html")
            || lower.ends_with(".htm")
            || mime_type == Some("text/html")
        {
            "casebuilder-html-text-v1".to_string()
        } else {
            "casebuilder-plain-text-v1".to_string()
        },
        status: if text.is_some() {
            "processed"
        } else {
            "failed"
        }
        .to_string(),
        message: if text.is_some() {
            "Stored privately. Text is ready for deterministic V0 extraction.".to_string()
        } else {
            "Stored privately, but the text parser could not decode this file as UTF-8.".to_string()
        },
        text,
    }
}

fn is_image_file(lower: &str, mime: &str) -> bool {
    mime.starts_with("image/")
        || [
            ".png", ".jpg", ".jpeg", ".gif", ".webp", ".heic", ".tif", ".tiff",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn is_audio_video_file(lower: &str, mime: &str) -> bool {
    mime.starts_with("audio/")
        || mime.starts_with("video/")
        || [".mp3", ".m4a", ".wav", ".mp4", ".mov", ".m4v", ".webm"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn strip_html_text(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;
    for ch in text.chars() {
        match ch {
            '<' if !in_entity => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                out.push(match entity.as_str() {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    "nbsp" => ' ',
                    _ => ' ',
                });
            }
            _ if in_tag => {}
            _ if in_entity => entity.push(ch),
            _ => out.push(ch),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_pdf_embedded_text(bytes: &[u8]) -> Option<String> {
    let raw = String::from_utf8_lossy(bytes);
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_literal = false;
    let mut escaped = false;
    for ch in raw.chars() {
        if in_literal {
            if escaped {
                current.push(match ch {
                    'n' | 'r' | 't' => ' ',
                    value => value,
                });
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == ')' {
                let cleaned = current.split_whitespace().collect::<Vec<_>>().join(" ");
                if cleaned
                    .chars()
                    .filter(|value| value.is_ascii_alphabetic())
                    .count()
                    >= 3
                {
                    values.push(cleaned);
                }
                current.clear();
                in_literal = false;
            } else {
                current.push(ch);
            }
        } else if ch == '(' {
            in_literal = true;
        }
    }
    let text = values.join("\n");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_docx_like_text(bytes: &[u8]) -> Option<String> {
    extract_docx_zip_text(bytes).or_else(|| {
        let raw = String::from_utf8(bytes.to_vec()).ok()?;
        extract_docx_xml_text(&raw)
    })
}

fn extract_docx_zip_text(bytes: &[u8]) -> Option<String> {
    let package = read_zip_package(bytes)?;
    let mut parts = Vec::new();

    for entry in &package.entries {
        if is_docx_text_part(&entry.name) {
            let entry = read_zip_entry(bytes, entry)?;
            let raw = String::from_utf8(entry).ok()?;
            if let Some(text) = extract_docx_xml_text(&raw) {
                parts.push(text);
            }
        }
    }

    let text = parts.join("\n");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_docx_xml_text(raw: &str) -> Option<String> {
    if !raw.contains("<w:t") && !raw.contains("<text") {
        return None;
    }
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = raw[cursor..].find('>') {
        let tag_end = cursor + start_rel + 1;
        let tag_start = raw[..tag_end].rfind('<').unwrap_or(cursor);
        let tag = &raw[tag_start..tag_end];
        if tag.starts_with("<w:t") || tag.starts_with("<text") {
            if let Some(end_rel) = raw[tag_end..].find("</") {
                out.push_str(&decode_xml_text(&raw[tag_end..tag_end + end_rel]));
                out.push(' ');
                cursor = tag_end + end_rel;
                continue;
            }
        }
        cursor = tag_end;
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() { None } else { Some(text) }
}

fn is_docx_text_part(name: &str) -> bool {
    name == "word/document.xml"
        || name == "word/footnotes.xml"
        || name == "word/endnotes.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

fn docx_package_manifest(
    document: &CaseDocument,
    version: Option<&DocumentVersion>,
    bytes: &[u8],
) -> ApiResult<DocxPackageManifest> {
    let package = read_zip_package(bytes).ok_or_else(|| {
        ApiError::BadRequest("DOCX package central directory is not readable.".to_string())
    })?;
    let mut unsupported_features = Vec::new();
    let mut text_preview = None;
    let mut text_part_count = 0_u64;
    let mut has_document_xml = false;
    for entry in &package.entries {
        if is_docx_text_part(&entry.name) {
            text_part_count += 1;
        }
        if entry.name == "word/document.xml" {
            has_document_xml = true;
            let raw = read_zip_entry(bytes, entry)
                .and_then(|entry_bytes| String::from_utf8(entry_bytes).ok())
                .unwrap_or_default();
            unsupported_features = docx_unsupported_features(&raw);
            text_preview = extract_docx_xml_text(&raw);
        }
    }
    let entries = package
        .entries
        .iter()
        .map(|entry| DocxPackageEntry {
            name: entry.name.clone(),
            size_bytes: entry.uncompressed_size as u64,
            compressed_size_bytes: entry.compressed_size as u64,
            compression: zip_compression_label(entry.compression),
            supported_text_part: is_docx_text_part(&entry.name),
        })
        .collect::<Vec<_>>();

    Ok(DocxPackageManifest {
        document_id: document.document_id.clone(),
        document_version_id: version.map(|version| version.document_version_id.clone()),
        entry_count: entries.len() as u64,
        text_part_count,
        editable: has_document_xml && unsupported_features.is_empty(),
        unsupported_features,
        entries,
        text_preview,
    })
}

fn docx_with_replaced_document_xml(bytes: &[u8], text: &str) -> ApiResult<(Vec<u8>, Vec<String>)> {
    let package = read_zip_package(bytes).ok_or_else(|| {
        ApiError::BadRequest("DOCX package central directory is not readable.".to_string())
    })?;
    let document_entry = package
        .entries
        .iter()
        .find(|entry| entry.name == "word/document.xml")
        .ok_or_else(|| {
            ApiError::BadRequest("DOCX package is missing word/document.xml.".to_string())
        })?;
    let existing_document_xml = read_zip_entry(bytes, document_entry)
        .and_then(|entry_bytes| String::from_utf8(entry_bytes).ok())
        .unwrap_or_default();
    let unsupported = docx_unsupported_features(&existing_document_xml);
    if !unsupported.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "DOCX contains unsupported complex OOXML features: {}",
            unsupported.join(", ")
        )));
    }
    let replacement = docx_document_xml_from_text(text).into_bytes();
    let mut out = Vec::new();
    let mut central_records = Vec::new();
    let mut entries = package.entries.clone();
    entries.sort_by_key(|entry| entry.local_header_offset);

    for (index, entry) in entries.iter().enumerate() {
        let local_header_offset = out.len() as u32;
        if entry.name == "word/document.xml" {
            write_stored_zip_local_entry(&mut out, &entry.name, &replacement);
            central_records.push(ZipCentralRecord {
                name: entry.name.clone(),
                version_made_by: entry.version_made_by,
                version_needed: 20,
                flags: 0,
                compression: 0,
                last_modified_time: entry.last_modified_time,
                last_modified_date: entry.last_modified_date,
                crc32: crc32(&replacement),
                compressed_size: replacement.len() as u32,
                uncompressed_size: replacement.len() as u32,
                internal_attrs: entry.internal_attrs,
                external_attrs: entry.external_attrs,
                local_header_offset,
            });
            continue;
        }
        let local_end = zip_local_entry_end(&package, &entries, index).ok_or_else(|| {
            ApiError::BadRequest(format!("DOCX ZIP entry {} is not readable.", entry.name))
        })?;
        let local_start = entry.local_header_offset;
        if local_start > local_end || local_end > bytes.len() {
            return Err(ApiError::BadRequest(format!(
                "DOCX ZIP entry {} has invalid byte offsets.",
                entry.name
            )));
        }
        out.extend_from_slice(&bytes[local_start..local_end]);
        central_records.push(ZipCentralRecord {
            name: entry.name.clone(),
            version_made_by: entry.version_made_by,
            version_needed: entry.version_needed,
            flags: entry.flags,
            compression: entry.compression,
            last_modified_time: entry.last_modified_time,
            last_modified_date: entry.last_modified_date,
            crc32: entry.crc32,
            compressed_size: entry.compressed_size as u32,
            uncompressed_size: entry.uncompressed_size as u32,
            internal_attrs: entry.internal_attrs,
            external_attrs: entry.external_attrs,
            local_header_offset,
        });
    }
    write_zip_central_directory(&mut out, &central_records)?;
    Ok((
        out,
        vec![
            "DOCX save rewrote word/document.xml and copied unmapped package entries forward."
                .to_string(),
        ],
    ))
}

fn read_zip_package(bytes: &[u8]) -> Option<ZipPackage> {
    let eocd = find_zip_eocd(bytes)?;
    let entry_count = le_u16(bytes, eocd + 10)? as usize;
    let central_directory_offset = le_u32(bytes, eocd + 16)? as usize;
    let mut cursor = central_directory_offset;
    let mut entries = Vec::new();

    for _ in 0..entry_count {
        if cursor + 46 > bytes.len() || le_u32(bytes, cursor)? != 0x0201_4b50 {
            return None;
        }
        let version_made_by = le_u16(bytes, cursor + 4)?;
        let version_needed = le_u16(bytes, cursor + 6)?;
        let flags = le_u16(bytes, cursor + 8)?;
        let compression = le_u16(bytes, cursor + 10)?;
        let last_modified_time = le_u16(bytes, cursor + 12)?;
        let last_modified_date = le_u16(bytes, cursor + 14)?;
        let crc32 = le_u32(bytes, cursor + 16)?;
        let compressed_size = le_u32(bytes, cursor + 20)? as usize;
        let uncompressed_size = le_u32(bytes, cursor + 24)? as usize;
        let name_len = le_u16(bytes, cursor + 28)? as usize;
        let extra_len = le_u16(bytes, cursor + 30)? as usize;
        let comment_len = le_u16(bytes, cursor + 32)? as usize;
        let internal_attrs = le_u16(bytes, cursor + 36)?;
        let external_attrs = le_u32(bytes, cursor + 38)?;
        let local_header_offset = le_u32(bytes, cursor + 42)? as usize;
        let name_start = cursor + 46;
        let name_end = name_start.checked_add(name_len)?;
        let next = name_end.checked_add(extra_len)?.checked_add(comment_len)?;
        if next > bytes.len() {
            return None;
        }
        let name = std::str::from_utf8(&bytes[name_start..name_end])
            .ok()?
            .to_string();
        entries.push(ZipEntryRecord {
            name,
            version_made_by,
            version_needed,
            flags,
            compression,
            last_modified_time,
            last_modified_date,
            crc32,
            compressed_size,
            uncompressed_size,
            internal_attrs,
            external_attrs,
            local_header_offset,
        });
        cursor = next;
    }
    Some(ZipPackage {
        entries,
        central_directory_offset,
    })
}

fn read_zip_entry(bytes: &[u8], entry: &ZipEntryRecord) -> Option<Vec<u8>> {
    let local_header_offset = entry.local_header_offset;
    if local_header_offset + 30 > bytes.len() || le_u32(bytes, local_header_offset)? != 0x0403_4b50
    {
        return None;
    }
    let name_len = le_u16(bytes, local_header_offset + 26)? as usize;
    let extra_len = le_u16(bytes, local_header_offset + 28)? as usize;
    let data_start = local_header_offset
        .checked_add(30)?
        .checked_add(name_len)?
        .checked_add(extra_len)?;
    let data_end = data_start.checked_add(entry.compressed_size)?;
    if data_end > bytes.len() {
        return None;
    }
    let payload = &bytes[data_start..data_end];
    match entry.compression {
        0 => Some(payload.to_vec()),
        8 => {
            let mut decoder = DeflateDecoder::new(Cursor::new(payload));
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).ok()?;
            Some(out)
        }
        _ => None,
    }
}

fn zip_local_entry_end(
    package: &ZipPackage,
    ordered_entries: &[ZipEntryRecord],
    index: usize,
) -> Option<usize> {
    ordered_entries
        .iter()
        .skip(index + 1)
        .find(|entry| entry.local_header_offset > ordered_entries[index].local_header_offset)
        .map(|entry| entry.local_header_offset)
        .or(Some(package.central_directory_offset))
}

fn zip_compression_label(compression: u16) -> String {
    match compression {
        0 => "stored".to_string(),
        8 => "deflated".to_string(),
        other => format!("unsupported:{other}"),
    }
}

fn docx_unsupported_features(document_xml: &str) -> Vec<String> {
    let mut features = Vec::new();
    for (needle, label) in [
        ("<w:tbl", "tables"),
        ("<w:drawing", "drawings"),
        ("<w:pict", "legacy_pictures"),
        ("<w:object", "embedded_objects"),
        ("<w:sdt", "content_controls"),
        ("<w:txbxContent", "text_boxes"),
    ] {
        if document_xml.contains(needle) {
            features.push(label.to_string());
        }
    }
    features
}

fn docx_document_xml_from_text(text: &str) -> String {
    let mut body = String::new();
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            body.push_str("<w:p/>");
        } else {
            body.push_str("<w:p><w:r><w:t xml:space=\"preserve\">");
            body.push_str(&encode_xml_text(trimmed));
            body.push_str("</w:t></w:r></w:p>");
        }
    }
    if body.is_empty() {
        body.push_str("<w:p/>");
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body>{body}<w:sectPr/></w:body></w:document>"
    )
}

fn write_stored_zip_local_entry(out: &mut Vec<u8>, name: &str, payload: &[u8]) {
    out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    push_le_u16(out, 20);
    push_le_u16(out, 0);
    push_le_u16(out, 0);
    push_le_u16(out, 0);
    push_le_u16(out, 0);
    push_le_u32(out, crc32(payload));
    push_le_u32(out, payload.len() as u32);
    push_le_u32(out, payload.len() as u32);
    push_le_u16(out, name.len() as u16);
    push_le_u16(out, 0);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(payload);
}

fn write_zip_central_directory(out: &mut Vec<u8>, records: &[ZipCentralRecord]) -> ApiResult<()> {
    let central_start = out.len();
    for record in records {
        if record.name.len() > u16::MAX as usize {
            return Err(ApiError::BadRequest(
                "ZIP entry name is too long for DOCX writer.".to_string(),
            ));
        }
        out.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        push_le_u16(out, record.version_made_by);
        push_le_u16(out, record.version_needed);
        push_le_u16(out, record.flags);
        push_le_u16(out, record.compression);
        push_le_u16(out, record.last_modified_time);
        push_le_u16(out, record.last_modified_date);
        push_le_u32(out, record.crc32);
        push_le_u32(out, record.compressed_size);
        push_le_u32(out, record.uncompressed_size);
        push_le_u16(out, record.name.len() as u16);
        push_le_u16(out, 0);
        push_le_u16(out, 0);
        push_le_u16(out, 0);
        push_le_u16(out, record.internal_attrs);
        push_le_u32(out, record.external_attrs);
        push_le_u32(out, record.local_header_offset);
        out.extend_from_slice(record.name.as_bytes());
    }
    let central_size = out.len().saturating_sub(central_start);
    if records.len() > u16::MAX as usize
        || central_start > u32::MAX as usize
        || central_size > u32::MAX as usize
    {
        return Err(ApiError::BadRequest(
            "DOCX package is too large for the v1 ZIP writer.".to_string(),
        ));
    }
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    push_le_u16(out, 0);
    push_le_u16(out, 0);
    push_le_u16(out, records.len() as u16);
    push_le_u16(out, records.len() as u16);
    push_le_u32(out, central_size as u32);
    push_le_u32(out, central_start as u32);
    push_le_u16(out, 0);
    Ok(())
}

fn find_zip_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let earliest = bytes.len().saturating_sub(22 + 65_535);
    (earliest..=bytes.len() - 22)
        .rev()
        .find(|offset| le_u32(bytes, *offset) == Some(0x0605_4b50))
}

fn le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

fn le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn push_le_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_le_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn decode_xml_text(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn encode_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn summarize_text(text: &str) -> String {
    let summary = text
        .split_whitespace()
        .take(60)
        .collect::<Vec<_>>()
        .join(" ");
    if summary.len() < text.len() {
        format!("{summary}...")
    } else {
        summary
    }
}

fn chunk_text(document_id: &str, text: &str) -> Vec<ExtractedTextChunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut current_end = 0usize;
    let mut cursor = 0usize;
    let mut index = 1;
    for line in text.split_inclusive('\n') {
        if current.len() + line.len() > 1800 && !current.is_empty() {
            let (chunk_text, byte_start, byte_end, char_start, char_end) =
                trim_offsets(text, &current, current_start, current_end);
            chunks.push(ExtractedTextChunk {
                chunk_id: format!("chunk:{document_id}:{index}"),
                document_id: document_id.to_string(),
                page: index,
                text: chunk_text,
                document_version_id: None,
                object_blob_id: None,
                source_span_id: None,
                byte_start: Some(byte_start),
                byte_end: Some(byte_end),
                char_start: Some(char_start),
                char_end: Some(char_end),
            });
            current.clear();
            index += 1;
        }
        if current.is_empty() {
            current_start = cursor;
        }
        current.push_str(line);
        cursor += line.len();
        current_end = cursor;
    }
    if !current.trim().is_empty() {
        let (chunk_text, byte_start, byte_end, char_start, char_end) =
            trim_offsets(text, &current, current_start, current_end);
        chunks.push(ExtractedTextChunk {
            chunk_id: format!("chunk:{document_id}:{index}"),
            document_id: document_id.to_string(),
            page: index,
            text: chunk_text,
            document_version_id: None,
            object_blob_id: None,
            source_span_id: None,
            byte_start: Some(byte_start),
            byte_end: Some(byte_end),
            char_start: Some(char_start),
            char_end: Some(char_end),
        });
    }
    chunks
}

fn trim_offsets(
    text: &str,
    current: &str,
    start: usize,
    end: usize,
) -> (String, u64, u64, u64, u64) {
    let leading = current.len() - current.trim_start().len();
    let trailing = current.len() - current.trim_end().len();
    let byte_start = start + leading;
    let byte_end = end.saturating_sub(trailing);
    let chunk_text = text
        .get(byte_start..byte_end)
        .unwrap_or_else(|| current.trim())
        .to_string();
    (
        chunk_text,
        byte_start as u64,
        byte_end as u64,
        text[..byte_start].chars().count() as u64,
        text[..byte_end].chars().count() as u64,
    )
}

fn propose_facts(
    matter_id: &str,
    document_id: &str,
    text: &str,
    context: &SourceContext,
) -> Vec<CaseFact> {
    sentence_candidates_with_offsets(text)
        .into_iter()
        .take(24)
        .enumerate()
        .map(|(index, sentence)| {
            let ordinal = index as u64 + 1;
            let fact_id = format!("fact:{}:{}", sanitize_path_segment(document_id), ordinal);
            let source_span =
                source_span_for_sentence(matter_id, document_id, ordinal, &sentence, context);
            CaseFact {
                id: fact_id.clone(),
                fact_id,
                matter_id: matter_id.to_string(),
                statement: sentence.text.clone(),
                text: sentence.text,
                status: "proposed".to_string(),
                confidence: 0.55,
                date: None,
                party_id: None,
                source_document_ids: vec![document_id.to_string()],
                source_evidence_ids: Vec::new(),
                contradicted_by_evidence_ids: Vec::new(),
                supports_claim_ids: Vec::new(),
                supports_defense_ids: Vec::new(),
                used_in_draft_ids: Vec::new(),
                needs_verification: true,
                source_spans: vec![source_span],
                notes: Some(
                    "Deterministic V0 extraction from document text; user review required."
                        .to_string(),
                ),
            }
        })
        .collect()
}

fn timeline_suggestions_from_facts(
    matter_id: &str,
    document_id: Option<&str>,
    facts: &[CaseFact],
    chunks: &[ExtractedTextChunk],
    source_type: &str,
    work_product_id: Option<&str>,
    block_id: Option<&str>,
    agent_run_id: Option<&str>,
    index_run_id: Option<&str>,
    limit: usize,
) -> Vec<TimelineSuggestion> {
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();
    for fact in facts {
        if suggestions.len() >= limit {
            break;
        }
        let Some(date) = date_candidates_in_text(&fact.statement).into_iter().next() else {
            continue;
        };
        let source_document_id = document_id
            .map(str::to_string)
            .or_else(|| fact.source_document_ids.first().cloned());
        let source_span_ids = fact
            .source_spans
            .iter()
            .map(|span| span.source_span_id.clone())
            .collect::<Vec<_>>();
        let mut text_chunk_ids = fact
            .source_spans
            .iter()
            .filter_map(|span| span.chunk_id.clone())
            .collect::<Vec<_>>();
        for chunk_id in text_chunk_ids_for_range(chunks, fact.source_spans.first()) {
            push_unique(&mut text_chunk_ids, chunk_id);
        }
        let key = format!(
            "{}:{}:{}",
            date.iso_date,
            source_document_id.clone().unwrap_or_default(),
            normalize_for_match(&fact.statement)
        );
        if !seen.insert(key.clone()) {
            continue;
        }
        let now = now_string();
        let suggestion_id = timeline_suggestion_id(&format!("{matter_id}:{source_type}:{key}"));
        suggestions.push(TimelineSuggestion {
            id: suggestion_id.clone(),
            suggestion_id,
            matter_id: matter_id.to_string(),
            date: date.iso_date,
            date_text: date.date_text,
            date_confidence: date.confidence,
            title: timeline_title_for_text(&fact.statement),
            description: Some(fact.statement.clone()),
            kind: timeline_kind_for_text(&fact.statement),
            source_type: source_type.to_string(),
            source_document_id,
            source_span_ids,
            text_chunk_ids,
            linked_fact_ids: vec![fact.fact_id.clone()],
            linked_claim_ids: fact.supports_claim_ids.clone(),
            work_product_id: work_product_id.map(str::to_string),
            block_id: block_id.map(str::to_string),
            agent_run_id: agent_run_id.map(str::to_string),
            index_run_id: index_run_id.map(str::to_string),
            status: "suggested".to_string(),
            warnings: date.warnings,
            approved_event_id: None,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    suggestions
}

fn timeline_suggestions_from_text(
    matter_id: &str,
    text: &str,
    source_type: &str,
    source_document_id: Option<&str>,
    source_span_ids: Vec<String>,
    text_chunk_ids: Vec<String>,
    linked_fact_ids: Vec<String>,
    linked_claim_ids: Vec<String>,
    work_product_id: Option<&str>,
    block_id: Option<&str>,
    agent_run_id: Option<&str>,
    index_run_id: Option<&str>,
    limit: usize,
) -> Vec<TimelineSuggestion> {
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();
    for sentence in sentence_candidates_with_offsets(text) {
        if suggestions.len() >= limit {
            break;
        }
        let Some(date) = date_candidates_in_text(&sentence.text).into_iter().next() else {
            continue;
        };
        let sentence_text = sentence.text.clone();
        let key = format!(
            "{}:{}:{}:{}",
            date.iso_date,
            source_document_id.unwrap_or_default(),
            block_id.unwrap_or_default(),
            normalize_for_match(&sentence_text)
        );
        if !seen.insert(key.clone()) {
            continue;
        }
        let now = now_string();
        let suggestion_id = timeline_suggestion_id(&format!("{matter_id}:{source_type}:{key}"));
        suggestions.push(TimelineSuggestion {
            id: suggestion_id.clone(),
            suggestion_id,
            matter_id: matter_id.to_string(),
            date: date.iso_date,
            date_text: date.date_text,
            date_confidence: date.confidence,
            title: timeline_title_for_text(&sentence_text),
            description: Some(sentence_text.clone()),
            kind: timeline_kind_for_text(&sentence_text),
            source_type: source_type.to_string(),
            source_document_id: source_document_id.map(str::to_string),
            source_span_ids: source_span_ids.clone(),
            text_chunk_ids: text_chunk_ids.clone(),
            linked_fact_ids: linked_fact_ids.clone(),
            linked_claim_ids: linked_claim_ids.clone(),
            work_product_id: work_product_id.map(str::to_string),
            block_id: block_id.map(str::to_string),
            agent_run_id: agent_run_id.map(str::to_string),
            index_run_id: index_run_id.map(str::to_string),
            status: "suggested".to_string(),
            warnings: date.warnings,
            approved_event_id: None,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    suggestions
}

fn date_entity_mentions_for_chunk(
    matter_id: &str,
    document_id: &str,
    chunk: &ExtractedTextChunk,
) -> Vec<EntityMention> {
    date_candidates_in_text(&chunk.text)
        .into_iter()
        .enumerate()
        .map(|(index, date)| {
            let entity_mention_id = format!(
                "entity-mention:{}:date:{}",
                sanitize_path_segment(&chunk.chunk_id),
                index + 1
            );
            let byte_base = chunk.byte_start.unwrap_or_default();
            let char_base = chunk.char_start.unwrap_or_default();
            EntityMention {
                id: entity_mention_id.clone(),
                entity_mention_id,
                matter_id: matter_id.to_string(),
                document_id: document_id.to_string(),
                text_chunk_id: Some(chunk.chunk_id.clone()),
                source_span_id: chunk.source_span_id.clone(),
                mention_text: date.date_text,
                entity_type: "date".to_string(),
                confidence: date.confidence,
                byte_start: Some(byte_base + date.byte_start),
                byte_end: Some(byte_base + date.byte_end),
                char_start: Some(char_base + date.char_start),
                char_end: Some(char_base + date.char_end),
                review_status: "unreviewed".to_string(),
            }
        })
        .collect()
}

fn text_chunk_ids_for_range(
    chunks: &[ExtractedTextChunk],
    span: Option<&SourceSpan>,
) -> Vec<String> {
    let Some(span) = span else {
        return Vec::new();
    };
    if let Some(chunk_id) = &span.chunk_id {
        return vec![chunk_id.clone()];
    }
    let Some(start) = span.char_start else {
        return Vec::new();
    };
    let Some(end) = span.char_end else {
        return Vec::new();
    };
    chunks
        .iter()
        .filter(|chunk| {
            let chunk_start = chunk.char_start.unwrap_or_default();
            let chunk_end = chunk.char_end.unwrap_or(chunk_start);
            chunk_start < end && chunk_end > start
        })
        .map(|chunk| chunk.chunk_id.clone())
        .collect()
}

fn date_candidates_in_text(text: &str) -> Vec<DateCandidate> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for capture in ISO_DATE_RE.captures_iter(text) {
        let Some(matched) = capture.get(0) else {
            continue;
        };
        let iso = matched.as_str().to_string();
        if days_from_iso_date(&iso).is_none() || !seen.insert((iso.clone(), matched.start())) {
            continue;
        }
        out.push(date_candidate(
            text,
            matched.start(),
            matched.end(),
            iso,
            0.99,
            Vec::new(),
        ));
    }
    for capture in MONTH_DATE_RE.captures_iter(text) {
        let Some(matched) = capture.get(0) else {
            continue;
        };
        let Some(month) = capture
            .get(1)
            .and_then(|value| month_number(value.as_str()))
        else {
            continue;
        };
        let Some(day) = capture
            .get(2)
            .and_then(|value| value.as_str().parse::<u32>().ok())
        else {
            continue;
        };
        let Some(year) = capture
            .get(3)
            .and_then(|value| value.as_str().parse::<i32>().ok())
        else {
            continue;
        };
        let iso = format!("{year:04}-{month:02}-{day:02}");
        if days_from_iso_date(&iso).is_none() || !seen.insert((iso.clone(), matched.start())) {
            continue;
        }
        out.push(date_candidate(
            text,
            matched.start(),
            matched.end(),
            iso,
            0.9,
            Vec::new(),
        ));
    }
    for capture in NUMERIC_DATE_RE.captures_iter(text) {
        let Some(matched) = capture.get(0) else {
            continue;
        };
        let Some(month) = capture
            .get(1)
            .and_then(|value| value.as_str().parse::<u32>().ok())
        else {
            continue;
        };
        let Some(day) = capture
            .get(2)
            .and_then(|value| value.as_str().parse::<u32>().ok())
        else {
            continue;
        };
        let Some(year) = capture
            .get(3)
            .and_then(|value| value.as_str().parse::<i32>().ok())
        else {
            continue;
        };
        let iso = format!("{year:04}-{month:02}-{day:02}");
        if days_from_iso_date(&iso).is_none() || !seen.insert((iso.clone(), matched.start())) {
            continue;
        }
        out.push(date_candidate(
            text,
            matched.start(),
            matched.end(),
            iso,
            0.65,
            vec!["numeric_date_format_needs_review".to_string()],
        ));
    }
    out.sort_by_key(|date| date.byte_start);
    out
}

fn date_candidate(
    text: &str,
    byte_start: usize,
    byte_end: usize,
    iso_date: String,
    confidence: f32,
    warnings: Vec<String>,
) -> DateCandidate {
    DateCandidate {
        iso_date,
        date_text: text
            .get(byte_start..byte_end)
            .unwrap_or_default()
            .to_string(),
        confidence,
        byte_start: byte_start as u64,
        byte_end: byte_end as u64,
        char_start: text[..byte_start].chars().count() as u64,
        char_end: text[..byte_end].chars().count() as u64,
        warnings,
    }
}

fn month_number(value: &str) -> Option<u32> {
    match value.trim_end_matches('.').to_ascii_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn timeline_suggestion_id(seed: &str) -> String {
    format!("timeline-suggestion:{}", hex_prefix(seed.as_bytes(), 24))
}

fn timeline_event_id_from_suggestion(suggestion_id: &str) -> String {
    format!("event:{}", hex_prefix(suggestion_id.as_bytes(), 24))
}

fn timeline_title_for_text(text: &str) -> String {
    let mut value = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for date in date_candidates_in_text(&value) {
        value = value.replace(&date.date_text, "").trim().to_string();
    }
    let value = value.trim_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':' | '-' | ' '));
    if value.is_empty() {
        "Timeline event".to_string()
    } else {
        text_excerpt(value, 96)
    }
}

fn timeline_kind_for_text(text: &str) -> String {
    let normalized = text.to_ascii_lowercase();
    if normalized.contains("notice") {
        "notice"
    } else if normalized.contains("filed") || normalized.contains("filing") {
        "filing"
    } else if normalized.contains("served") || normalized.contains("service") {
        "service"
    } else if normalized.contains("paid")
        || normalized.contains("payment")
        || normalized.contains("rent")
    {
        "payment"
    } else if normalized.contains("called")
        || normalized.contains("emailed")
        || normalized.contains("texted")
        || normalized.contains("sent")
    {
        "communication"
    } else if normalized.contains("hearing") || normalized.contains("court") {
        "court"
    } else if normalized.contains("deadline") || normalized.contains("due") {
        "deadline"
    } else {
        "other"
    }
    .to_string()
}

fn sentence_candidates_with_offsets(text: &str) -> Vec<SentenceCandidate> {
    let mut candidates = Vec::new();
    let mut current = String::new();
    let mut sentence_start = 0usize;
    let mut cursor = 0usize;

    for ch in text.chars() {
        if current.is_empty() {
            sentence_start = cursor;
        }
        current.push(ch);
        cursor += ch.len_utf8();
        if matches!(ch, '.' | '?' | '!' | '\n') {
            push_sentence_candidate(&mut candidates, text, &current, sentence_start, cursor);
            current.clear();
        }
    }
    push_sentence_candidate(&mut candidates, text, &current, sentence_start, cursor);
    candidates
}

fn push_sentence_candidate(
    candidates: &mut Vec<SentenceCandidate>,
    full_text: &str,
    sentence: &str,
    start: usize,
    end: usize,
) {
    let leading = sentence.len() - sentence.trim_start().len();
    let trailing = sentence.len() - sentence.trim_end().len();
    let byte_start = start + leading;
    let byte_end = end.saturating_sub(trailing);
    let cleaned = full_text
        .get(byte_start..byte_end)
        .unwrap_or_else(|| sentence.trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.len() < 35 || cleaned.len() > 400 {
        return;
    }
    if !cleaned.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return;
    }
    if cleaned.ends_with(':') {
        return;
    }
    if !candidates.iter().any(|existing| existing.text == cleaned) {
        candidates.push(SentenceCandidate {
            text: cleaned,
            byte_start: byte_start as u64,
            byte_end: byte_end as u64,
            char_start: full_text[..byte_start].chars().count() as u64,
            char_end: full_text[..byte_end].chars().count() as u64,
        });
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn support_anchor_id_matches(anchor: &WorkProductAnchor, anchor_id: &str) -> bool {
    anchor.anchor_id == anchor_id || anchor.id == anchor_id
}

fn apply_work_product_support_update(
    product: &mut WorkProduct,
    anchor_id: &str,
    request: PatchWorkProductSupportRequest,
) -> ApiResult<WorkProductAnchor> {
    let anchor_index = product
        .anchors
        .iter()
        .position(|anchor| support_anchor_id_matches(anchor, anchor_id))
        .ok_or_else(|| ApiError::NotFound("Work product support link not found".to_string()))?;
    let anchor = product
        .anchors
        .get_mut(anchor_index)
        .expect("anchor index was resolved");
    if let Some(value) = request.relation {
        let relation = value.trim();
        if relation.is_empty() {
            return Err(ApiError::BadRequest(
                "Support relation cannot be empty.".to_string(),
            ));
        }
        anchor.relation = relation.to_string();
    }
    if let Some(value) = request.status {
        let status = value.trim();
        if status.is_empty() {
            return Err(ApiError::BadRequest(
                "Support status cannot be empty.".to_string(),
            ));
        }
        anchor.status = status.to_string();
    }
    apply_support_optional_string_patch(&mut anchor.citation, request.citation, "citation")?;
    apply_support_optional_string_patch(
        &mut anchor.canonical_id,
        request.canonical_id,
        "canonical_id",
    )?;
    apply_support_optional_string_patch(&mut anchor.pinpoint, request.pinpoint, "pinpoint")?;
    apply_support_optional_string_patch(&mut anchor.quote, request.quote, "quote")?;
    let updated_anchor = anchor.clone();
    sync_work_product_anchor_projection(product, &updated_anchor);
    rebuild_work_product_ast_from_projection(product);
    Ok(updated_anchor)
}

fn apply_support_optional_string_patch(
    target: &mut Option<String>,
    patch: NullableStringPatch,
    field_name: &str,
) -> ApiResult<()> {
    match patch {
        NullableStringPatch::Unset => Ok(()),
        NullableStringPatch::Clear => {
            *target = None;
            Ok(())
        }
        NullableStringPatch::Set(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(ApiError::BadRequest(format!(
                    "Support {field_name} cannot be empty; send null to clear it."
                )));
            }
            *target = Some(trimmed.to_string());
            Ok(())
        }
    }
}

fn apply_work_product_support_removal(
    product: &mut WorkProduct,
    anchor_id: &str,
) -> ApiResult<WorkProductAnchor> {
    let anchor_index = product
        .anchors
        .iter()
        .position(|anchor| support_anchor_id_matches(anchor, anchor_id))
        .ok_or_else(|| ApiError::NotFound("Work product support link not found".to_string()))?;
    let removed_anchor = product.anchors.remove(anchor_index);
    let removed_mark_id = format!("{}:mark", removed_anchor.anchor_id);
    product.marks.retain(|mark| {
        mark.target_id != removed_anchor.anchor_id
            && mark.target_id != removed_anchor.id
            && mark.mark_id != removed_mark_id
            && mark.id != removed_mark_id
    });
    for block in &mut product.blocks {
        if !support_anchor_targets_block(&removed_anchor, block) {
            continue;
        }
        block.mark_ids.retain(|mark_id| {
            mark_id != &removed_anchor.anchor_id
                && mark_id != &removed_anchor.id
                && mark_id != &removed_mark_id
        });
        remove_anchor_from_block_projection(block, &removed_anchor, &product.anchors);
    }
    rebuild_work_product_ast_from_projection(product);
    Ok(removed_anchor)
}

fn apply_work_product_text_range_link(
    product: &mut WorkProduct,
    request: WorkProductTextRangeLinkRequest,
) -> ApiResult<Vec<AstOperation>> {
    normalize_work_product_ast(product);
    let block = find_ast_block(&product.document_ast.blocks, &request.block_id)
        .ok_or_else(|| ApiError::NotFound("AST text range source block not found".to_string()))?;
    let selected_quote = validate_text_range_request(block, &request)?;
    let now = now_string();
    let creates_citation = text_range_request_creates_citation(&request);
    let creates_exhibit = text_range_request_creates_exhibit(&request);
    let range = TextRange {
        start_offset: request.start_offset,
        end_offset: request.end_offset,
        quote: Some(selected_quote),
    };
    let relation = trimmed_optional_string(request.relation.as_deref())
        .unwrap_or_else(|| "supports".to_string());
    let suffix = sanitize_path_segment(&format!(
        "{}:{}:{}:{}",
        request.start_offset, request.end_offset, request.target_type, request.target_id
    ));
    let link_id = format!("{}:range-link:{suffix}", request.block_id);
    let link = WorkProductLink {
        link_id: link_id.clone(),
        source_block_id: request.block_id.clone(),
        source_text_range: Some(range.clone()),
        target_type: request.target_type.clone(),
        target_id: request.target_id.clone(),
        relation,
        confidence: None,
        created_by: "user".to_string(),
        created_at: now.clone(),
    };
    let mut operation_capacity = 1;
    if creates_citation {
        operation_capacity += 1;
    }
    if creates_exhibit {
        operation_capacity += 1;
    }
    let mut operations = Vec::with_capacity(operation_capacity);
    operations.push(AstOperation::AddLink { link });

    if creates_citation {
        let canonical_id = trimmed_optional_string(request.canonical_id.as_deref());
        let citation_text = trimmed_optional_string(request.citation.as_deref())
            .unwrap_or_else(|| request.target_id.clone());
        let citation_use_id = format!("{}:citation:{suffix}", request.block_id);
        operations.push(AstOperation::AddCitation {
            citation: WorkProductCitationUse {
                citation_use_id,
                source_block_id: request.block_id.clone(),
                source_text_range: Some(range.clone()),
                raw_text: citation_text.clone(),
                normalized_citation: Some(citation_text),
                target_type: if request.target_type == "authority" {
                    "provision".to_string()
                } else {
                    request.target_type.clone()
                },
                target_id: Some(
                    canonical_id
                        .clone()
                        .unwrap_or_else(|| request.target_id.clone()),
                ),
                pinpoint: trimmed_optional_string(request.pinpoint.as_deref()),
                status: if canonical_id.is_some() {
                    "resolved".to_string()
                } else {
                    "needs_review".to_string()
                },
                resolver_message: None,
                created_at: now.clone(),
            },
        });
    }

    if creates_exhibit {
        let document_id = trimmed_optional_string(request.document_id.as_deref()).or_else(|| {
            if matches!(request.target_type.as_str(), "document" | "case_document") {
                Some(request.target_id.clone())
            } else {
                None
            }
        });
        let exhibit_id = if request.target_type == "exhibit" {
            Some(request.target_id.clone())
        } else {
            None
        };
        let exhibit_reference_id = format!("{}:exhibit:{suffix}", request.block_id);
        operations.push(AstOperation::AddExhibitReference {
            exhibit: WorkProductExhibitReference {
                exhibit_reference_id,
                source_block_id: request.block_id.clone(),
                source_text_range: Some(range),
                label: trimmed_optional_string(request.exhibit_label.as_deref())
                    .unwrap_or_else(|| "Exhibit".to_string()),
                exhibit_id,
                document_id,
                page_range: trimmed_optional_string(request.page_range.as_deref()),
                status: "linked".to_string(),
                created_at: now,
            },
        });
    }

    for operation in &operations {
        apply_ast_operation(&mut product.document_ast, operation)?;
    }
    Ok(operations)
}

fn validate_text_range_request(
    block: &WorkProductBlock,
    request: &WorkProductTextRangeLinkRequest,
) -> ApiResult<String> {
    let selected_quote =
        text_for_char_range(&block.text, request.start_offset, request.end_offset)?;
    validate_text_range_quote(&selected_quote, Some(&request.quote))?;
    Ok(selected_quote)
}

fn validate_optional_text_range(text: &str, range: Option<&TextRange>) -> ApiResult<()> {
    if let Some(range) = range {
        let selected_quote = text_for_char_range(text, range.start_offset, range.end_offset)?;
        validate_text_range_quote(&selected_quote, range.quote.as_deref())?;
    }
    Ok(())
}

fn validate_text_range_quote(selected_quote: &str, quote: Option<&str>) -> ApiResult<()> {
    let Some(quote) = quote else {
        return Ok(());
    };
    if quote.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Text range quote is empty.".to_string(),
        ));
    }
    if selected_quote != quote && selected_quote.trim() != quote.trim() {
        return Err(ApiError::BadRequest(
            "Text range quote does not match the source block.".to_string(),
        ));
    }
    Ok(())
}

fn text_for_char_range(text: &str, start_offset: u64, end_offset: u64) -> ApiResult<String> {
    if start_offset >= end_offset {
        return Err(ApiError::BadRequest(
            "Text range must have a positive length.".to_string(),
        ));
    }
    let mut selected = String::new();
    let mut char_index = 0_u64;
    for ch in text.chars() {
        if char_index >= start_offset && char_index < end_offset {
            selected.push(ch);
        }
        char_index += 1;
        if char_index >= end_offset {
            break;
        }
    }
    if char_index < end_offset {
        return Err(ApiError::BadRequest(
            "Text range extends past the source block.".to_string(),
        ));
    }
    Ok(selected)
}

fn text_range_request_creates_citation(request: &WorkProductTextRangeLinkRequest) -> bool {
    has_non_empty_value(request.citation.as_deref())
        || matches!(
            request.target_type.as_str(),
            "authority" | "legal_authority" | "provision" | "legal_text"
        )
}

fn text_range_request_creates_exhibit(request: &WorkProductTextRangeLinkRequest) -> bool {
    has_non_empty_value(request.exhibit_label.as_deref())
        || has_non_empty_value(request.document_id.as_deref())
        || matches!(
            request.target_type.as_str(),
            "document" | "case_document" | "exhibit"
        )
}

fn trimmed_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(ToOwned::to_owned)
}

fn has_non_empty_value(value: Option<&str>) -> bool {
    matches!(value, Some(item) if !item.trim().is_empty())
}

fn normalize_document_annotation_type(value: &str) -> ApiResult<String> {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "highlight" | "note" | "redaction" | "exhibit_label" | "fact_link" | "citation"
        | "issue" => Ok(normalized),
        _ => Err(ApiError::BadRequest(
            "Unsupported document annotation type.".to_string(),
        )),
    }
}

fn default_annotation_label(annotation_type: &str) -> &'static str {
    match annotation_type {
        "highlight" => "Highlight",
        "redaction" => "Redaction",
        "exhibit_label" => "Exhibit label",
        "fact_link" => "Fact link",
        "citation" => "Citation",
        "issue" => "Issue",
        _ => "Note",
    }
}

fn validate_document_annotation_range(request: &UpsertDocumentAnnotationRequest) -> ApiResult<()> {
    if let Some(range) = &request.page_range {
        if range.page == 0 {
            return Err(ApiError::BadRequest(
                "Annotation page numbers are one-based.".to_string(),
            ));
        }
        for (name, value) in [
            ("x", range.x),
            ("y", range.y),
            ("width", range.width),
            ("height", range.height),
        ] {
            if value.is_some_and(|value| value.is_sign_negative()) {
                return Err(ApiError::BadRequest(format!(
                    "Annotation page range {name} cannot be negative."
                )));
            }
        }
    }
    if let Some(range) = &request.text_range {
        if let (Some(start), Some(end)) = (range.char_start, range.char_end) {
            if start >= end {
                return Err(ApiError::BadRequest(
                    "Annotation text char range must have a positive length.".to_string(),
                ));
            }
        }
        if let (Some(start), Some(end)) = (range.byte_start, range.byte_end) {
            if start >= end {
                return Err(ApiError::BadRequest(
                    "Annotation text byte range must have a positive length.".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn text_to_markdown_paragraphs(text: &str) -> String {
    text.split("\n\n")
        .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn sync_work_product_anchor_projection(product: &mut WorkProduct, anchor: &WorkProductAnchor) {
    let relation_projects = support_relation_projects_to_block_ids(&anchor.relation);
    let has_projected_anchor =
        relation_projects || has_projected_anchor_for_target(&product.anchors, anchor);
    for block in &mut product.blocks {
        if !support_anchor_targets_block(anchor, block) {
            continue;
        }
        match anchor.target_type.as_str() {
            "fact" => {
                if relation_projects {
                    push_unique(&mut block.fact_ids, anchor.target_id.clone());
                } else if !has_projected_anchor {
                    block
                        .fact_ids
                        .retain(|fact_id| fact_id != &anchor.target_id);
                }
            }
            "evidence" | "document" | "source_span" => {
                if relation_projects {
                    push_unique(&mut block.evidence_ids, anchor.target_id.clone());
                } else if !has_projected_anchor {
                    block
                        .evidence_ids
                        .retain(|evidence_id| evidence_id != &anchor.target_id);
                }
            }
            "authority" | "legal_authority" | "provision" | "legal_text" => {
                let authority = authority_ref_from_anchor(anchor);
                let mut updated = false;
                for existing in &mut block.authorities {
                    if same_authority(existing, &authority) {
                        *existing = authority.clone();
                        updated = true;
                    }
                }
                if !updated {
                    block.authorities.push(authority);
                }
            }
            _ => {}
        }
    }
}

fn remove_anchor_from_block_projection(
    block: &mut WorkProductBlock,
    anchor: &WorkProductAnchor,
    remaining_anchors: &[WorkProductAnchor],
) {
    match anchor.target_type.as_str() {
        "fact" => {
            if !has_projected_anchor_for_target(remaining_anchors, anchor) {
                block
                    .fact_ids
                    .retain(|fact_id| fact_id != &anchor.target_id);
            }
        }
        "evidence" | "document" | "source_span" => {
            if !has_projected_anchor_for_target(remaining_anchors, anchor) {
                block
                    .evidence_ids
                    .retain(|evidence_id| evidence_id != &anchor.target_id);
            }
        }
        "authority" | "legal_authority" | "provision" | "legal_text" => {
            if !has_authority_anchor_for_target(remaining_anchors, anchor) {
                remove_authority(&mut block.authorities, &authority_ref_from_anchor(anchor));
            }
        }
        _ => {}
    }
}

fn support_anchor_targets_block(anchor: &WorkProductAnchor, block: &WorkProductBlock) -> bool {
    anchor.block_id == block.block_id || anchor.block_id == block.id
}

fn has_projected_anchor_for_target(
    anchors: &[WorkProductAnchor],
    target: &WorkProductAnchor,
) -> bool {
    anchors.iter().any(|anchor| {
        support_anchor_targets_same_projection(anchor, target)
            && support_relation_projects_to_block_ids(&anchor.relation)
    })
}

fn has_authority_anchor_for_target(
    anchors: &[WorkProductAnchor],
    target: &WorkProductAnchor,
) -> bool {
    anchors
        .iter()
        .any(|anchor| support_anchor_targets_same_projection(anchor, target))
}

fn support_anchor_targets_same_projection(
    left: &WorkProductAnchor,
    right: &WorkProductAnchor,
) -> bool {
    left.block_id == right.block_id
        && left.target_type == right.target_type
        && left.target_id == right.target_id
}

fn support_relation_projects_to_block_ids(relation: &str) -> bool {
    matches!(
        relation,
        "supports" | "partially_supports" | "authenticates"
    )
}

fn authority_ref_from_anchor(anchor: &WorkProductAnchor) -> AuthorityRef {
    AuthorityRef {
        citation: anchor
            .citation
            .clone()
            .unwrap_or_else(|| anchor.target_id.clone()),
        canonical_id: anchor
            .canonical_id
            .clone()
            .unwrap_or_else(|| anchor.target_id.clone()),
        reason: Some(anchor.relation.clone()),
        pinpoint: anchor.pinpoint.clone(),
    }
}

fn legal_impact_for_support_anchor(anchor: &WorkProductAnchor) -> LegalImpactSummary {
    let mut impact = LegalImpactSummary::default();
    match anchor.target_type.as_str() {
        "fact" => impact.affected_facts.push(anchor.target_id.clone()),
        "evidence" | "document" | "source_span" => {
            impact.affected_evidence.push(anchor.target_id.clone())
        }
        "authority" | "legal_authority" | "provision" | "legal_text" => {
            impact.affected_authorities.push(
                anchor
                    .canonical_id
                    .clone()
                    .unwrap_or_else(|| anchor.target_id.clone()),
            )
        }
        _ => {}
    }
    impact
}

fn legal_impact_for_ast_operations(operations: &[AstOperation]) -> LegalImpactSummary {
    let mut impact = LegalImpactSummary::default();
    for operation in operations {
        match operation {
            AstOperation::AddLink { link } => match link.target_type.as_str() {
                "fact" => push_unique(&mut impact.affected_facts, link.target_id.clone()),
                "evidence" | "document" | "source_span" | "exhibit" => {
                    push_unique(&mut impact.affected_evidence, link.target_id.clone())
                }
                "authority" | "legal_authority" | "provision" | "legal_text" => {
                    push_unique(&mut impact.affected_authorities, link.target_id.clone())
                }
                _ => {}
            },
            AstOperation::AddCitation { citation } => {
                if let Some(target_id) = citation.target_id.clone() {
                    push_unique(&mut impact.affected_authorities, target_id);
                }
            }
            AstOperation::AddExhibitReference { exhibit } => {
                if let Some(exhibit_id) = exhibit.exhibit_id.clone() {
                    push_unique(&mut impact.affected_exhibits, exhibit_id);
                }
                if let Some(document_id) = exhibit.document_id.clone() {
                    push_unique(&mut impact.affected_exhibits, document_id);
                }
            }
            _ => {}
        }
    }
    impact
}

fn ast_operation_target_id(operation: &AstOperation) -> String {
    match operation {
        AstOperation::AddLink { link } => link.link_id.clone(),
        AstOperation::AddCitation { citation } => citation.citation_use_id.clone(),
        AstOperation::AddExhibitReference { exhibit } => exhibit.exhibit_reference_id.clone(),
        AstOperation::UpdateBlock { block_id, .. }
        | AstOperation::DeleteBlock { block_id, .. }
        | AstOperation::MoveBlock { block_id, .. }
        | AstOperation::SplitBlock { block_id, .. } => block_id.clone(),
        AstOperation::RemoveLink { link_id } => link_id.clone(),
        AstOperation::ResolveCitation {
            citation_use_id, ..
        }
        | AstOperation::RemoveCitation { citation_use_id } => citation_use_id.clone(),
        AstOperation::ResolveExhibitReference {
            exhibit_reference_id,
            ..
        } => exhibit_reference_id.clone(),
        AstOperation::AddRuleFinding { finding } => finding.finding_id.clone(),
        AstOperation::ResolveRuleFinding { finding_id, .. } => finding_id.clone(),
        AstOperation::InsertBlock { block, .. } => block.block_id.clone(),
        AstOperation::MergeBlocks { first_block_id, .. } => first_block_id.clone(),
        AstOperation::RenumberParagraphs => "paragraphs".to_string(),
        AstOperation::ApplyTemplate { template_id } => template_id.clone(),
    }
}

fn push_authority(values: &mut Vec<AuthorityRef>, value: AuthorityRef) {
    if !values
        .iter()
        .any(|existing| same_authority(existing, &value))
    {
        values.push(value);
    }
}

fn remove_authority(values: &mut Vec<AuthorityRef>, value: &AuthorityRef) {
    values.retain(|existing| !same_authority(existing, value));
}

fn same_authority(left: &AuthorityRef, right: &AuthorityRef) -> bool {
    if !left.canonical_id.is_empty() && !right.canonical_id.is_empty() {
        left.canonical_id == right.canonical_id
    } else {
        left.citation == right.citation
    }
}

fn count_words(paragraphs: &[DraftParagraph], sections: &[DraftSection]) -> u64 {
    paragraphs
        .iter()
        .map(|paragraph| paragraph.text.split_whitespace().count() as u64)
        .sum::<u64>()
        + sections
            .iter()
            .map(|section| section.body.split_whitespace().count() as u64)
            .sum::<u64>()
}

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::ast_diff::diff_work_product_layers;
    use super::ast_patch::apply_ast_operation;
    use super::ast_validation::validate_work_product_document;
    use super::markdown_adapter::markdown_to_work_product_ast;
    use super::rule_engine::work_product_finding;
    use super::{
        ASSEMBLYAI_CAPTION_CHARS_PER_CAPTION, ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL,
        ASSEMBLYAI_PROMPT_MAX_WORDS, ASSEMBLYAI_REDACTED_AUDIO_QUALITY,
        ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL, ASSEMBLYAI_TRANSCRIPT_LIST_DEFAULT_LIMIT,
        ASSEMBLYAI_WORD_SEARCH_MAX_TERMS, AssemblyAiParagraph, AssemblyAiParagraphsResponse,
        AssemblyAiRedactedAudioResponse, AssemblyAiSentence, AssemblyAiSentencesResponse,
        AssemblyAiSubtitleFormat, AssemblyAiTranscriptResponse, AssemblyAiUtterance,
        AssemblyAiWord, AssemblyAiWordSearchResponse, CASE_INDEX_VERSION, CHUNKER_VERSION,
        CITATION_RESOLVER_VERSION, PARSER_REGISTRY_VERSION, SourceContext,
        apply_work_product_support_removal, apply_work_product_support_update,
        apply_work_product_text_range_link, assemblyai_default_word_search_terms,
        assemblyai_effective_prompt, assemblyai_http_error, assemblyai_keyterms_word_count,
        assemblyai_prompt_preset_text, assemblyai_raw_words, assemblyai_speech_models,
        assemblyai_transcript_create_request, assemblyai_transcript_delete_response,
        assemblyai_transcript_error_message, assemblyai_transcript_list_query_pairs,
        canonical_id_for_citation, chunk_text, citation_uses_for_text, date_candidates_in_text,
        default_formatting_profile, docx_package_manifest, docx_with_replaced_document_xml,
        failed_ingestion_run, generate_opaque_id, looks_like_complaint,
        normalize_assemblyai_prompt_preset, normalize_assemblyai_remove_audio_tags,
        normalize_assemblyai_transcript_id, normalize_assemblyai_transcript_list_query,
        normalize_compare_layers, object_blob_id_for_hash, oregon_civil_complaint_rule_pack,
        parse_complaint_structure, parse_document_bytes, propose_facts, prosemirror_doc_for_text,
        provider_subtitle_or_local, rebuild_work_product_ast_from_projection,
        redact_transcript_text, refresh_work_product_state, restore_work_product_scope,
        safe_work_product_download_filename, sanitize_assemblyai_keyterms,
        sanitize_assemblyai_prompt, sanitize_assemblyai_word_search_terms, sanitize_filename,
        sanitized_external_error, sha256_hex, should_inline_payload,
        should_use_assemblyai_subtitles, slug, snapshot_entity_state_key, snapshot_full_state_key,
        snapshot_manifest_for_product, snapshot_manifest_hash_for_states, snapshot_manifest_key,
        summarize_version_snapshot_for_list, summarize_work_product_for_list,
        timeline_suggestions_from_facts, timeline_suggestions_from_text,
        transcript_segments_from_provider, transcript_segments_to_srt, transcript_segments_to_vtt,
        validate_assemblyai_transcription_request, validate_ast_patch_concurrency,
        version_change_state_summary, work_product_block_graph_payload, work_product_export_key,
        work_product_hashes, work_product_profile,
    };
    use crate::error::ApiError;
    use crate::models::casebuilder::{
        AssemblyAiSpeakerOptions, AssemblyAiTranscriptDeleteResponse,
        AssemblyAiTranscriptListQuery, AssemblyAiTranscriptListResponse, AstOperation, AstPatch,
        CaseDocument, CreateTranscriptionRequest, IngestionRun, NullableStringPatch,
        PatchWorkProductSupportRequest, TextRange, TranscriptionJob, VersionChangeSummary,
        VersionSnapshot, WorkProduct, WorkProductAction, WorkProductAnchor, WorkProductArtifact,
        WorkProductBlock, WorkProductCitationUse, WorkProductDocument, WorkProductDownloadResponse,
        WorkProductExhibitReference, WorkProductFinding, WorkProductLink,
        WorkProductTextRangeLinkRequest,
    };
    use crate::services::object_store::build_document_object_key;
    use std::collections::BTreeMap;

    #[test]
    fn sanitizes_file_names_to_local_paths() {
        assert_eq!(
            sanitize_filename("../secret motion.txt"),
            "secret_motion.txt"
        );
        assert_eq!(sanitize_filename("Lease 4B.pdf"), "Lease_4B.pdf");
    }

    #[test]
    fn hashes_uploaded_content_with_sha256_prefix() {
        let hash = sha256_hex(b"case text");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn work_product_hashes_are_stable_and_layered() {
        let base = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let same = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let text_edit =
            test_work_product("Plaintiff timely paid rent.", Vec::new(), Vec::new(), None);
        let support_edit = test_work_product(
            "Plaintiff paid rent.",
            vec!["fact:rent".to_string()],
            vec!["evidence:receipt".to_string()],
            None,
        );
        let qc_edit = test_work_product(
            "Plaintiff paid rent.",
            Vec::new(),
            Vec::new(),
            Some("Unsupported allegation"),
        );
        let mut format_edit = base.clone();
        format_edit.formatting_profile.double_spaced =
            !format_edit.formatting_profile.double_spaced;

        let base_hashes = work_product_hashes(&base).unwrap();
        let same_hashes = work_product_hashes(&same).unwrap();
        assert_eq!(base_hashes.document_hash, same_hashes.document_hash);
        assert_eq!(
            base_hashes.support_graph_hash,
            same_hashes.support_graph_hash
        );
        assert_eq!(base_hashes.qc_state_hash, same_hashes.qc_state_hash);
        assert_eq!(base_hashes.formatting_hash, same_hashes.formatting_hash);

        assert_ne!(
            base_hashes.document_hash,
            work_product_hashes(&text_edit).unwrap().document_hash
        );
        assert_ne!(
            base_hashes.support_graph_hash,
            work_product_hashes(&support_edit)
                .unwrap()
                .support_graph_hash
        );
        assert_ne!(
            base_hashes.qc_state_hash,
            work_product_hashes(&qc_edit).unwrap().qc_state_hash
        );
        assert_ne!(
            base_hashes.formatting_hash,
            work_product_hashes(&format_edit).unwrap().formatting_hash
        );
    }

    #[test]
    fn work_product_ast_validation_rejects_duplicate_block_ids() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let duplicate = product.document_ast.blocks[0].clone();
        product.document_ast.blocks.push(duplicate);

        let validation = validate_work_product_document(&product);

        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|issue| issue.code == "duplicate_block_id")
        );
    }

    #[test]
    fn ast_patch_updates_block_text_in_place() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::UpdateBlock {
                block_id: block_id.clone(),
                before: None,
                after: serde_json::json!({
                    "text": "Plaintiff timely paid rent.",
                    "title": "Paragraph 1"
                }),
            },
        )
        .expect("patch applies");

        assert_eq!(product.document_ast.blocks[0].block_id, block_id);
        assert_eq!(
            product.document_ast.blocks[0].text,
            "Plaintiff timely paid rent."
        );
    }

    #[test]
    fn ast_rule_finding_patch_survives_projection_refresh() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();
        let finding = work_product_finding(
            &product,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &block_id,
            "Paragraph needs support.",
            "Factual allegations should point to support.",
            "Link support.",
            "2026-01-01T00:00:00Z",
        );
        let finding_id = finding.finding_id.clone();

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddRuleFinding { finding },
        )
        .expect("finding patch applies");
        refresh_work_product_state(&mut product);

        assert!(
            product
                .document_ast
                .rule_findings
                .iter()
                .any(|finding| finding.finding_id == finding_id)
        );
        assert!(
            product
                .findings
                .iter()
                .any(|finding| finding.finding_id == finding_id)
        );
    }

    #[test]
    fn markdown_conversion_creates_ast_blocks() {
        let product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);

        let (document, warnings) = markdown_to_work_product_ast(
            &product,
            "## COUNT I - Breach of Contract\n\n1. Plaintiff paid rent.\n\n2. Defendant refused repairs.",
        );

        assert!(warnings.iter().any(|warning| {
            warning.contains("Markdown round trip did not include block metadata comments")
        }));
        assert_eq!(document.blocks.len(), 3);
        assert_eq!(document.blocks[0].block_type, "count");
        assert_eq!(document.blocks[1].block_type, "numbered_paragraph");
        assert_eq!(document.blocks[1].paragraph_number, Some(1));
    }

    #[test]
    fn support_relation_update_rebuilds_ast_links_and_projection() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        add_test_support_anchor(&mut product, "fact:rent", "fact", "supports");

        let updated = apply_work_product_support_update(
            &mut product,
            "work-product:test:block:1:anchor:1",
            PatchWorkProductSupportRequest {
                relation: Some("contradicts".to_string()),
                status: None,
                citation: NullableStringPatch::Unset,
                canonical_id: NullableStringPatch::Unset,
                pinpoint: NullableStringPatch::Unset,
                quote: NullableStringPatch::Unset,
            },
        )
        .expect("support relation updates");

        assert_eq!(updated.relation, "contradicts");
        assert!(product.blocks[0].fact_ids.is_empty());
        assert!(product.document_ast.links.iter().any(|link| {
            link.target_type == "fact"
                && link.target_id == "fact:rent"
                && link.relation == "contradicts"
        }));
    }

    #[test]
    fn support_removal_rebuilds_ast_links_and_projection() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        add_test_support_anchor(&mut product, "fact:rent", "fact", "supports");

        let removed =
            apply_work_product_support_removal(&mut product, "work-product:test:block:1:anchor:1")
                .expect("support link removes");

        assert_eq!(removed.target_id, "fact:rent");
        assert!(product.anchors.is_empty());
        assert!(product.marks.is_empty());
        assert!(product.blocks[0].fact_ids.is_empty());
        assert!(
            !product
                .document_ast
                .links
                .iter()
                .any(|link| link.target_id == "fact:rent")
        );
    }

    #[test]
    fn text_range_link_adds_support_citation_and_exhibit_records() {
        let mut product = test_work_product(
            "Plaintiff paid rent with receipt A.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let block_id = product.document_ast.blocks[0].block_id.clone();

        apply_work_product_text_range_link(
            &mut product,
            WorkProductTextRangeLinkRequest {
                block_id: block_id.clone(),
                start_offset: 10,
                end_offset: 19,
                quote: "paid rent".to_string(),
                target_type: "fact".to_string(),
                target_id: "fact:rent".to_string(),
                relation: Some("supports".to_string()),
                citation: None,
                canonical_id: None,
                pinpoint: None,
                exhibit_label: None,
                document_id: None,
                page_range: None,
            },
        )
        .expect("range support link applies");

        apply_work_product_text_range_link(
            &mut product,
            WorkProductTextRangeLinkRequest {
                block_id: block_id.clone(),
                start_offset: 20,
                end_offset: 24,
                quote: "with".to_string(),
                target_type: "authority".to_string(),
                target_id: "ORS 90.320".to_string(),
                relation: Some("cites".to_string()),
                citation: Some("ORS 90.320".to_string()),
                canonical_id: Some("ors:90.320".to_string()),
                pinpoint: Some("(1)".to_string()),
                exhibit_label: None,
                document_id: None,
                page_range: None,
            },
        )
        .expect("range citation applies");

        apply_work_product_text_range_link(
            &mut product,
            WorkProductTextRangeLinkRequest {
                block_id: block_id.clone(),
                start_offset: 25,
                end_offset: 34,
                quote: "receipt A".to_string(),
                target_type: "document".to_string(),
                target_id: "document:receipt".to_string(),
                relation: Some("authenticates".to_string()),
                citation: None,
                canonical_id: None,
                pinpoint: None,
                exhibit_label: Some("Exhibit A".to_string()),
                document_id: Some("document:receipt".to_string()),
                page_range: Some("1".to_string()),
            },
        )
        .expect("range exhibit applies");

        assert!(product.document_ast.links.iter().any(|link| {
            link.target_type == "fact"
                && link.target_id == "fact:rent"
                && link
                    .source_text_range
                    .as_ref()
                    .and_then(|range| range.quote.as_deref())
                    == Some("paid rent")
        }));
        assert!(product.document_ast.citations.iter().any(|citation| {
            citation.raw_text == "ORS 90.320"
                && citation.target_id.as_deref() == Some("ors:90.320")
                && citation
                    .source_text_range
                    .as_ref()
                    .and_then(|range| range.quote.as_deref())
                    == Some("with")
        }));
        assert!(product.document_ast.exhibits.iter().any(|exhibit| {
            exhibit.label == "Exhibit A"
                && exhibit.document_id.as_deref() == Some("document:receipt")
                && exhibit
                    .source_text_range
                    .as_ref()
                    .and_then(|range| range.quote.as_deref())
                    == Some("receipt A")
        }));
        let ast_block = &product.document_ast.blocks[0];
        assert_eq!(ast_block.links.len(), 3);
        assert_eq!(ast_block.citations.len(), 1);
        assert_eq!(ast_block.exhibits.len(), 1);
    }

    #[test]
    fn text_range_link_rejects_quote_that_does_not_match_source() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();

        let error = apply_work_product_text_range_link(
            &mut product,
            WorkProductTextRangeLinkRequest {
                block_id,
                start_offset: 10,
                end_offset: 19,
                quote: "wrong quote".to_string(),
                target_type: "fact".to_string(),
                target_id: "fact:rent".to_string(),
                relation: Some("supports".to_string()),
                citation: None,
                canonical_id: None,
                pinpoint: None,
                exhibit_label: None,
                document_id: None,
                page_range: None,
            },
        )
        .expect_err("mismatched selected text should fail");

        match error {
            ApiError::BadRequest(message) => {
                assert!(message.contains("does not match"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
    }

    #[test]
    fn text_range_link_uses_character_offsets_for_unicode_text() {
        let mut product = test_work_product("Lead 🧾 receipt A.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();

        apply_work_product_text_range_link(
            &mut product,
            WorkProductTextRangeLinkRequest {
                block_id,
                start_offset: 7,
                end_offset: 14,
                quote: "receipt".to_string(),
                target_type: "fact".to_string(),
                target_id: "fact:receipt".to_string(),
                relation: Some("supports".to_string()),
                citation: None,
                canonical_id: None,
                pinpoint: None,
                exhibit_label: None,
                document_id: None,
                page_range: None,
            },
        )
        .expect("unicode-prefixed range should apply");

        let range = product.document_ast.links[0]
            .source_text_range
            .as_ref()
            .expect("range is stored");
        assert_eq!(range.start_offset, 7);
        assert_eq!(range.end_offset, 14);
        assert_eq!(range.quote.as_deref(), Some("receipt"));
    }

    #[test]
    fn ast_patch_validates_add_support_text_ranges() {
        let product = test_work_product(
            "Plaintiff paid rent with receipt A.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let block_id = product.document_ast.blocks[0].block_id.clone();
        let mut document = product.document_ast.clone();

        apply_ast_operation(
            &mut document,
            &AstOperation::AddLink {
                link: test_link(
                    "link:valid",
                    &block_id,
                    Some(text_range(10, 19, Some("paid rent"))),
                ),
            },
        )
        .expect("valid link range applies");
        apply_ast_operation(
            &mut document,
            &AstOperation::AddCitation {
                citation: test_citation(
                    "citation:valid",
                    &block_id,
                    Some(text_range(20, 24, Some("with"))),
                ),
            },
        )
        .expect("valid citation range applies");
        apply_ast_operation(
            &mut document,
            &AstOperation::AddExhibitReference {
                exhibit: test_exhibit(
                    "exhibit:valid",
                    &block_id,
                    Some(text_range(25, 34, Some("receipt A"))),
                ),
            },
        )
        .expect("valid exhibit range applies");

        let bad_link = apply_ast_operation(
            &mut document,
            &AstOperation::AddLink {
                link: test_link(
                    "link:bad",
                    &block_id,
                    Some(text_range(10, 19, Some("wrong quote"))),
                ),
            },
        )
        .expect_err("mismatched link quote should fail");
        let bad_citation = apply_ast_operation(
            &mut document,
            &AstOperation::AddCitation {
                citation: test_citation(
                    "citation:bad",
                    &block_id,
                    Some(text_range(20, 99, Some("with"))),
                ),
            },
        )
        .expect_err("out-of-bounds citation range should fail");
        let bad_exhibit = apply_ast_operation(
            &mut document,
            &AstOperation::AddExhibitReference {
                exhibit: test_exhibit(
                    "exhibit:bad",
                    &block_id,
                    Some(text_range(25, 25, Some("receipt A"))),
                ),
            },
        )
        .expect_err("empty exhibit range should fail");

        assert!(matches!(bad_link, ApiError::BadRequest(_)));
        assert!(matches!(bad_citation, ApiError::BadRequest(_)));
        assert!(matches!(bad_exhibit, ApiError::BadRequest(_)));
    }

    #[test]
    fn split_block_rehomes_text_range_refs_and_keeps_whole_block_refs() {
        let mut product = test_work_product("Alpha Beta Gamma", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();
        let new_block_id = "work-product:test:block:2".to_string();
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: test_link(
                    "link:before",
                    &block_id,
                    Some(text_range(0, 5, Some("Alpha"))),
                ),
            },
        )
        .expect("before link applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: test_link(
                    "link:after",
                    &block_id,
                    Some(text_range(11, 16, Some("Gamma"))),
                ),
            },
        )
        .expect("after link applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: test_link("link:whole", &block_id, None),
            },
        )
        .expect("whole-block link applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddCitation {
                citation: test_citation(
                    "citation:after",
                    &block_id,
                    Some(text_range(11, 16, Some("Gamma"))),
                ),
            },
        )
        .expect("after citation applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddExhibitReference {
                exhibit: test_exhibit(
                    "exhibit:after",
                    &block_id,
                    Some(text_range(11, 16, Some("Gamma"))),
                ),
            },
        )
        .expect("after exhibit applies");
        let finding = work_product_finding(
            &product,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &block_id,
            "Paragraph needs support.",
            "Factual allegations should point to support.",
            "Link support.",
            "2026-01-01T00:00:00Z",
        );
        let finding_id = finding.finding_id.clone();
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddRuleFinding { finding },
        )
        .expect("finding applies");

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::SplitBlock {
                block_id: block_id.clone(),
                offset: 11,
                new_block_id: new_block_id.clone(),
            },
        )
        .expect("split applies");

        let original = product
            .document_ast
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .expect("original block remains");
        let split = product
            .document_ast
            .blocks
            .iter()
            .find(|block| block.block_id == new_block_id)
            .expect("new block exists");
        assert_eq!(original.text, "Alpha Beta ");
        assert_eq!(split.text, "Gamma");
        assert!(original.links.contains(&"link:before".to_string()));
        assert!(original.links.contains(&"link:whole".to_string()));
        assert!(!original.links.contains(&"link:after".to_string()));
        assert!(original.rule_finding_ids.contains(&finding_id));
        assert!(split.links.contains(&"link:after".to_string()));
        assert!(split.citations.contains(&"citation:after".to_string()));
        assert!(split.exhibits.contains(&"exhibit:after".to_string()));

        let moved_link = product
            .document_ast
            .links
            .iter()
            .find(|link| link.link_id == "link:after")
            .expect("after link exists");
        assert_eq!(moved_link.source_block_id, new_block_id);
        let moved_range = moved_link
            .source_text_range
            .as_ref()
            .expect("after link keeps range");
        assert_eq!(moved_range.start_offset, 0);
        assert_eq!(moved_range.end_offset, 5);
        assert_eq!(moved_range.quote.as_deref(), Some("Gamma"));
        assert!(validate_work_product_document(&product).valid);
    }

    #[test]
    fn split_block_rejects_straddling_text_range_refs() {
        let mut product = test_work_product("Alpha Beta Gamma", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: test_link(
                    "link:straddle",
                    &block_id,
                    Some(text_range(6, 16, Some("Beta Gamma"))),
                ),
            },
        )
        .expect("straddling link can be stored before split");

        let error = apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::SplitBlock {
                block_id,
                offset: 11,
                new_block_id: "work-product:test:block:2".to_string(),
            },
        )
        .expect_err("split should reject straddling range");

        match error {
            ApiError::BadRequest(message) => {
                assert!(message.contains("divide an existing text-range reference"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
    }

    #[test]
    fn merge_blocks_rehomes_support_records_and_shifts_ranges() {
        let mut product = test_work_product("Alpha", Vec::new(), Vec::new(), None);
        let first_block_id = product.document_ast.blocks[0].block_id.clone();
        let second_block_id = "work-product:test:block:2".to_string();
        product
            .document_ast
            .blocks
            .push(test_block(&product, &second_block_id, "Beta Gamma", 2));
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: test_link(
                    "link:second",
                    &second_block_id,
                    Some(text_range(5, 10, Some("Gamma"))),
                ),
            },
        )
        .expect("second link applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddCitation {
                citation: test_citation(
                    "citation:second",
                    &second_block_id,
                    Some(text_range(5, 10, Some("Gamma"))),
                ),
            },
        )
        .expect("second citation applies");
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddExhibitReference {
                exhibit: test_exhibit(
                    "exhibit:second",
                    &second_block_id,
                    Some(text_range(5, 10, Some("Gamma"))),
                ),
            },
        )
        .expect("second exhibit applies");
        let finding = work_product_finding(
            &product,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &second_block_id,
            "Paragraph needs support.",
            "Factual allegations should point to support.",
            "Link support.",
            "2026-01-01T00:00:00Z",
        );
        let finding_id = finding.finding_id.clone();
        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddRuleFinding { finding },
        )
        .expect("second finding applies");

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::MergeBlocks {
                first_block_id: first_block_id.clone(),
                second_block_id: second_block_id.clone(),
            },
        )
        .expect("merge applies");

        assert_eq!(product.document_ast.blocks.len(), 1);
        assert_eq!(product.document_ast.blocks[0].text, "Alpha\n\nBeta Gamma");
        assert!(
            product.document_ast.blocks[0]
                .links
                .contains(&"link:second".to_string())
        );
        assert!(
            product.document_ast.blocks[0]
                .citations
                .contains(&"citation:second".to_string())
        );
        assert!(
            product.document_ast.blocks[0]
                .exhibits
                .contains(&"exhibit:second".to_string())
        );
        assert!(
            product.document_ast.blocks[0]
                .rule_finding_ids
                .contains(&finding_id)
        );

        let shifted_link = product
            .document_ast
            .links
            .iter()
            .find(|link| link.link_id == "link:second")
            .expect("link exists");
        assert_eq!(shifted_link.source_block_id, first_block_id);
        let shifted_range = shifted_link
            .source_text_range
            .as_ref()
            .expect("range exists");
        assert_eq!(shifted_range.start_offset, 12);
        assert_eq!(shifted_range.end_offset, 17);
        assert_eq!(shifted_range.quote.as_deref(), Some("Gamma"));
        assert!(
            product
                .document_ast
                .rule_findings
                .iter()
                .any(|finding| finding.finding_id == finding_id
                    && finding.target_id == product.document_ast.blocks[0].block_id)
        );
        assert!(validate_work_product_document(&product).valid);
    }

    #[test]
    fn nullable_support_patch_fields_clear_and_omitted_fields_remain() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        add_test_support_anchor(&mut product, "fact:rent", "fact", "supports");
        {
            let anchor = product
                .anchors
                .iter_mut()
                .find(|anchor| anchor.anchor_id == "work-product:test:block:1:anchor:1")
                .expect("anchor exists");
            anchor.citation = Some("Old citation".to_string());
            anchor.canonical_id = Some("old:canonical".to_string());
            anchor.pinpoint = Some("1".to_string());
            anchor.quote = Some("paid rent".to_string());
        }
        rebuild_work_product_ast_from_projection(&mut product);

        let unchanged = apply_work_product_support_update(
            &mut product,
            "work-product:test:block:1:anchor:1",
            PatchWorkProductSupportRequest {
                relation: None,
                status: Some("verified".to_string()),
                citation: NullableStringPatch::Unset,
                canonical_id: NullableStringPatch::Unset,
                pinpoint: NullableStringPatch::Unset,
                quote: NullableStringPatch::Unset,
            },
        )
        .expect("omitted fields remain unchanged");
        assert_eq!(unchanged.citation.as_deref(), Some("Old citation"));
        assert_eq!(unchanged.canonical_id.as_deref(), Some("old:canonical"));
        assert_eq!(unchanged.pinpoint.as_deref(), Some("1"));
        assert_eq!(unchanged.quote.as_deref(), Some("paid rent"));

        let cleared = apply_work_product_support_update(
            &mut product,
            "work-product:test:block:1:anchor:1",
            PatchWorkProductSupportRequest {
                relation: None,
                status: None,
                citation: NullableStringPatch::Clear,
                canonical_id: NullableStringPatch::Clear,
                pinpoint: NullableStringPatch::Clear,
                quote: NullableStringPatch::Clear,
            },
        )
        .expect("null fields clear metadata");
        assert!(cleared.citation.is_none());
        assert!(cleared.canonical_id.is_none());
        assert!(cleared.pinpoint.is_none());
        assert!(cleared.quote.is_none());
    }

    fn text_range(start_offset: u64, end_offset: u64, quote: Option<&str>) -> TextRange {
        TextRange {
            start_offset,
            end_offset,
            quote: quote.map(str::to_string),
        }
    }

    fn test_link(
        link_id: &str,
        source_block_id: &str,
        source_text_range: Option<TextRange>,
    ) -> WorkProductLink {
        WorkProductLink {
            link_id: link_id.to_string(),
            source_block_id: source_block_id.to_string(),
            source_text_range,
            target_type: "fact".to_string(),
            target_id: "fact:rent".to_string(),
            relation: "supports".to_string(),
            confidence: Some(1.0),
            created_by: "test".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_citation(
        citation_use_id: &str,
        source_block_id: &str,
        source_text_range: Option<TextRange>,
    ) -> WorkProductCitationUse {
        WorkProductCitationUse {
            citation_use_id: citation_use_id.to_string(),
            source_block_id: source_block_id.to_string(),
            source_text_range,
            raw_text: "ORS 90.320".to_string(),
            normalized_citation: Some("ORS 90.320".to_string()),
            target_type: "legal_authority".to_string(),
            target_id: Some("ors:90.320".to_string()),
            pinpoint: Some("(1)".to_string()),
            status: "resolved".to_string(),
            resolver_message: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_exhibit(
        exhibit_reference_id: &str,
        source_block_id: &str,
        source_text_range: Option<TextRange>,
    ) -> WorkProductExhibitReference {
        WorkProductExhibitReference {
            exhibit_reference_id: exhibit_reference_id.to_string(),
            source_block_id: source_block_id.to_string(),
            source_text_range,
            label: "Exhibit A".to_string(),
            exhibit_id: Some("exhibit:a".to_string()),
            document_id: Some("document:receipt".to_string()),
            page_range: Some("1".to_string()),
            status: "linked".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_block(
        product: &WorkProduct,
        block_id: &str,
        text: &str,
        ordinal: u64,
    ) -> WorkProductBlock {
        WorkProductBlock {
            id: block_id.to_string(),
            block_id: block_id.to_string(),
            matter_id: product.matter_id.clone(),
            work_product_id: product.work_product_id.clone(),
            block_type: "paragraph".to_string(),
            role: "factual_allegation".to_string(),
            title: format!("Paragraph {ordinal}"),
            text: text.to_string(),
            ordinal,
            parent_block_id: None,
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        }
    }

    fn add_test_support_anchor(
        product: &mut WorkProduct,
        target_id: &str,
        target_type: &str,
        relation: &str,
    ) {
        let block_id = product.blocks[0].block_id.clone();
        let anchor_id = format!("{block_id}:anchor:1");
        product.blocks[0].fact_ids.push(target_id.to_string());
        product.blocks[0].mark_ids.push(anchor_id.clone());
        product.anchors.push(WorkProductAnchor {
            id: anchor_id.clone(),
            anchor_id: anchor_id.clone(),
            matter_id: product.matter_id.clone(),
            work_product_id: product.work_product_id.clone(),
            block_id,
            anchor_type: target_type.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            relation: relation.to_string(),
            citation: None,
            canonical_id: None,
            pinpoint: None,
            quote: None,
            status: "needs_review".to_string(),
        });
        rebuild_work_product_ast_from_projection(product);
    }

    fn test_work_product(
        text: &str,
        fact_ids: Vec<String>,
        evidence_ids: Vec<String>,
        qc_message: Option<&str>,
    ) -> WorkProduct {
        let block_id = "work-product:test:block:1".to_string();
        let mut product = WorkProduct {
            id: "work-product:test".to_string(),
            work_product_id: "work-product:test".to_string(),
            matter_id: "matter:test".to_string(),
            title: "Test complaint".to_string(),
            product_type: "complaint".to_string(),
            status: "draft".to_string(),
            review_status: "needs_human_review".to_string(),
            setup_stage: "editor".to_string(),
            source_draft_id: None,
            source_complaint_id: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            profile: work_product_profile("complaint"),
            document_ast: WorkProductDocument::default(),
            blocks: vec![WorkProductBlock {
                id: block_id.clone(),
                block_id: block_id.clone(),
                matter_id: "matter:test".to_string(),
                work_product_id: "work-product:test".to_string(),
                block_type: "paragraph".to_string(),
                role: "factual_allegation".to_string(),
                title: "Paragraph 1".to_string(),
                text: text.to_string(),
                ordinal: 1,
                parent_block_id: None,
                fact_ids,
                evidence_ids,
                authorities: Vec::new(),
                mark_ids: Vec::new(),
                locked: false,
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(text)),
                ..WorkProductBlock::default()
            }],
            marks: Vec::new(),
            anchors: Vec::new(),
            findings: qc_message
                .map(|message| {
                    vec![WorkProductFinding {
                        id: "finding:test".to_string(),
                        finding_id: "finding:test".to_string(),
                        matter_id: "matter:test".to_string(),
                        work_product_id: "work-product:test".to_string(),
                        rule_id: "support:required".to_string(),
                        rule_pack_id: Some("test-rule-pack".to_string()),
                        source_citation: None,
                        source_url: None,
                        category: "support".to_string(),
                        severity: "warning".to_string(),
                        target_type: "paragraph".to_string(),
                        target_id: block_id,
                        message: message.to_string(),
                        explanation: message.to_string(),
                        suggested_fix: "Link support.".to_string(),
                        auto_fix_available: false,
                        primary_action: WorkProductAction {
                            action_id: "action:test".to_string(),
                            label: "Link support".to_string(),
                            action_type: "link_support".to_string(),
                            href: None,
                            target_type: "paragraph".to_string(),
                            target_id: "work-product:test:block:1".to_string(),
                        },
                        status: "open".to_string(),
                        created_at: "2026-01-01T00:00:00Z".to_string(),
                        updated_at: "2026-01-01T00:00:00Z".to_string(),
                    }]
                })
                .unwrap_or_default(),
            artifacts: Vec::new(),
            history: Vec::new(),
            ai_commands: Vec::new(),
            formatting_profile: default_formatting_profile(),
            rule_pack: oregon_civil_complaint_rule_pack(),
        };
        refresh_work_product_state(&mut product);
        product
    }

    fn test_version_snapshot(full_state_inline: Option<serde_json::Value>) -> VersionSnapshot {
        VersionSnapshot {
            id: "work-product:test:snapshot:1".to_string(),
            snapshot_id: "work-product:test:snapshot:1".to_string(),
            matter_id: "matter:test".to_string(),
            subject_type: "work_product".to_string(),
            subject_id: "work-product:test".to_string(),
            product_type: "complaint".to_string(),
            profile_id: "complaint".to_string(),
            branch_id: "work-product:test:branch:main".to_string(),
            sequence_number: 1,
            title: "Snapshot 1".to_string(),
            message: Some("Snapshot".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            created_by: "system".to_string(),
            actor_id: None,
            snapshot_type: "auto".to_string(),
            parent_snapshot_ids: Vec::new(),
            document_hash: "sha256:document".to_string(),
            support_graph_hash: "sha256:support".to_string(),
            qc_state_hash: "sha256:qc".to_string(),
            formatting_hash: "sha256:formatting".to_string(),
            manifest_hash: "sha256:manifest".to_string(),
            manifest_ref: None,
            full_state_ref: None,
            full_state_inline,
            summary: VersionChangeSummary {
                text_changes: 0,
                support_changes: 0,
                citation_changes: 0,
                authority_changes: 0,
                qc_changes: 0,
                export_changes: 0,
                ai_changes: 0,
                targets_changed: Vec::new(),
                risk_level: "low".to_string(),
                user_summary: "Snapshot".to_string(),
            },
        }
    }

    fn test_work_product_artifact(artifact_id: &str, artifact_hash: &str) -> WorkProductArtifact {
        WorkProductArtifact {
            artifact_id: artifact_id.to_string(),
            id: artifact_id.to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "work-product:test".to_string(),
            format: "html".to_string(),
            profile: "review".to_string(),
            mode: "review_needed".to_string(),
            status: "generated_review_needed".to_string(),
            download_url: "/api/v1/download".to_string(),
            page_count: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            warnings: vec!["review needed".to_string()],
            content_preview: "bounded preview".to_string(),
            snapshot_id: Some("snapshot:1".to_string()),
            artifact_hash: Some(artifact_hash.to_string()),
            render_profile_hash: Some("sha256:render".to_string()),
            qc_status_at_export: Some("needs_review".to_string()),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some("blob:sha256:artifact".to_string()),
            size_bytes: Some(42),
            mime_type: Some("text/html".to_string()),
            storage_status: Some("stored".to_string()),
        }
    }

    #[test]
    fn chunks_text_without_empty_chunks() {
        let chunks = chunk_text("doc:1", "first\nsecond");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "first\nsecond");
        assert_eq!(chunks[0].byte_start, Some(0));
        assert_eq!(
            chunks[0].char_end,
            Some("first\nsecond".chars().count() as u64)
        );
    }

    #[test]
    fn extracts_named_iso_and_numeric_dates_with_review_confidence() {
        let dates = date_candidates_in_text(
            "Tenant reported mold on April 1, 2026. Rent posted 2026-04-02. Numeric 4/3/2026 needs review.",
        );

        assert_eq!(dates[0].iso_date, "2026-04-01");
        assert_eq!(dates[1].iso_date, "2026-04-02");
        assert_eq!(dates[2].iso_date, "2026-04-03");
        assert!(
            dates[2]
                .warnings
                .contains(&"numeric_date_format_needs_review".to_string())
        );
    }

    #[test]
    fn timeline_suggestions_are_review_first_from_plain_text() {
        let suggestions = timeline_suggestions_from_text(
            "matter:test",
            "Tenant reported mold on April 1, 2026. Repairs were not completed.",
            "document",
            Some("doc:test"),
            vec!["source-span:test".to_string()],
            vec!["chunk:test".to_string()],
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            None,
            10,
        );

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].status, "suggested");
        assert_eq!(suggestions[0].date, "2026-04-01");
        assert_eq!(
            suggestions[0].source_document_id.as_deref(),
            Some("doc:test")
        );
    }

    #[test]
    fn timeline_suggestions_link_indexed_facts_without_creating_events() {
        let context = SourceContext {
            document_version_id: Some("version:doc:1".to_string()),
            object_blob_id: Some("blob:doc:1".to_string()),
            ingestion_run_id: Some("ingestion:doc:1".to_string()),
        };
        let facts = propose_facts(
            "matter:test",
            "doc:test",
            "Tenant reported mold on April 1, 2026. Repairs were not completed for two weeks.",
            &context,
        );
        let suggestions = timeline_suggestions_from_facts(
            "matter:test",
            Some("doc:test"),
            &facts,
            &[],
            "document_index",
            None,
            None,
            Some("timeline-agent-run:test"),
            Some("index-run:test"),
            10,
        );

        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].linked_fact_ids,
            vec![facts[0].fact_id.clone()]
        );
        assert_eq!(
            suggestions[0].index_run_id.as_deref(),
            Some("index-run:test")
        );
        assert!(suggestions[0].approved_event_id.is_none());
    }

    #[test]
    fn slug_has_stable_prefix_safe_shape() {
        assert_eq!(
            slug("Smith v. ABC Property Management"),
            "smith-v-abc-property-management"
        );
    }

    #[test]
    fn duplicate_hashes_share_object_blob_identity() {
        let first = object_blob_id_for_hash(
            "sha256:ABCDEFabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234",
        );
        let second = object_blob_id_for_hash(
            "abcdefabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234",
        );
        assert_eq!(first, second);
        assert!(first.starts_with("blob:sha256:"));
    }

    #[test]
    fn new_document_object_keys_do_not_include_raw_filenames() {
        let document_id = generate_opaque_id("doc");
        let key = build_document_object_key(&document_id, "../Private Tenant Notice.pdf");
        assert!(!document_id.contains("tenant"));
        assert!(!document_id.contains("notice"));
        assert!(!key.contains("Private"));
        assert!(!key.contains("Tenant"));
        assert!(key.ends_with("/original.pdf"));
    }

    #[test]
    fn ast_storage_policy_inlines_only_within_threshold() {
        assert!(should_inline_payload(64 * 1024, 64 * 1024));
        assert!(!should_inline_payload(64 * 1024 + 1, 64 * 1024));
    }

    #[test]
    fn ast_artifact_object_keys_are_hash_scoped() {
        let matter_id = "matter:Smith v. ABC Property:123";
        let work_product_id = "work-product:Private Motion:456";
        let snapshot_id = "work-product:Private Motion:456:snapshot:1";
        let key = snapshot_full_state_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        );
        assert!(key.starts_with("casebuilder/matters/"));
        assert!(!key.contains("Smith"));
        assert!(!key.contains("Private"));
        assert!(!key.contains("Motion"));
        assert!(key.ends_with(".json"));

        let manifest_key = snapshot_manifest_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(!manifest_key.contains("Smith"));
        assert!(!manifest_key.contains("Private"));
        assert!(manifest_key.ends_with(".json"));

        let state_key = snapshot_entity_state_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "document_ast",
            "sha256:0123456789abcdef",
        );
        assert!(state_key.contains("/states/document_ast/0123456789abcdef.json"));

        let export_key = work_product_export_key(
            matter_id,
            work_product_id,
            "artifact:Secret Draft:1",
            "sha256:fedcba",
            "html",
        );
        assert!(!export_key.contains("Secret"));
        assert!(export_key.ends_with("/fedcba.html"));
    }

    #[test]
    fn snapshot_manifest_hash_survives_state_object_offload() {
        let product = test_work_product(
            "Confidential allegation that belongs only in snapshot state.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let (mut manifest, mut states) = snapshot_manifest_for_product(
            "matter:test",
            "work-product:test:snapshot:1",
            &product,
            "2026-01-01T00:00:00Z",
        )
        .expect("manifest");
        let before_hash = manifest.manifest_hash.clone();

        for state in &mut states {
            state.state_inline = None;
            state.state_ref = Some(object_blob_id_for_hash(&state.entity_hash));
        }
        manifest.storage_ref = Some(object_blob_id_for_hash(&before_hash));

        assert_eq!(
            before_hash,
            snapshot_manifest_hash_for_states(&states).expect("hash")
        );
        assert!(states.iter().all(|state| {
            state
                .state_ref
                .as_deref()
                .is_some_and(|value| value.starts_with("blob:sha256:"))
        }));
    }

    #[test]
    fn version_change_payloads_store_hash_summaries_not_document_text() {
        let value = serde_json::json!({
            "document_ast": {
                "blocks": [{
                    "block_id": "block:1",
                    "text": "Secret merits analysis should not be duplicated in VersionChange."
                }]
            }
        });

        let (hash, summary) = version_change_state_summary(Some(value)).expect("summary");
        let summary_text = serde_json::to_string(&summary).expect("json");

        assert!(
            hash.as_deref()
                .is_some_and(|value| value.starts_with("sha256:"))
        );
        assert!(summary_text.contains("\"state_storage\":\"version_snapshot\""));
        assert!(summary_text.contains("\"inline_payload\":false"));
        assert!(!summary_text.contains("Secret merits analysis"));
    }

    #[test]
    fn stale_ast_patch_base_document_hash_is_rejected_without_text() {
        let patch = AstPatch {
            patch_id: "patch:test".to_string(),
            draft_id: None,
            work_product_id: Some("work-product:test".to_string()),
            base_document_hash: Some("sha256:stale".to_string()),
            base_snapshot_id: None,
            created_by: "user".to_string(),
            reason: Some("Secret factual edit".to_string()),
            operations: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let error = validate_ast_patch_concurrency(
            &patch,
            "work-product:test",
            "sha256:current",
            Some("work-product:test:snapshot:2"),
        )
        .expect_err("stale hash is rejected");

        match error {
            ApiError::Conflict(message) => {
                assert!(!message.contains("patch_id=patch:test"));
                assert!(message.contains("base_document_hash=sha256:stale"));
                assert!(message.contains("current_document_hash=sha256:current"));
                assert!(!message.contains("Secret factual edit"));
            }
            other => panic!("expected conflict, got {other:?}"),
        }
    }

    #[test]
    fn ast_patch_reference_errors_do_not_echo_legal_text_or_ids() {
        let mut product = test_work_product(
            "Secret factual edit should stay out of error messages.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let error = apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: WorkProductLink {
                    link_id: "link:1".to_string(),
                    source_block_id: "block:Secret Missing Block".to_string(),
                    source_text_range: None,
                    target_type: "fact".to_string(),
                    target_id: "fact:Secret Tenant Payment".to_string(),
                    relation: "supports".to_string(),
                    confidence: None,
                    created_by: "user".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
            },
        )
        .expect_err("missing source block is rejected");

        match error {
            ApiError::NotFound(message) => {
                assert_eq!(message, "AST link source block not found");
                assert!(!message.contains("Secret"));
                assert!(!message.contains("Tenant"));
                assert!(!message.contains("Payment"));
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }

    #[test]
    fn compare_layers_cover_legal_ast_without_copying_sensitive_payloads() {
        let base = test_work_product(
            "Confidential base allegation should not appear in layer diffs.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let mut changed = base.clone();
        let block_id = changed.document_ast.blocks[0].block_id.clone();
        changed.document_ast.links.push(WorkProductLink {
            link_id: "link:1".to_string(),
            source_block_id: block_id.clone(),
            source_text_range: None,
            target_type: "fact".to_string(),
            target_id: "fact:secret-payment-detail".to_string(),
            relation: "supports".to_string(),
            confidence: Some(0.9),
            created_by: "user".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        });
        changed.document_ast.blocks[0]
            .links
            .push("link:1".to_string());
        changed.document_ast.citations.push(WorkProductCitationUse {
            citation_use_id: "citation:1".to_string(),
            source_block_id: block_id.clone(),
            source_text_range: None,
            raw_text: "SECRET RAW CITATION TEXT".to_string(),
            normalized_citation: Some("ORS 90.320".to_string()),
            target_type: "provision".to_string(),
            target_id: Some("or:ors:90.320".to_string()),
            pinpoint: Some("SECRET PINPOINT".to_string()),
            status: "resolved".to_string(),
            resolver_message: Some("SECRET RESOLVER MESSAGE".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        });
        changed.document_ast.blocks[0]
            .citations
            .push("citation:1".to_string());
        changed
            .document_ast
            .exhibits
            .push(WorkProductExhibitReference {
                exhibit_reference_id: "exhibit:1".to_string(),
                source_block_id: block_id.clone(),
                source_text_range: None,
                label: "SECRET EXHIBIT LABEL".to_string(),
                exhibit_id: Some("evidence:secret-exhibit".to_string()),
                document_id: None,
                page_range: Some("1-2".to_string()),
                status: "resolved".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            });
        changed.document_ast.blocks[0]
            .exhibits
            .push("exhibit:1".to_string());
        let finding = work_product_finding(
            &changed,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &block_id,
            "SECRET FINDING MESSAGE",
            "SECRET FINDING EXPLANATION",
            "SECRET FINDING FIX",
            "2026-01-01T00:00:00Z",
        );
        changed.document_ast.rule_findings.push(finding.clone());
        changed.findings.push(finding);
        changed.formatting_profile.double_spaced = !changed.formatting_profile.double_spaced;
        changed.artifacts.push(WorkProductArtifact {
            artifact_id: "artifact:1".to_string(),
            id: "artifact:1".to_string(),
            matter_id: changed.matter_id.clone(),
            work_product_id: changed.work_product_id.clone(),
            format: "html".to_string(),
            profile: "review".to_string(),
            mode: "review_needed".to_string(),
            status: "generated_review_needed".to_string(),
            download_url: "/api/v1/download".to_string(),
            page_count: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            warnings: vec!["review needed".to_string()],
            content_preview: "SECRET EXPORT PREVIEW".to_string(),
            snapshot_id: Some("snapshot:1".to_string()),
            artifact_hash: Some("sha256:artifact".to_string()),
            render_profile_hash: Some("sha256:render".to_string()),
            qc_status_at_export: Some("needs_review".to_string()),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some("blob:sha256:artifact".to_string()),
            size_bytes: Some(512),
            mime_type: Some("text/html".to_string()),
            storage_status: Some("stored".to_string()),
        });

        let diffs = diff_work_product_layers(
            &base,
            &changed,
            &normalize_compare_layers(vec!["all".to_string()]),
        )
        .expect("layer diff");
        for layer in [
            "support",
            "citations",
            "exhibits",
            "rule_findings",
            "formatting",
            "exports",
        ] {
            assert!(
                diffs.iter().any(|diff| diff.layer == layer),
                "missing {layer} diff"
            );
        }
        let text = serde_json::to_string(&diffs).expect("diffs serialize");
        for forbidden in [
            "SECRET RAW CITATION TEXT",
            "SECRET EXHIBIT LABEL",
            "SECRET FINDING MESSAGE",
            "SECRET EXPORT PREVIEW",
            "Confidential base allegation",
        ] {
            assert!(!text.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn scoped_restore_block_preserves_unrelated_current_ast_edits() {
        let mut snapshot = test_work_product("Snapshot block one.", Vec::new(), Vec::new(), None);
        let mut current = snapshot.clone();
        let target_id = snapshot.document_ast.blocks[0].block_id.clone();
        let mut second_snapshot = snapshot.document_ast.blocks[0].clone();
        second_snapshot.block_id = "work-product:test:block:2".to_string();
        second_snapshot.id = second_snapshot.block_id.clone();
        second_snapshot.text = "Snapshot block two.".to_string();
        second_snapshot.ordinal = 2;
        snapshot.document_ast.blocks.push(second_snapshot.clone());
        refresh_work_product_state(&mut snapshot);

        current.document_ast.blocks[0].text = "Current block one edit.".to_string();
        let mut second_current = second_snapshot;
        second_current.text = "Current unrelated block two edit.".to_string();
        current.document_ast.blocks.push(second_current);
        refresh_work_product_state(&mut current);

        let (restored, warnings) =
            restore_work_product_scope(&current, &snapshot, "block", &[target_id.clone()])
                .expect("restore block");

        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("Targeted restore"))
        );
        assert_eq!(restored.document_ast.blocks[0].text, "Snapshot block one.");
        assert_eq!(
            restored.document_ast.blocks[1].text,
            "Current unrelated block two edit."
        );
    }

    #[test]
    fn scoped_restore_export_state_merges_without_deleting_newer_artifacts() {
        let mut snapshot = test_work_product("Snapshot.", Vec::new(), Vec::new(), None);
        let mut current = snapshot.clone();
        snapshot.artifacts.push(test_work_product_artifact(
            "artifact:snapshot",
            "sha256:snapshot",
        ));
        current.artifacts.push(test_work_product_artifact(
            "artifact:current",
            "sha256:current",
        ));

        let (restored, _warnings) =
            restore_work_product_scope(&current, &snapshot, "export_state", &[])
                .expect("restore export state");

        assert!(
            restored
                .artifacts
                .iter()
                .any(|artifact| artifact.artifact_id == "artifact:snapshot")
        );
        assert!(
            restored
                .artifacts
                .iter()
                .any(|artifact| artifact.artifact_id == "artifact:current")
        );
    }

    #[test]
    fn work_product_download_response_omits_storage_keys_and_safe_filename_omits_titles() {
        let artifact = test_work_product_artifact("artifact:opaque", "sha256:artifact");
        let response = WorkProductDownloadResponse {
            method: "GET".to_string(),
            url: "/api/v1/matters/matter/work-products/work-product/artifacts/artifact/download"
                .to_string(),
            expires_at: "2".to_string(),
            headers: BTreeMap::new(),
            filename: safe_work_product_download_filename(&artifact),
            mime_type: Some("text/html".to_string()),
            bytes: 42,
            artifact,
        };
        let text = serde_json::to_string(&response).expect("response serializes");
        assert!(!text.contains("storage_key"));
        assert!(!text.contains("casebuilder/matters/private/raw-key"));

        let sensitive_artifact = test_work_product_artifact(
            "artifact:Secret Draft About Tenant Payment",
            "sha256:artifact",
        );
        let filename = safe_work_product_download_filename(&sensitive_artifact);
        assert!(!filename.contains("Secret"));
        assert!(!filename.contains("Tenant"));
        assert!(!filename.contains("Payment"));
        assert!(filename.starts_with("work-product-export-"));
    }

    #[test]
    fn work_product_list_summary_omits_ast_payload_by_default() {
        let mut summary = test_work_product(
            "Long legal document body should not ride along in list responses.",
            Vec::new(),
            Vec::new(),
            None,
        );

        summarize_work_product_for_list(&mut summary);

        assert!(summary.blocks.is_empty());
        assert!(summary.document_ast.blocks.is_empty());
        assert!(summary.document_ast.links.is_empty());
        assert_eq!(summary.document_ast.title, "Test complaint");
    }

    #[test]
    fn large_work_product_list_summary_omits_large_ast_by_default() {
        let mut summary = test_work_product(
            "Large legal document body should not ride along in list responses.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let template = summary.document_ast.blocks[0].clone();
        for index in 2..=750 {
            let mut block = template.clone();
            block.block_id = format!("work-product:test:block:{index}");
            block.id = block.block_id.clone();
            block.ordinal = index;
            block.text = format!("Large confidential paragraph {index}.");
            summary.document_ast.blocks.push(block);
        }
        refresh_work_product_state(&mut summary);

        summarize_work_product_for_list(&mut summary);

        assert!(summary.blocks.is_empty());
        assert!(summary.document_ast.blocks.is_empty());
        assert_eq!(summary.document_ast.title, "Test complaint");
    }

    #[test]
    fn snapshot_list_summary_omits_inline_full_state() {
        let mut snapshot = test_version_snapshot(Some(serde_json::json!({
            "document_ast": {
                "blocks": [{
                    "text": "Snapshot detail text should not appear in snapshot lists."
                }]
            }
        })));
        snapshot.full_state_ref = Some("blob:sha256:abcdef".to_string());

        summarize_version_snapshot_for_list(&mut snapshot);

        assert!(snapshot.full_state_inline.is_none());
        assert_eq!(
            snapshot.full_state_ref.as_deref(),
            Some("blob:sha256:abcdef")
        );
    }

    #[test]
    fn oversized_block_graph_payload_keeps_excerpt_and_hash() {
        let product = test_work_product(&"x".repeat(80), Vec::new(), Vec::new(), None);
        let block = &product.blocks[0];
        let payload = work_product_block_graph_payload(block, 8).expect("payload");
        let value: serde_json::Value = serde_json::from_str(&payload).expect("json payload");
        assert_eq!(value["text_storage_status"], "graph_excerpt");
        assert_eq!(value["text_size_bytes"], 80);
        assert!(
            value["text_hash"]
                .as_str()
                .expect("hash")
                .starts_with("sha256:")
        );
    }

    #[test]
    fn proposed_facts_get_source_spans() {
        let context = SourceContext {
            document_version_id: Some("version:doc_opaque:original".to_string()),
            object_blob_id: Some("blob:sha256:abc".to_string()),
            ingestion_run_id: Some("ingestion:doc_opaque:primary".to_string()),
        };
        let facts = propose_facts(
            "matter:test",
            "doc:opaque",
            "The tenant paid rent on March 1, 2024, and the landlord accepted the payment without objection.",
            &context,
        );
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].source_spans.len(), 1);
        assert_eq!(
            facts[0].source_spans[0].document_version_id.as_deref(),
            Some("version:doc_opaque:original")
        );
        assert_eq!(
            facts[0].source_spans[0].quote.as_deref(),
            Some(
                "The tenant paid rent on March 1, 2024, and the landlord accepted the payment without objection."
            )
        );
    }

    #[test]
    fn failed_extraction_marks_ingestion_run_failed() {
        let run = IngestionRun {
            ingestion_run_id: "ingestion:doc:primary".to_string(),
            id: "ingestion:doc:primary".to_string(),
            matter_id: "matter:test".to_string(),
            document_id: "doc:test".to_string(),
            document_version_id: Some("version:doc:original".to_string()),
            object_blob_id: Some("blob:sha256:abc".to_string()),
            input_sha256: Some("sha256:abc".to_string()),
            status: "stored".to_string(),
            stage: "stored".to_string(),
            mode: "deterministic".to_string(),
            started_at: "1".to_string(),
            completed_at: None,
            error_code: None,
            error_message: None,
            retryable: false,
            produced_node_ids: Vec::new(),
            produced_object_keys: Vec::new(),
            parser_id: Some("casebuilder-parser-registry".to_string()),
            parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
            chunker_version: Some(CHUNKER_VERSION.to_string()),
            citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
            index_version: Some(CASE_INDEX_VERSION.to_string()),
        };
        let failed =
            failed_ingestion_run(&run, "extract_text", "no_extractable_text", "empty", false);
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.stage, "extract_text");
        assert_eq!(failed.error_code.as_deref(), Some("no_extractable_text"));
        assert!(!failed.retryable);
        assert!(failed.completed_at.is_some());
    }

    #[test]
    fn parser_registry_extracts_supported_text_and_flags_deferred_media() {
        let markdown = parse_document_bytes(
            "complaint.md",
            Some("text/markdown"),
            b"# Complaint\n1. Plaintiff cites ORS 90.100.",
        );
        assert_eq!(markdown.status, "processed");
        assert!(markdown.text.unwrap().contains("Plaintiff cites"));

        let html = parse_document_bytes(
            "notice.html",
            Some("text/html"),
            b"<main><p>Tenant paid &amp; landlord accepted.</p></main>",
        );
        assert_eq!(
            html.text.as_deref(),
            Some("Tenant paid & landlord accepted.")
        );

        let docx = stored_zip_document_xml(
	            br#"<w:document><w:body><w:p><w:r><w:t>Complaint paragraph from DOCX.</w:t></w:r></w:p></w:body></w:document>"#,
	        );
        let docx = parse_document_bytes(
            "complaint.docx",
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            &docx,
        );
        assert_eq!(docx.status, "processed");
        assert!(
            docx.text
                .unwrap()
                .contains("Complaint paragraph from DOCX.")
        );

        let pdf = parse_document_bytes(
            "embedded.pdf",
            Some("application/pdf"),
            b"%PDF-1.4\nBT (Plaintiff paid rent on March 1, 2026.) Tj ET",
        );
        assert_eq!(pdf.status, "processed");
        assert!(pdf.text.unwrap().contains("Plaintiff paid rent"));

        let image = parse_document_bytes("scan.png", Some("image/png"), b"not text");
        assert_eq!(image.status, "ocr_required");
        assert!(image.text.is_none());

        let media = parse_document_bytes("hearing.mp4", Some("video/mp4"), b"not text");
        assert_eq!(media.status, "transcription_deferred");
    }

    #[test]
    fn assemblyai_request_uses_casebuilder_defaults_without_webhook_leakage() {
        let job = TranscriptionJob {
            transcription_job_id: "transcription:doc:test".to_string(),
            id: "transcription:doc:test".to_string(),
            matter_id: "matter:test".to_string(),
            document_id: "doc:test".to_string(),
            document_version_id: Some("version:doc:test:original".to_string()),
            object_blob_id: Some("blob:sha256:test".to_string()),
            provider: "assemblyai".to_string(),
            provider_mode: "live".to_string(),
            provider_transcript_id: None,
            provider_status: None,
            status: "queued".to_string(),
            review_status: "not_started".to_string(),
            raw_artifact_version_id: None,
            normalized_artifact_version_id: None,
            redacted_artifact_version_id: None,
            redacted_audio_version_id: None,
            reviewed_document_version_id: None,
            caption_vtt_version_id: None,
            caption_srt_version_id: None,
            language_code: None,
            duration_ms: None,
            speaker_count: 0,
            segment_count: 0,
            word_count: 0,
            speakers_expected: None,
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: None,
            prompt: None,
            keyterms_prompt: Vec::new(),
            remove_audio_tags: Some(ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL.to_string()),
            redact_pii: true,
            speech_models: assemblyai_speech_models(),
            retryable: false,
            error_code: None,
            error_message: None,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            completed_at: None,
            reviewed_at: None,
        };
        let request = assemblyai_transcript_create_request(
            "assemblyai://upload",
            &CreateTranscriptionRequest {
                force: None,
                language_code: None,
                redact_pii: Some(true),
                speaker_labels: None,
                speakers_expected: None,
                speaker_options: None,
                word_search_terms: Vec::new(),
                prompt_preset: None,
                prompt: None,
                keyterms_prompt: Vec::new(),
                remove_audio_tags: None,
            },
            &job,
            Some("https://example.test/webhook".to_string()),
            Some("secret".to_string()),
        );
        let payload = serde_json::to_value(request).unwrap();
        assert_eq!(
            payload["speech_models"],
            serde_json::json!(["universal-3-pro", "universal-2"])
        );
        assert_eq!(payload["language_detection"], true);
        assert_eq!(payload["speaker_labels"], true);
        assert!(payload.get("speakers_expected").is_none());
        assert!(payload.get("speaker_options").is_none());
        assert_eq!(payload["redact_pii"], true);
        assert_eq!(payload["redact_pii_sub"], "entity_name");
        assert_eq!(payload["redact_pii_return_unredacted"], true);
        assert_eq!(payload["redact_pii_audio"], true);
        assert_eq!(
            payload["redact_pii_audio_quality"],
            ASSEMBLYAI_REDACTED_AUDIO_QUALITY
        );
        assert_eq!(
            payload["remove_audio_tags"],
            ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL
        );
        assert!(payload.get("prompt").is_none());
        assert!(payload.get("keyterms_prompt").is_none());
        assert!(
            payload["redact_pii_policies"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("person_name"))
        );
        assert!(
            payload["redact_pii_policies"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("phone_number"))
        );
        assert_eq!(
            payload["webhook_auth_header_name"],
            "x-casebuilder-assemblyai-secret"
        );
        assert!(!payload.to_string().contains("ASSEMBLYAI_API_KEY"));
    }

    #[test]
    fn assemblyai_speaker_diarization_options_are_validated_and_sent() {
        let job = test_transcription_job(false);
        let mut request = CreateTranscriptionRequest {
            force: None,
            language_code: None,
            redact_pii: Some(false),
            speaker_labels: Some(true),
            speakers_expected: Some(3),
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: None,
            prompt: None,
            keyterms_prompt: Vec::new(),
            remove_audio_tags: None,
        };

        validate_assemblyai_transcription_request(&request).unwrap();
        let payload = serde_json::to_value(assemblyai_transcript_create_request(
            "assemblyai://upload",
            &request,
            &job,
            None,
            None,
        ))
        .unwrap();
        assert_eq!(payload["speaker_labels"], true);
        assert_eq!(payload["speakers_expected"], 3);
        assert!(payload.get("speaker_options").is_none());

        request.speakers_expected = None;
        request.speaker_options = Some(AssemblyAiSpeakerOptions {
            min_speakers_expected: Some(3),
            max_speakers_expected: Some(5),
        });
        validate_assemblyai_transcription_request(&request).unwrap();
        let payload = serde_json::to_value(assemblyai_transcript_create_request(
            "assemblyai://upload",
            &request,
            &job,
            None,
            None,
        ))
        .unwrap();
        assert_eq!(payload["speaker_options"]["min_speakers_expected"], 3);
        assert_eq!(payload["speaker_options"]["max_speakers_expected"], 5);
        assert!(payload.get("speakers_expected").is_none());

        request.speakers_expected = Some(2);
        assert!(validate_assemblyai_transcription_request(&request).is_err());

        request.speakers_expected = None;
        request.speaker_options = Some(AssemblyAiSpeakerOptions {
            min_speakers_expected: Some(6),
            max_speakers_expected: Some(5),
        });
        assert!(validate_assemblyai_transcription_request(&request).is_err());

        request.speaker_labels = Some(false);
        request.speaker_options = None;
        request.speakers_expected = Some(2);
        assert!(validate_assemblyai_transcription_request(&request).is_err());
    }

    #[test]
    fn assemblyai_universal3_prompt_options_are_validated() {
        let mut request = CreateTranscriptionRequest {
            force: None,
            language_code: None,
            redact_pii: Some(false),
            speaker_labels: Some(true),
            speakers_expected: None,
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: None,
            prompt: Some(" Preserve legal names and exact exhibit references. ".to_string()),
            keyterms_prompt: Vec::new(),
            remove_audio_tags: Some("ALL".to_string()),
        };

        validate_assemblyai_transcription_request(&request).unwrap();
        assert_eq!(
            sanitize_assemblyai_prompt(request.prompt.as_deref()).as_deref(),
            Some("Preserve legal names and exact exhibit references.")
        );
        assert_eq!(
            normalize_assemblyai_remove_audio_tags(request.remove_audio_tags.as_deref()).unwrap(),
            Some(ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL.to_string())
        );

        request.keyterms_prompt = vec!["Kelly Byrne-Donoghue".to_string()];
        assert!(validate_assemblyai_transcription_request(&request).is_err());

        request.prompt = None;
        request.keyterms_prompt = vec![
            "Kelly Byrne-Donoghue".to_string(),
            "kelly byrne-donoghue".to_string(),
            "one two three four five six seven".to_string(),
        ];
        assert_eq!(
            sanitize_assemblyai_keyterms(request.keyterms_prompt.iter().map(String::as_str)),
            vec!["Kelly Byrne-Donoghue".to_string()]
        );

        let mut budgeted_keyterms = (0..995)
            .map(|index| format!("term{index}"))
            .collect::<Vec<_>>();
        budgeted_keyterms.push("one two three four five".to_string());
        budgeted_keyterms.push("overflow".to_string());
        let sanitized = sanitize_assemblyai_keyterms(budgeted_keyterms.iter().map(String::as_str));
        assert_eq!(
            assemblyai_keyterms_word_count(&sanitized),
            ASSEMBLYAI_KEYTERMS_MAX_WORDS_TOTAL
        );
        assert!(!sanitized.contains(&"overflow".to_string()));

        request.remove_audio_tags = Some("events".to_string());
        assert!(validate_assemblyai_transcription_request(&request).is_err());

        request.remove_audio_tags = None;
        request.prompt = Some(vec!["word"; ASSEMBLYAI_PROMPT_MAX_WORDS + 1].join(" "));
        request.keyterms_prompt = Vec::new();
        assert!(validate_assemblyai_transcription_request(&request).is_err());
    }

    #[test]
    fn assemblyai_keyterms_prompt_is_sent_when_prompt_is_absent() {
        let job = test_transcription_job(false);
        let request = CreateTranscriptionRequest {
            force: None,
            language_code: None,
            redact_pii: Some(false),
            speaker_labels: Some(true),
            speakers_expected: None,
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: None,
            prompt: None,
            keyterms_prompt: vec!["ORS 90.320".to_string(), "Kelly Byrne-Donoghue".to_string()],
            remove_audio_tags: None,
        };

        let payload = serde_json::to_value(assemblyai_transcript_create_request(
            "assemblyai://upload",
            &request,
            &job,
            None,
            None,
        ))
        .unwrap();

        assert_eq!(
            payload["keyterms_prompt"],
            serde_json::json!(["ORS 90.320", "Kelly Byrne-Donoghue"])
        );
        assert_eq!(
            payload["remove_audio_tags"],
            ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL
        );
        assert!(payload.get("prompt").is_none());
    }

    #[test]
    fn assemblyai_prompt_presets_follow_async_prompting_guide() {
        let mut request = CreateTranscriptionRequest {
            force: None,
            language_code: None,
            redact_pii: Some(false),
            speaker_labels: Some(true),
            speakers_expected: None,
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: Some("legal".to_string()),
            prompt: None,
            keyterms_prompt: Vec::new(),
            remove_audio_tags: None,
        };

        validate_assemblyai_transcription_request(&request).unwrap();
        assert_eq!(
            normalize_assemblyai_prompt_preset(Some("unclear-masked")).unwrap(),
            Some("unclear_masked".to_string())
        );
        let prompt = assemblyai_effective_prompt(&request)
            .unwrap()
            .expect("legal preset produces a prompt");
        assert!(prompt.contains("Mandatory:"));
        assert!(prompt.contains("legal"));
        assert!(prompt.split_whitespace().count() <= ASSEMBLYAI_PROMPT_MAX_WORDS);

        let payload = serde_json::to_value(assemblyai_transcript_create_request(
            "assemblyai://upload",
            &request,
            &test_transcription_job(false),
            None,
            None,
        ))
        .unwrap();
        assert_eq!(payload["prompt"], assemblyai_prompt_preset_text("legal"));

        request.keyterms_prompt = vec!["exhibit a".to_string()];
        assert!(validate_assemblyai_transcription_request(&request).is_err());

        request.keyterms_prompt = Vec::new();
        request.prompt = Some("Explicit prompt wins.".to_string());
        assert_eq!(
            assemblyai_effective_prompt(&request).unwrap().as_deref(),
            Some("Explicit prompt wins.")
        );

        request.prompt = None;
        request.prompt_preset = Some("best transcript ever".to_string());
        assert!(validate_assemblyai_transcription_request(&request).is_err());
    }

    #[test]
    fn assemblyai_get_response_allows_documented_null_arrays() {
        let provider: AssemblyAiTranscriptResponse = serde_json::from_value(serde_json::json!({
            "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
            "status": "completed",
            "text": "Transcript is ready.",
            "utterances": null,
            "words": null,
            "unredacted_text": null,
            "unredacted_utterances": null,
            "unredacted_words": null,
            "language_code": "en_us",
            "audio_duration": 281,
            "confidence": 0.9959,
            "redact_pii": false,
            "redact_pii_return_unredacted": null
        }))
        .unwrap();

        assert_eq!(provider.status, "completed");
        assert_eq!(provider.text.as_deref(), Some("Transcript is ready."));
        assert!(provider.utterances.is_empty());
        assert!(provider.words.is_empty());
        assert!(provider.unredacted_utterances.is_empty());
        assert!(provider.unredacted_words.is_empty());
        assert_eq!(provider.audio_duration, Some(281.0));
    }

    #[test]
    fn assemblyai_sentences_response_matches_documented_shape() {
        let response: AssemblyAiSentencesResponse = serde_json::from_value(serde_json::json!({
            "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
            "confidence": 0.95,
            "audio_duration": 4.82,
            "sentences": [
                {
                    "text": "Mary called 503-555-1212.",
                    "start": 250,
                    "end": 4820,
                    "confidence": 0.95,
                    "channel": "1",
                    "speaker": "A",
                    "words": [
                        {
                            "text": "Mary",
                            "start": 250,
                            "end": 650,
                            "confidence": 0.95,
                            "speaker": "A",
                            "channel": "1"
                        }
                    ]
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.sentences.len(), 1);
        assert_eq!(response.sentences[0].speaker.as_deref(), Some("A"));
        assert_eq!(response.sentences[0].channel.as_deref(), Some("1"));
        assert_eq!(response.sentences[0].words[0].channel.as_deref(), Some("1"));
    }

    #[test]
    fn assemblyai_paragraphs_response_matches_documented_shape() {
        let response: AssemblyAiParagraphsResponse = serde_json::from_value(serde_json::json!({
            "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
            "confidence": 0.95,
            "audio_duration": 4.82,
            "paragraphs": [
                {
                    "text": "Mary called 503-555-1212.",
                    "start": 250,
                    "end": 4820,
                    "confidence": 0.95,
                    "words": [
                        {
                            "text": "Mary",
                            "start": 250,
                            "end": 650,
                            "confidence": 0.95,
                            "speaker": "A",
                            "channel": "1"
                        }
                    ]
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.paragraphs.len(), 1);
        assert_eq!(
            response.paragraphs[0].words[0].speaker.as_deref(),
            Some("A")
        );
        assert_eq!(
            response.paragraphs[0].words[0].channel.as_deref(),
            Some("1")
        );
    }

    #[test]
    fn assemblyai_subtitle_helpers_match_export_contract() {
        assert_eq!(AssemblyAiSubtitleFormat::Srt.as_str(), "srt");
        assert_eq!(AssemblyAiSubtitleFormat::Vtt.as_str(), "vtt");
        assert_eq!(ASSEMBLYAI_CAPTION_CHARS_PER_CAPTION, 80);

        let provider = AssemblyAiTranscriptResponse {
            id: "provider:transcript".to_string(),
            status: "completed".to_string(),
            text: Some("Redacted caption.".to_string()),
            utterances: Vec::new(),
            words: Vec::new(),
            unredacted_text: Some("Raw caption.".to_string()),
            unredacted_utterances: Vec::new(),
            unredacted_words: Vec::new(),
            language_code: None,
            audio_duration: None,
            confidence: None,
            error: None,
            redact_pii: Some(true),
            redact_pii_return_unredacted: Some(true),
        };
        assert!(should_use_assemblyai_subtitles(&provider, true));
        assert!(should_use_assemblyai_subtitles(&provider, false));

        let provider_without_redaction = AssemblyAiTranscriptResponse {
            unredacted_text: None,
            redact_pii: Some(false),
            redact_pii_return_unredacted: None,
            ..provider
        };
        assert!(!should_use_assemblyai_subtitles(
            &provider_without_redaction,
            true
        ));
        assert!(should_use_assemblyai_subtitles(
            &provider_without_redaction,
            false
        ));

        let (subtitle, source) =
            provider_subtitle_or_local(Some("WEBVTT\n\nprovider".to_string()), "local".to_string());
        assert_eq!(subtitle, "WEBVTT\n\nprovider");
        assert_eq!(source, "assemblyai_subtitles");

        let (subtitle, source) =
            provider_subtitle_or_local(Some("  ".to_string()), "local".to_string());
        assert_eq!(subtitle, "local");
        assert_eq!(source, "casebuilder_local");
    }

    #[test]
    fn assemblyai_redacted_audio_response_matches_documented_shape() {
        let response: AssemblyAiRedactedAudioResponse = serde_json::from_value(serde_json::json!({
            "status": "redacted_audio_ready",
            "redacted_audio_url": "https://example.test/redacted-audio/transcript.mp3"
        }))
        .unwrap();

        assert_eq!(response.status, "redacted_audio_ready");
        assert_eq!(
            response.redacted_audio_url,
            "https://example.test/redacted-audio/transcript.mp3"
        );
    }

    #[test]
    fn assemblyai_word_search_response_matches_documented_shape() {
        let response: AssemblyAiWordSearchResponse = serde_json::from_value(serde_json::json!({
            "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
            "total_count": 3,
            "matches": [
                {
                    "text": "notice",
                    "count": 2,
                    "timestamps": [[250, 650], [1200, 1800]],
                    "indexes": [0, 9]
                },
                {
                    "text": "rent payment",
                    "count": 1,
                    "timestamps": [[2200, 3200]],
                    "indexes": [15]
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.total_count, 3);
        assert_eq!(response.matches[0].text, "notice");
        assert_eq!(response.matches[0].timestamps[0], vec![250, 650]);
        assert_eq!(response.matches[1].indexes, vec![15]);
    }

    #[test]
    fn assemblyai_word_search_terms_are_bounded_and_case_relevant() {
        let terms = sanitize_assemblyai_word_search_terms([
            " Rent ",
            "rent",
            "ORS 90.320",
            "one two three four five six",
            "that",
        ]);

        assert_eq!(terms, vec!["rent", "ors 90.320"]);

        let provider = AssemblyAiTranscriptResponse {
            id: "provider:transcript".to_string(),
            status: "completed".to_string(),
            text: Some(
                "The tenant sent a rent payment receipt. The tenant emailed a repair notice."
                    .to_string(),
            ),
            utterances: Vec::new(),
            words: Vec::new(),
            unredacted_text: None,
            unredacted_utterances: Vec::new(),
            unredacted_words: Vec::new(),
            language_code: None,
            audio_duration: None,
            confidence: None,
            error: None,
            redact_pii: Some(false),
            redact_pii_return_unredacted: None,
        };
        let terms = assemblyai_default_word_search_terms(
            &provider,
            &AssemblyAiSentencesResponse {
                id: provider.id.clone(),
                confidence: None,
                audio_duration: None,
                sentences: Vec::new(),
            },
            &AssemblyAiParagraphsResponse {
                id: provider.id.clone(),
                confidence: None,
                audio_duration: None,
                paragraphs: Vec::new(),
            },
        );

        assert!(terms.len() <= ASSEMBLYAI_WORD_SEARCH_MAX_TERMS);
        assert!(terms.contains(&"payment".to_string()));
        assert!(terms.contains(&"receipt".to_string()));
        assert!(terms.contains(&"tenant".to_string()));
    }

    #[test]
    fn assemblyai_transcript_list_response_matches_documented_shape() {
        let response: AssemblyAiTranscriptListResponse = serde_json::from_value(serde_json::json!({
            "page_details": {
                "limit": 10,
                "result_count": 1,
                "current_url": "https://api.assemblyai.com/v2/transcript?limit=10",
                "prev_url": "https://api.assemblyai.com/v2/transcript?before_id=9ea68fd3-f953-42c1-9742-976c447fb463",
                "next_url": null
            },
            "transcripts": [
                {
                    "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
                    "resource_url": "https://api.assemblyai.com/v2/transcript/9ea68fd3-f953-42c1-9742-976c447fb463",
                    "status": "completed",
                    "created": "2026-05-01T10:00:00.000Z",
                    "completed": "2026-05-01T10:01:00.000Z",
                    "audio_url": "https://example.test/audio.mp3",
                    "error": null
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.page_details.limit, 10);
        assert_eq!(response.page_details.result_count, 1);
        assert_eq!(response.page_details.next_url, None);
        assert_eq!(response.transcripts[0].status, "completed");
        assert_eq!(
            response.transcripts[0].completed.as_deref(),
            Some("2026-05-01T10:01:00.000Z")
        );
    }

    #[test]
    fn assemblyai_transcript_list_query_is_validated_and_serialized() {
        let query = normalize_assemblyai_transcript_list_query(AssemblyAiTranscriptListQuery {
            limit: Some(25),
            status: Some(" Completed ".to_string()),
            created_on: Some("2026-05-01".to_string()),
            before_id: Some("9ea68fd3-f953-42c1-9742-976c447fb463".to_string()),
            after_id: None,
            throttled_only: Some(false),
        })
        .unwrap();
        assert_eq!(query.status.as_deref(), Some("completed"));

        let pairs = assemblyai_transcript_list_query_pairs(&query);
        assert!(pairs.contains(&("limit", "25".to_string())));
        assert!(pairs.contains(&("status", "completed".to_string())));
        assert!(pairs.contains(&("created_on", "2026-05-01".to_string())));
        assert!(pairs.contains(&(
            "before_id",
            "9ea68fd3-f953-42c1-9742-976c447fb463".to_string()
        )));
        assert!(pairs.contains(&("throttled_only", "false".to_string())));

        let default_query =
            normalize_assemblyai_transcript_list_query(AssemblyAiTranscriptListQuery::default())
                .unwrap();
        assert_eq!(
            default_query.limit,
            Some(ASSEMBLYAI_TRANSCRIPT_LIST_DEFAULT_LIMIT)
        );
        assert!(
            normalize_assemblyai_transcript_list_query(AssemblyAiTranscriptListQuery {
                limit: Some(0),
                ..AssemblyAiTranscriptListQuery::default()
            })
            .is_err()
        );
        assert!(
            normalize_assemblyai_transcript_list_query(AssemblyAiTranscriptListQuery {
                status: Some("deleted".to_string()),
                ..AssemblyAiTranscriptListQuery::default()
            })
            .is_err()
        );
        assert!(
            normalize_assemblyai_transcript_list_query(AssemblyAiTranscriptListQuery {
                created_on: Some("05/01/2026".to_string()),
                ..AssemblyAiTranscriptListQuery::default()
            })
            .is_err()
        );
    }

    #[test]
    fn assemblyai_transcript_delete_response_preserves_provider_payload() {
        let response: AssemblyAiTranscriptDeleteResponse = assemblyai_transcript_delete_response(
            "9ea68fd3-f953-42c1-9742-976c447fb463",
            serde_json::json!({
                "id": "9ea68fd3-f953-42c1-9742-976c447fb463",
                "status": "deleted",
                "audio_url": "https://example.test/audio.mp3",
                "text": null
            }),
        );

        assert_eq!(response.id, "9ea68fd3-f953-42c1-9742-976c447fb463");
        assert_eq!(response.status, "deleted");
        assert!(response.deleted);
        assert_eq!(
            response.provider_response["audio_url"].as_str(),
            Some("https://example.test/audio.mp3")
        );
    }

    #[test]
    fn assemblyai_transcript_id_is_required_for_delete() {
        assert!(normalize_assemblyai_transcript_id("  ").is_err());
        assert_eq!(
            normalize_assemblyai_transcript_id("9ea68fd3-f953-42c1-9742-976c447fb463").unwrap(),
            "9ea68fd3-f953-42c1-9742-976c447fb463"
        );
    }

    #[test]
    fn assemblyai_errors_keep_safe_operational_context() {
        let error =
            assemblyai_http_error("transcript submission", reqwest::StatusCode::BAD_REQUEST);
        assert_eq!(
            sanitized_external_error(&error),
            "AssemblyAI transcript submission failed with HTTP 400 Bad Request."
        );

        let generic_error = ApiError::External("raw external provider response".to_string());
        assert_eq!(
            sanitized_external_error(&generic_error),
            "External transcription provider request failed."
        );

        let provider = AssemblyAiTranscriptResponse {
            id: "provider:transcript".to_string(),
            status: "error".to_string(),
            text: None,
            utterances: Vec::new(),
            words: Vec::new(),
            unredacted_text: None,
            unredacted_utterances: Vec::new(),
            unredacted_words: Vec::new(),
            language_code: None,
            audio_duration: None,
            confidence: None,
            error: Some(" Invalid media URL.\nTry another file. ".to_string()),
            redact_pii: None,
            redact_pii_return_unredacted: None,
        };
        assert_eq!(
            assemblyai_transcript_error_message(&provider),
            "AssemblyAI returned an error status: Invalid media URL. Try another file."
        );
    }

    #[test]
    fn assemblyai_return_unredacted_preserves_raw_segments() {
        let mut document = test_case_document("doc:media", "hearing.mp4");
        document.mime_type = Some("video/mp4".to_string());
        let job = test_transcription_job(true);
        let provider = AssemblyAiTranscriptResponse {
            id: "provider:transcript".to_string(),
            status: "completed".to_string(),
            text: Some("[PERSON_NAME] called [PHONE_NUMBER].".to_string()),
            utterances: vec![AssemblyAiUtterance {
                speaker: Some("A".to_string()),
                channel: None,
                text: "[PERSON_NAME] called [PHONE_NUMBER].".to_string(),
                start: 250,
                end: 4820,
                confidence: Some(0.95),
            }],
            words: vec![
                AssemblyAiWord {
                    text: "[PERSON_NAME]".to_string(),
                    start: 250,
                    end: 650,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
                AssemblyAiWord {
                    text: "called".to_string(),
                    start: 700,
                    end: 1050,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
                AssemblyAiWord {
                    text: "[PHONE_NUMBER].".to_string(),
                    start: 1100,
                    end: 1800,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
            ],
            unredacted_text: Some("Mary called 503-555-1212.".to_string()),
            unredacted_utterances: vec![AssemblyAiUtterance {
                speaker: Some("A".to_string()),
                channel: None,
                text: "Mary called 503-555-1212.".to_string(),
                start: 250,
                end: 4820,
                confidence: Some(0.95),
            }],
            unredacted_words: vec![
                AssemblyAiWord {
                    text: "Mary".to_string(),
                    start: 250,
                    end: 650,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
                AssemblyAiWord {
                    text: "called".to_string(),
                    start: 700,
                    end: 1050,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
                AssemblyAiWord {
                    text: "503-555-1212.".to_string(),
                    start: 1100,
                    end: 1800,
                    confidence: Some(0.95),
                    speaker: Some("A".to_string()),
                    channel: None,
                },
            ],
            language_code: Some("en_us".to_string()),
            audio_duration: Some(4.82),
            confidence: Some(0.95),
            error: None,
            redact_pii: Some(true),
            redact_pii_return_unredacted: Some(true),
        };
        let sentences = AssemblyAiSentencesResponse {
            id: provider.id.clone(),
            confidence: Some(0.95),
            audio_duration: Some(4.82),
            sentences: vec![AssemblyAiSentence {
                text: "[PERSON_NAME] called [PHONE_NUMBER].".to_string(),
                start: 250,
                end: 4820,
                confidence: Some(0.95),
                words: provider.words.clone(),
                channel: None,
                speaker: Some("A".to_string()),
            }],
        };
        let paragraphs = AssemblyAiParagraphsResponse {
            id: provider.id.clone(),
            confidence: Some(0.95),
            audio_duration: Some(4.82),
            paragraphs: vec![AssemblyAiParagraph {
                text: "[PERSON_NAME] called [PHONE_NUMBER].".to_string(),
                start: 250,
                end: 4820,
                confidence: Some(0.95),
                words: provider.words.clone(),
            }],
        };
        let (segments, speakers) = transcript_segments_from_provider(
            "matter:test",
            &document,
            &job,
            &provider,
            &sentences,
            &paragraphs,
            "1",
        );
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Mary called 503-555-1212.");
        assert_eq!(
            segments[0].redacted_text.as_deref(),
            Some("[PERSON_NAME] called [PHONE_NUMBER].")
        );
        assert_eq!(segments[0].speaker_label.as_deref(), Some("A"));
        assert_eq!(segments[0].paragraph_ordinal, Some(1));
        assert_eq!(speakers.len(), 1);
        assert_eq!(assemblyai_raw_words(&provider)[0].text, "Mary");
    }

    #[test]
    fn transcript_redaction_and_captions_are_deterministic() {
        let redacted = redact_transcript_text(
            "Call me at 503-555-1212 or email tenant@example.com. SSN 123-45-6789.",
        );
        assert!(redacted.contains("[redacted phone]"));
        assert!(redacted.contains("[redacted email]"));
        assert!(redacted.contains("[redacted ssn]"));
        let segment = crate::models::casebuilder::TranscriptSegment {
            segment_id: "segment:1".to_string(),
            id: "segment:1".to_string(),
            matter_id: "matter:test".to_string(),
            document_id: "doc:test".to_string(),
            transcription_job_id: "transcription:1".to_string(),
            source_span_id: Some("span:1".to_string()),
            ordinal: 1,
            paragraph_ordinal: Some(1),
            speaker_label: Some("A".to_string()),
            speaker_name: None,
            channel: None,
            text: "Hello there.".to_string(),
            redacted_text: Some("Hello there.".to_string()),
            time_start_ms: 1_000,
            time_end_ms: 2_500,
            confidence: 0.9,
            review_status: "unreviewed".to_string(),
            edited: false,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        };
        let vtt = transcript_segments_to_vtt(&[segment.clone()], true);
        let srt = transcript_segments_to_srt(&[segment], true);
        assert!(vtt.contains("00:00:01.000 --> 00:00:02.500"));
        assert!(srt.contains("00:00:01,000 --> 00:00:02,500"));
    }

    #[test]
    fn docx_manifest_detects_editable_text_parts() {
        let bytes = stored_zip_document_xml(
            br#"<w:document><w:body><w:p><w:r><w:t>Simple editable paragraph.</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let document = test_case_document("doc:test", "simple.docx");
        let manifest = docx_package_manifest(&document, None, &bytes).unwrap();
        assert_eq!(manifest.entry_count, 1);
        assert_eq!(manifest.text_part_count, 1);
        assert!(manifest.editable);
        assert!(
            manifest
                .text_preview
                .as_deref()
                .unwrap()
                .contains("Simple editable paragraph.")
        );
    }

    #[test]
    fn docx_round_trip_replaces_document_xml_and_keeps_package_readable() {
        let bytes = stored_zip_document_xml(
            br#"<w:document><w:body><w:p><w:r><w:t>Old paragraph.</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let (updated, warnings) =
            docx_with_replaced_document_xml(&bytes, "New paragraph.\n\nSecond paragraph.").unwrap();
        assert!(!warnings.is_empty());
        let parsed = parse_document_bytes(
            "simple.docx",
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            &updated,
        );
        assert_eq!(parsed.status, "processed");
        let text = parsed.text.unwrap();
        assert!(text.contains("New paragraph."));
        assert!(text.contains("Second paragraph."));
        assert!(!text.contains("Old paragraph."));
    }

    #[test]
    fn docx_round_trip_rejects_complex_objects_for_v1() {
        let bytes = stored_zip_document_xml(
            br#"<w:document><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table text.</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
        );
        let error = docx_with_replaced_document_xml(&bytes, "Replacement").unwrap_err();
        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    fn test_case_document(document_id: &str, filename: &str) -> CaseDocument {
        CaseDocument {
            document_id: document_id.to_string(),
            id: document_id.to_string(),
            matter_id: "matter:test".to_string(),
            filename: filename.to_string(),
            title: filename.to_string(),
            document_type: "other".to_string(),
            mime_type: Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    .to_string(),
            ),
            pages: 1,
            bytes: 0,
            file_hash: None,
            uploaded_at: "1".to_string(),
            source: "user_upload".to_string(),
            confidentiality: "private".to_string(),
            processing_status: "stored".to_string(),
            is_exhibit: false,
            exhibit_label: None,
            summary: String::new(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder: "Uploads".to_string(),
            storage_path: None,
            storage_provider: "local".to_string(),
            storage_status: "stored".to_string(),
            storage_bucket: None,
            storage_key: None,
            content_etag: None,
            upload_expires_at: None,
            deleted_at: None,
            original_relative_path: None,
            upload_batch_id: None,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: None,
        }
    }

    fn test_transcription_job(redact_pii: bool) -> TranscriptionJob {
        TranscriptionJob {
            transcription_job_id: "transcription:doc:test".to_string(),
            id: "transcription:doc:test".to_string(),
            matter_id: "matter:test".to_string(),
            document_id: "doc:media".to_string(),
            document_version_id: Some("version:doc:media:original".to_string()),
            object_blob_id: Some("blob:sha256:test".to_string()),
            provider: "assemblyai".to_string(),
            provider_mode: "live".to_string(),
            provider_transcript_id: Some("provider:transcript".to_string()),
            provider_status: Some("completed".to_string()),
            status: "review_ready".to_string(),
            review_status: "needs_review".to_string(),
            raw_artifact_version_id: None,
            normalized_artifact_version_id: None,
            redacted_artifact_version_id: None,
            redacted_audio_version_id: None,
            reviewed_document_version_id: None,
            caption_vtt_version_id: None,
            caption_srt_version_id: None,
            language_code: None,
            duration_ms: None,
            speaker_count: 0,
            segment_count: 0,
            word_count: 0,
            speakers_expected: None,
            speaker_options: None,
            word_search_terms: Vec::new(),
            prompt_preset: None,
            prompt: None,
            keyterms_prompt: Vec::new(),
            remove_audio_tags: Some(ASSEMBLYAI_REMOVE_AUDIO_TAGS_ALL.to_string()),
            redact_pii,
            speech_models: assemblyai_speech_models(),
            retryable: false,
            error_code: None,
            error_message: None,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            completed_at: None,
            reviewed_at: None,
        }
    }

    fn stored_zip_document_xml(xml: &[u8]) -> Vec<u8> {
        let name = b"word/document.xml";
        let mut zip = Vec::new();
        push_u32(&mut zip, 0x0403_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, xml.len() as u32);
        push_u32(&mut zip, xml.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        zip.extend_from_slice(name);
        zip.extend_from_slice(xml);

        let central_directory_offset = zip.len();
        push_u32(&mut zip, 0x0201_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, xml.len() as u32);
        push_u32(&mut zip, xml.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, 0);
        zip.extend_from_slice(name);

        let central_directory_size = zip.len() - central_directory_offset;
        push_u32(&mut zip, 0x0605_4b50);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 1);
        push_u16(&mut zip, 1);
        push_u32(&mut zip, central_directory_size as u32);
        push_u32(&mut zip, central_directory_offset as u32);
        push_u16(&mut zip, 0);
        zip
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn complaint_import_parser_preserves_labels_counts_and_citations() {
        let text = r#"
# PAYNTER v. BLUE OX RV PARK - MASTER CIVIL COMPLAINT
## CAPTION
**IN THE CIRCUIT COURT OF THE STATE OF OREGON**
**FOR THE COUNTY OF LINN**

## I. FACTUAL ALLEGATIONS
1. Plaintiffs began occupying Space #076 on October 27, 2025.
10A. Plaintiff Debra Paynter is an elderly person under ORS 124.100(1)(a).

### FIRST CLAIM FOR RELIEF - RETALIATION
42. Defendants retaliated in violation of ORS 90.385 and ORCP 16 D.

## PRAYER FOR RELIEF
43. Plaintiffs request injunctive relief under UTCR 2.010.
"#;
        assert!(looks_like_complaint("master_complaint.md", text));
        let parsed = parse_complaint_structure(text);
        assert_eq!(parsed.paragraphs.len(), 4);
        assert_eq!(parsed.paragraphs[1].original_label, "10A");
        assert_eq!(parsed.counts.len(), 1);

        let citations = citation_uses_for_text(
            "matter:test",
            "complaint:test",
            "complaint:test:paragraph:2",
            "paragraph",
            &parsed.paragraphs[1].text,
        );
        assert_eq!(citations[0].canonical_id.as_deref(), Some("or:ors:124.100"));

        let rule_citations = citation_uses_for_text(
            "matter:test",
            "complaint:test",
            "complaint:test:paragraph:4",
            "paragraph",
            &parsed.paragraphs[3].text,
        );
        assert_eq!(rule_citations[0].status, "resolved");
        assert_eq!(
            rule_citations[0].canonical_id.as_deref(),
            Some("or:utcr:2.010")
        );
    }

    #[test]
    fn citation_canonical_ids_cover_ors_orcp_and_utcr() {
        assert_eq!(
            canonical_id_for_citation("ORS 90.385"),
            Some("or:ors:90.385".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("ORCP 16 D"),
            Some("or:orcp:16-d".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("UTCR 2.010(4)"),
            Some("or:utcr:2.010".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Or Laws 2023, ch 13, § 4"),
            Some("or:session-law:2023:ch:13:sec:4".to_string())
        );
    }
}
