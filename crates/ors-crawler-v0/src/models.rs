use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceDocument {
    pub source_document_id: String,
    pub source_provider: String,
    pub source_kind: String,
    pub url: String,
    pub chapter: String,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub edition_id: Option<String>,
    #[serde(default)]
    pub authority_family: Option<String>,
    #[serde(default)]
    pub authority_type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub page_count: Option<usize>,
    #[serde(default)]
    pub effective_date: Option<String>,
    #[serde(default)]
    pub copyright_status: Option<String>,
    #[serde(default)]
    pub chapter_title: Option<String>,
    pub edition_year: i32,
    #[serde(default)]
    pub html_encoding: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub paragraph_count: Option<usize>,
    #[serde(default)]
    pub first_body_paragraph_index: Option<usize>,
    #[serde(default)]
    pub parser_profile: Option<String>,
    pub official_status: String,
    pub disclaimer_required: bool,
    pub raw_hash: String,
    pub normalized_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalCorpus {
    pub corpus_id: String,
    pub name: String,
    pub short_name: String,
    pub authority_family: String,
    pub authority_type: String,
    pub jurisdiction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorpusEdition {
    pub edition_id: String,
    pub corpus_id: String,
    pub edition_year: i32,
    #[serde(default)]
    pub effective_date: Option<String>,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub current: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcePage {
    pub source_page_id: String,
    pub source_document_id: String,
    pub page_number: usize,
    #[serde(default)]
    pub printed_label: Option<String>,
    pub text: String,
    pub normalized_text: String,
    pub text_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceTocEntry {
    pub source_toc_entry_id: String,
    pub source_document_id: String,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub title: String,
    pub chapter: Option<String>,
    pub page_label: Option<String>,
    pub page_number: Option<usize>,
    pub toc_order: usize,
    pub entry_type: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourtRuleChapter {
    pub chapter_id: String,
    pub corpus_id: String,
    pub edition_id: String,
    pub chapter: String,
    pub title: String,
    pub citation: String,
    pub edition_year: i32,
    pub effective_date: String,
    #[serde(default)]
    pub source_page_start: Option<usize>,
    #[serde(default)]
    pub source_page_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalTextIdentity {
    pub canonical_id: String,
    pub citation: String,
    pub jurisdiction_id: String,
    pub authority_family: String,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub authority_type: Option<String>,
    #[serde(default)]
    pub authority_level: Option<i32>,
    #[serde(default)]
    pub effective_date: Option<String>,
    pub title: Option<String>,
    pub chapter: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalTextVersion {
    pub version_id: String,
    pub canonical_id: String,
    pub citation: String,
    pub title: Option<String>,
    pub chapter: String,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub edition_id: Option<String>,
    #[serde(default)]
    pub authority_family: Option<String>,
    #[serde(default)]
    pub authority_type: Option<String>,
    #[serde(default)]
    pub authority_level: Option<i32>,
    #[serde(default)]
    pub effective_date: Option<String>,
    #[serde(default)]
    pub source_page_start: Option<usize>,
    #[serde(default)]
    pub source_page_end: Option<usize>,
    pub edition_year: i32,
    pub status: String,
    pub status_text: Option<String>,
    pub text: String,
    pub text_hash: String,
    #[serde(default)]
    pub original_text: Option<String>,
    #[serde(default)]
    pub paragraph_start_order: Option<usize>,
    #[serde(default)]
    pub paragraph_end_order: Option<usize>,
    #[serde(default)]
    pub source_paragraph_ids: Vec<String>,
    pub source_document_id: String,
    pub official_status: String,
    pub disclaimer_required: bool,
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<i32>,
    pub embedding: Option<Vec<f32>>,
    pub embedding_input_hash: Option<String>,
    pub embedding_input_type: Option<String>,
    pub embedding_output_dtype: Option<String>,
    pub embedded_at: Option<String>,
    pub embedding_profile: Option<String>,
    pub embedding_strategy: Option<String>,
    #[serde(default)]
    pub embedding_source_dimension: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Provision {
    pub provision_id: String,
    pub version_id: String,
    pub canonical_id: String,
    pub citation: String,
    pub display_citation: String,
    #[serde(default)]
    pub chapter: Option<String>,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub edition_id: Option<String>,
    #[serde(default)]
    pub authority_family: Option<String>,
    #[serde(default)]
    pub authority_type: Option<String>,
    #[serde(default)]
    pub authority_level: Option<i32>,
    #[serde(default)]
    pub effective_date: Option<String>,
    #[serde(default)]
    pub source_page_start: Option<usize>,
    #[serde(default)]
    pub source_page_end: Option<usize>,
    pub local_path: Vec<String>,
    pub provision_type: String,
    pub text: String,
    #[serde(default)]
    pub original_text: Option<String>,
    pub normalized_text: String,
    pub order_index: usize,
    pub depth: usize,
    pub text_hash: String,
    pub is_implied: bool,
    pub is_definition_candidate: bool,
    pub is_exception_candidate: bool,
    pub is_deadline_candidate: bool,
    pub is_penalty_candidate: bool,
    #[serde(default)]
    pub paragraph_start_order: Option<usize>,
    #[serde(default)]
    pub paragraph_end_order: Option<usize>,
    #[serde(default)]
    pub source_paragraph_ids: Vec<String>,
    #[serde(default)]
    pub heading_path: Vec<String>,
    #[serde(default)]
    pub structural_context: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<i32>,
    pub embedding: Option<Vec<f32>>,
    pub embedding_input_hash: Option<String>,
    pub embedding_input_type: Option<String>,
    pub embedding_output_dtype: Option<String>,
    pub embedded_at: Option<String>,
    pub embedding_profile: Option<String>,
    #[serde(default)]
    pub embedding_source_dimension: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CitationMention {
    pub citation_mention_id: String,
    pub source_provision_id: String,
    pub raw_text: String,
    pub normalized_citation: String,
    pub citation_type: String,
    pub target_canonical_id: Option<String>,
    pub target_start_canonical_id: Option<String>,
    pub target_end_canonical_id: Option<String>,
    pub target_provision_id: Option<String>,
    pub unresolved_subpath: Option<Vec<String>>,
    #[serde(default)]
    pub external_citation_id: Option<String>,
    pub resolver_status: String,
    pub confidence: f32,
    pub qc_severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrievalChunk {
    pub chunk_id: String,
    pub chunk_type: String,
    pub text: String,
    pub breadcrumb: String,
    #[serde(default)]
    pub source_provision_id: Option<String>,
    #[serde(default)]
    pub source_version_id: Option<String>,
    pub parent_version_id: String,
    pub canonical_id: String,
    pub citation: String,
    pub jurisdiction_id: String,
    pub authority_level: i32,
    #[serde(default)]
    pub authority_family: Option<String>,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub authority_type: Option<String>,
    #[serde(default)]
    pub effective_date: Option<String>,
    #[serde(default)]
    pub chapter: Option<String>,
    #[serde(default)]
    pub source_page_start: Option<usize>,
    #[serde(default)]
    pub source_page_end: Option<usize>,
    pub edition_year: i32,
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<i32>,
    pub embedding: Option<Vec<f32>>,
    pub embedding_input_hash: String,
    pub embedding_policy: Option<String>,
    pub answer_policy: Option<String>,
    pub chunk_schema_version: Option<String>,
    pub retrieval_profile: Option<String>,
    pub search_weight: Option<f32>,
    pub embedding_input_type: Option<String>,
    pub embedding_output_dtype: Option<String>,
    pub embedded_at: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    #[serde(default)]
    pub token_count: Option<usize>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub context_window: Option<usize>,
    #[serde(default)]
    pub chunking_strategy: Option<String>,
    #[serde(default)]
    pub chunk_version: Option<String>,
    #[serde(default)]
    pub overlap_tokens: Option<usize>,
    #[serde(default)]
    pub split_reason: Option<String>,
    #[serde(default)]
    pub part_index: Option<usize>,
    #[serde(default)]
    pub part_count: Option<usize>,
    pub is_definition_candidate: bool,
    pub is_exception_candidate: bool,
    pub is_penalty_candidate: bool,
    #[serde(default)]
    pub heading_path: Vec<String>,
    #[serde(default)]
    pub structural_context: Option<String>,
    #[serde(default)]
    pub embedding_profile: Option<String>,
    #[serde(default)]
    pub embedding_source_dimension: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedChunk {
    pub chunk_id: String,
    pub text: String,
    pub citation: Option<String>,
    pub breadcrumb: String,
    pub score: f64,
    pub citations: Vec<EnrichedCitation>,
    pub definitions: Vec<EnrichedDefinition>,
    pub status: Option<String>,
    pub edition_year: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedCitation {
    pub citation: String,
    pub target_citation: String,
    pub target_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedDefinition {
    pub term: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HtmlParagraph {
    pub paragraph_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub order_index: usize,
    #[serde(default)]
    pub raw_html: String,
    pub raw_text: String,
    pub cleaned_text: String,
    pub normalized_text: String,
    pub bold_text: Option<String>,
    pub has_bold: bool,
    pub has_underline: bool,
    pub has_italic: bool,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub margin_left: Option<String>,
    #[serde(default)]
    pub text_indent: Option<String>,
    #[serde(default)]
    pub style_raw: Option<String>,
    pub style_hint: Option<String>,
    pub class_hint: Option<String>,
    pub source_document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassifiedParagraph {
    pub paragraph: HtmlParagraph,
    pub kind: ChapterParagraphKind,
    pub confidence: f32,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterParagraphKind {
    FrontMatter,
    TocRow,
    TocHeading,
    BodySectionHeader,
    SectionCaption,
    Content,
    StructuralHeading,
    SourceNote,
    LegislativeHistory,
    TemporalNote,
    StatusNote,
    ReservedTail,
    Separator,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedChapter {
    pub chapter: String,
    pub edition_year: i32,
    pub source_document: SourceDocument,
    pub identities: Vec<LegalTextIdentity>,
    pub versions: Vec<LegalTextVersion>,
    pub provisions: Vec<Provision>,
    pub citations: Vec<CitationMention>,
    pub chunks: Vec<RetrievalChunk>,
    pub headings: Vec<ChapterHeading>,
    #[serde(default)]
    pub html_paragraphs_debug: Vec<HtmlParagraph>,
    #[serde(default)]
    pub chapter_front_matter: Vec<ChapterFrontMatter>,
    #[serde(default)]
    pub title_chapter_entries: Vec<TitleChapterEntry>,
    #[serde(default)]
    pub source_notes: Vec<SourceNote>,
    #[serde(default)]
    pub amendments: Vec<Amendment>,
    #[serde(default)]
    pub chapter_toc_entries: Vec<ChapterTocEntry>,
    #[serde(default)]
    pub reserved_ranges: Vec<ReservedRange>,
    #[serde(default)]
    pub time_intervals: Vec<TimeInterval>,
    #[serde(default)]
    pub parser_diagnostic_rows: Vec<ParserDiagnostic>,
    #[serde(default)]
    pub parser_diagnostics: ParserDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterHeading {
    pub heading_id: String,
    pub chapter: String,
    pub text: String,
    pub order_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceNote {
    pub source_note_id: String,
    pub note_type: String,
    pub text: String,
    #[serde(default)]
    pub normalized_text: String,
    pub source_document_id: String,
    pub canonical_id: String,
    pub version_id: Option<String>,
    #[serde(default)]
    pub provision_id: Option<String>,
    pub citation: String,
    pub paragraph_start_order: usize,
    pub paragraph_end_order: usize,
    #[serde(default)]
    pub source_paragraph_order: usize,
    pub source_paragraph_ids: Vec<String>,
    #[serde(default)]
    pub confidence: f32,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReporterNote {
    pub reporter_note_id: String,
    pub source_document_id: String,
    pub canonical_id: Option<String>,
    pub version_id: Option<String>,
    pub source_provision_id: Option<String>,
    pub citation: Option<String>,
    pub text: String,
    pub normalized_text: String,
    pub source_page_start: Option<usize>,
    pub source_page_end: Option<usize>,
    pub confidence: f32,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Commentary {
    pub commentary_id: String,
    pub source_document_id: String,
    pub canonical_id: Option<String>,
    pub version_id: Option<String>,
    pub source_provision_id: Option<String>,
    #[serde(default)]
    pub target_canonical_id: Option<String>,
    #[serde(default)]
    pub target_provision_id: Option<String>,
    pub citation: Option<String>,
    #[serde(default)]
    pub authority_family: Option<String>,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub authority_level: Option<i32>,
    #[serde(default)]
    pub source_role: Option<String>,
    pub commentary_type: String,
    pub text: String,
    pub normalized_text: String,
    pub source_page_start: Option<usize>,
    pub source_page_end: Option<usize>,
    pub confidence: f32,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalLegalCitation {
    pub external_citation_id: String,
    pub citation: String,
    pub normalized_citation: String,
    pub citation_type: String,
    pub jurisdiction_id: String,
    pub source_system: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParserDiagnostics {
    pub chapter: String,
    pub edition_year: i32,
    pub total_mso_normal: usize,
    pub section_starts_detected: usize,
    pub skipped_note_paragraphs: usize,
    pub skipped_structural_headings: usize,
    pub reserved_tail_stops: usize,
    pub paragraphs_ignored_before_body_start: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParserDiagnostic {
    pub parser_diagnostic_id: String,
    pub source_document_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub severity: String,
    pub diagnostic_type: String,
    pub message: String,
    pub source_paragraph_order: Option<usize>,
    pub related_id: Option<String>,
    pub parser_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterTocEntry {
    pub toc_entry_id: String,
    pub source_document_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub caption: String,
    pub heading_path: Vec<String>,
    pub toc_order: usize,
    pub source_paragraph_order: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterFrontMatter {
    pub front_matter_id: String,
    pub source_document_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub title_number: Option<String>,
    pub title_name: Option<String>,
    pub chapter_number: Option<String>,
    pub chapter_name: Option<String>,
    pub text: String,
    pub source_paragraph_order: usize,
    pub front_matter_type: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TitleChapterEntry {
    pub title_chapter_entry_id: String,
    pub source_document_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub title_number: Option<String>,
    pub title_name: Option<String>,
    pub chapter_number: String,
    pub chapter_name: String,
    pub chapter_list_order: usize,
    pub source_paragraph_order: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReservedRange {
    pub reserved_range_id: String,
    pub source_document_id: String,
    pub chapter: String,
    pub edition_year: i32,
    pub range_text: String,
    pub start_chapter: Option<String>,
    pub end_chapter: Option<String>,
    pub start_title: Option<String>,
    pub end_title: Option<String>,
    pub source_paragraph_order: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalEffect {
    pub temporal_effect_id: String,
    pub source_note_id: Option<String>,
    pub source_provision_id: Option<String>,
    pub version_id: Option<String>,
    pub canonical_id: Option<String>,
    pub effect_type: String,
    pub trigger_text: String,
    pub effective_date: Option<String>,
    pub operative_date: Option<String>,
    pub repeal_date: Option<String>,
    pub expiration_date: Option<String>,
    pub session_law_ref: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineageEvent {
    pub lineage_event_id: String,
    pub source_note_id: Option<String>,
    pub from_canonical_id: Option<String>,
    pub to_canonical_id: Option<String>,
    pub current_canonical_id: String,
    pub lineage_type: String,
    pub raw_text: String,
    pub year: Option<i32>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CitesEdge {
    pub edge_id: String,
    pub edge_type: String,
    pub source_provision_id: String,
    pub target_canonical_id: Option<String>,
    pub target_version_id: Option<String>,
    pub target_provision_id: Option<String>,
    pub target_chapter_id: Option<String>,
    pub citation_kind: Option<String>,
    pub citation_mention_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusEvent {
    pub status_event_id: String,
    pub status_type: String,
    pub status_text: Option<String>,
    pub source_document_id: Option<String>,
    pub canonical_id: String,
    pub version_id: Option<String>,
    pub event_year: Option<i32>,
    pub effective_date: Option<String>,
    #[serde(default)]
    pub source_note_id: Option<String>,
    #[serde(default)]
    pub effect_type: Option<String>,
    #[serde(default)]
    pub trigger_text: Option<String>,
    #[serde(default)]
    pub operative_date: Option<String>,
    #[serde(default)]
    pub repeal_date: Option<String>,
    #[serde(default)]
    pub session_law_ref: Option<String>,
    pub confidence: f32,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Amendment {
    pub amendment_id: String,
    pub amendment_type: String,
    pub session_law_citation: Option<String>,
    pub effective_date: Option<String>,
    pub text: String,
    #[serde(default)]
    pub raw_text: Option<String>,
    pub source_document_id: Option<String>,
    pub confidence: f32,
    pub canonical_id: Option<String>,
    pub version_id: Option<String>,
    pub session_law_id: Option<String>,
    #[serde(default)]
    pub affected_canonical_id: Option<String>,
    #[serde(default)]
    pub affected_version_id: Option<String>,
    #[serde(default)]
    pub source_note_id: Option<String>,
    #[serde(default)]
    pub proposal_method: Option<String>,
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub measure_number: Option<String>,
    #[serde(default)]
    pub resolution_chamber: Option<String>,
    #[serde(default)]
    pub resolution_number: Option<String>,
    #[serde(default)]
    pub filed_date: Option<String>,
    #[serde(default)]
    pub proposed_year: Option<i32>,
    #[serde(default)]
    pub adopted_date: Option<String>,
    #[serde(default)]
    pub election_date: Option<String>,
    #[serde(default)]
    pub resolution_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionLaw {
    pub session_law_id: String,
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
    pub citation: String,
    pub year: i32,
    pub chapter: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    pub bill_number: Option<String>,
    pub effective_date: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub raw_text: Option<String>,
    pub source_document_id: Option<String>,
    #[serde(default)]
    pub source_note_id: Option<String>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeInterval {
    pub time_interval_id: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub label: Option<String>,
    pub certainty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Jurisdiction {
    pub jurisdiction_id: String,
    pub name: String,
    pub jurisdiction_type: String,
    #[serde(default)]
    pub parent_jurisdiction_id: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Court {
    pub court_id: String,
    pub name: String,
    pub court_type: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub county_jurisdiction_id: Option<String>,
    #[serde(default)]
    pub judicial_district_id: Option<String>,
    #[serde(default)]
    pub judicial_district: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourtRulesRegistrySource {
    pub registry_source_id: String,
    pub source_type: String,
    pub jurisdiction: String,
    pub jurisdiction_id: String,
    pub source_url: String,
    pub snapshot_date: String,
    pub contains_current_future: bool,
    pub contains_prior: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourtRulesRegistrySnapshot {
    pub registry_snapshot_id: String,
    pub registry_source_id: String,
    pub snapshot_date: String,
    pub jurisdiction_id: String,
    pub source_url: String,
    pub parser_profile: String,
    pub entry_count: usize,
    pub input_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulePublicationEntry {
    pub publication_entry_id: String,
    pub registry_source_id: String,
    pub registry_snapshot_id: String,
    pub authority_document_id: String,
    pub effective_interval_id: String,
    pub title: String,
    pub jurisdiction: String,
    pub jurisdiction_id: String,
    pub subcategory: String,
    pub authority_kind: String,
    pub publication_bucket: String,
    pub table_section: String,
    pub row_index: usize,
    pub effective_start_date: String,
    #[serde(default)]
    pub effective_end_date: Option<String>,
    pub date_status: String,
    #[serde(default)]
    pub status_flags: Vec<String>,
    #[serde(default)]
    pub authority_identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleAuthorityDocument {
    pub authority_document_id: String,
    pub title: String,
    pub jurisdiction_id: String,
    pub jurisdiction: String,
    pub subcategory: String,
    pub authority_kind: String,
    #[serde(default)]
    pub authority_identifier: Option<String>,
    pub effective_start_date: String,
    #[serde(default)]
    pub effective_end_date: Option<String>,
    pub publication_bucket: String,
    pub date_status: String,
    #[serde(default)]
    pub status_flags: Vec<String>,
    #[serde(default)]
    pub topic_ids: Vec<String>,
    #[serde(default)]
    pub amends_authority_document_id: Option<String>,
    pub source_registry_id: String,
    pub source_snapshot_id: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupplementaryLocalRuleEdition {
    pub edition_id: String,
    pub authority_document_id: String,
    pub corpus_id: String,
    #[serde(default)]
    pub supplements_corpus_id: Option<String>,
    pub jurisdiction_id: String,
    pub court_id: String,
    pub edition_year: i32,
    pub title: String,
    pub effective_start_date: String,
    #[serde(default)]
    pub effective_end_date: Option<String>,
    pub date_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectiveInterval {
    pub effective_interval_id: String,
    pub authority_document_id: String,
    pub start_date: String,
    #[serde(default)]
    pub end_date: Option<String>,
    pub label: String,
    pub certainty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleTopic {
    pub rule_topic_id: String,
    pub name: String,
    pub normalized_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleSupersessionEdge {
    pub edge_id: String,
    pub from_authority_document_id: String,
    pub to_authority_document_id: String,
    pub relationship_type: String,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleApplicabilityEdge {
    pub edge_id: String,
    pub authority_document_id: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub court_id: Option<String>,
    pub relationship_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkProductRulePackAuthority {
    pub rule_pack_authority_id: String,
    pub rule_pack_id: String,
    pub authority_document_id: String,
    pub work_product_type: String,
    pub jurisdiction_id: String,
    pub inclusion_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefinedTerm {
    pub defined_term_id: String,
    pub term: String,
    pub normalized_term: String,
    pub jurisdiction_id: String,
    pub authority_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Definition {
    pub definition_id: String,
    pub term: String,
    pub normalized_term: String,
    pub definition_text: String,
    pub scope_type: Option<String>,
    pub scope_citation: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
    pub review_status: Option<String>,
    pub extraction_method: String,
    pub defined_term_id: Option<String>,
    pub definition_scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefinitionScope {
    pub definition_scope_id: String,
    pub scope_type: String,
    pub scope_citation: Option<String>,
    pub target_canonical_id: Option<String>,
    pub target_chapter_id: Option<String>,
    pub target_range_start: Option<String>,
    pub target_range_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalSemanticNode {
    pub semantic_id: String,
    pub semantic_type: String,
    pub text: String,
    pub normalized_text: String,
    pub source_provision_id: String,
    pub confidence: f32,
    pub review_status: Option<String>,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProceduralRequirement {
    pub requirement_id: String,
    pub semantic_type: String,
    pub requirement_type: String,
    pub label: String,
    pub text: String,
    pub normalized_text: String,
    pub source_provision_id: String,
    pub source_citation: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub severity_default: Option<String>,
    pub authority_family: String,
    pub effective_date: String,
    pub confidence: f32,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkProductRulePack {
    pub rule_pack_id: String,
    pub name: String,
    pub jurisdiction: String,
    pub court_system: String,
    pub effective_date: String,
    pub source_corpus_id: String,
    pub source_edition_id: String,
    #[serde(default)]
    pub work_product_types: Vec<String>,
    #[serde(default)]
    pub inherits: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormattingProfile {
    pub formatting_profile_id: String,
    pub name: String,
    pub source_corpus_id: String,
    pub source_edition_id: String,
    pub effective_date: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulePackMembership {
    pub membership_id: String,
    pub rule_pack_id: String,
    pub requirement_id: String,
    pub requirement_type: String,
    pub source_provision_id: String,
    pub source_citation: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub severity_default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Obligation {
    pub obligation_id: String,
    pub text: String,
    pub actor_text: Option<String>,
    pub action_text: Option<String>,
    pub object_text: Option<String>,
    pub condition_text: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
    pub actor_id: Option<String>,
    pub action_id: Option<String>,
    pub deadline_id: Option<String>,
    pub exception_id: Option<String>,
    pub penalty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Exception {
    pub exception_id: String,
    pub text: String,
    pub trigger_phrase: Option<String>,
    pub exception_type: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
    pub target_provision_id: Option<String>,
    pub target_canonical_id: Option<String>,
    pub target_obligation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deadline {
    pub deadline_id: String,
    pub text: String,
    pub duration: Option<String>,
    pub date_text: Option<String>,
    pub trigger_event: Option<String>,
    pub actor: Option<String>,
    pub action_required: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
    pub obligation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Penalty {
    pub penalty_id: String,
    pub text: String,
    pub penalty_type: Option<String>,
    pub amount: Option<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub condition: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
    pub obligation_id: Option<String>,
    #[serde(default)]
    pub criminal_class: Option<String>,
    #[serde(default)]
    pub civil_penalty_amount: Option<String>,
    #[serde(default)]
    pub min_amount: Option<String>,
    #[serde(default)]
    pub max_amount: Option<String>,
    #[serde(default)]
    pub jail_term: Option<String>,
    #[serde(default)]
    pub license_suspension: Option<bool>,
    #[serde(default)]
    pub revocation: Option<bool>,
    #[serde(default)]
    pub target_conduct: Option<String>,
    #[serde(default)]
    pub target_citation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Remedy {
    pub remedy_id: String,
    pub text: String,
    pub remedy_type: Option<String>,
    pub available_to: Option<String>,
    pub available_against: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalActor {
    pub actor_id: String,
    pub name: String,
    pub normalized_name: String,
    pub actor_type: Option<String>,
    pub jurisdiction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalAction {
    pub action_id: String,
    pub verb: String,
    pub object: Option<String>,
    pub normalized_action: String,
    #[serde(default)]
    pub source_provision_id: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoneyAmount {
    pub money_amount_id: String,
    pub amount_text: String,
    pub amount_value: Option<f64>,
    pub percent_value: Option<f64>,
    pub amount_type: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaxRule {
    pub tax_rule_id: String,
    pub tax_type: Option<String>,
    pub rate_text: Option<String>,
    pub base: Option<String>,
    pub cap: Option<String>,
    pub recipient: Option<String>,
    pub fund_name: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimit {
    pub rate_limit_id: String,
    pub rate_type: Option<String>,
    pub percent_value: Option<f64>,
    pub amount_text: Option<String>,
    pub cap_text: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequiredNotice {
    pub required_notice_id: String,
    pub notice_type: Option<String>,
    pub text: String,
    pub required_recipient: Option<String>,
    pub required_sender: Option<String>,
    pub trigger_event: Option<String>,
    pub source_provision_id: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormText {
    pub form_text_id: String,
    pub form_type: Option<String>,
    pub text: String,
    pub source_provision_id: String,
    pub source_paragraph_ids: Vec<String>,
    pub confidence: f32,
}

// ── QC Report Models ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QcStatus {
    #[default]
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcFullReport {
    pub run_id: String,
    pub generated_at: String,
    pub edition_year: i32,
    pub status: QcStatus,
    pub source: QcSourceStats,
    pub parse: QcParseStats,
    pub chunks: QcChunkStats,
    pub citations: QcCitationStats,
    pub graph: QcGraphStats,
    pub embedding_readiness: QcEmbeddingReadiness,
    pub provision_embedding_readiness: QcProvisionEmbeddingReadiness,
    pub version_embedding_readiness: QcVersionEmbeddingReadiness,
    pub resolver_readiness: QcResolverReadiness,
    pub coverage: QcCoverageStats,
    pub semantic: QcSemanticStats,
    pub golden: QcGoldenStats,
    pub blocking_errors: Vec<String>,
    pub warnings: Vec<String>,
    pub examples: QcExamples,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcSourceStats {
    pub chapters_expected: usize,
    pub chapters_fetched: usize,
    pub fetch_failures: usize,
    pub raw_html_files: usize,
    pub raw_html_bytes: u64,
    pub empty_html_files: usize,
    pub tiny_html_files: usize,
    pub empty_raw_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcParseStats {
    pub sections: usize,
    pub versions: usize,
    pub provisions: usize,
    pub valid_provisions: usize,
    pub duplicate_canonical_ids: usize,
    pub duplicate_version_ids: usize,
    pub duplicate_provision_ids: usize,
    pub duplicate_provision_paths: usize,
    pub repaired_duplicate_paths: usize,
    pub implied_parent_paths: usize,
    pub orphan_provisions: usize,
    pub active_sections_missing_titles: usize,
    pub heading_leaks: usize,
    pub invalid_status_classification: usize,
    pub active_with_empty_text: usize,
    pub repealed_classified_active: usize,
    pub identities_count: usize,
    pub versions_count: usize,
    pub provisions_count: usize,
    pub headings_count: usize,
    pub missing_text: usize,
    pub suspicious_short_text: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcChunkStats {
    pub total_chunks: usize,
    pub full_statute_chunks: usize,
    pub contextual_provision_chunks: usize,
    pub definition_chunks: usize,
    pub orphan_chunks: usize,
    pub empty_chunks: usize,
    pub duplicate_chunk_ids: usize,
    pub missing_embedding_input_hash: usize,
    pub missing_embedding_policy: usize,
    pub missing_answer_policy: usize,
    pub missing_retrieval_profile: usize,
    pub missing_chunk_schema_version: usize,
    pub missing_search_weight: usize,
    pub oversized_chunks_warn: usize,
    pub oversized_chunks_fail: usize,
    pub invalid_answer_policy: usize,
    pub generated_marked_authoritative: usize,
    pub invalid_chunk_schema_version: usize,
    pub exception_chunks: usize,
    pub deadline_chunks: usize,
    pub penalty_chunks: usize,
    pub citation_context_chunks: usize,
    #[serde(default)]
    pub missing_token_count: usize,
    #[serde(default)]
    pub missing_max_tokens: usize,
    #[serde(default)]
    pub missing_context_window: usize,
    #[serde(default)]
    pub missing_chunking_strategy: usize,
    #[serde(default)]
    pub missing_chunk_version: usize,
    #[serde(default)]
    pub invalid_part_metadata: usize,
    #[serde(default)]
    pub chunks_over_max_tokens: usize,
    #[serde(default)]
    pub chunks_over_hard_token_limit: usize,
    #[serde(default)]
    pub max_token_count: usize,
    #[serde(default)]
    pub p50_token_count: usize,
    #[serde(default)]
    pub p95_token_count: usize,
    #[serde(default)]
    pub p99_token_count: usize,
    #[serde(default)]
    pub chunk_version_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub chunking_strategy_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub split_reason_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub chunk_type_token_distribution: BTreeMap<String, QcTokenDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcTokenDistribution {
    pub count: usize,
    pub max: usize,
    pub p50: usize,
    pub p95: usize,
    pub p99: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcCitationStats {
    pub citation_mentions: usize,
    pub parsed_unverified: usize,
    pub resolved: usize,
    pub unresolved: usize,
    pub unsupported: usize,
    pub resolution_pending: bool,
    pub orphan_citation_mentions: usize,
    pub total_mentions: usize,
    pub suspicious_resolution: usize,
    pub resolved_section: usize,
    pub resolved_section_and_provision: usize,
    pub resolved_chapter: usize,
    pub resolved_range: usize,
    pub resolved_section_unresolved_subpath: usize,
    pub unresolved_target_not_in_corpus: usize,
    pub unresolved_malformed_citation: usize,
    pub unsupported_citation_type: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcGraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub orphan_edges: usize,
    pub duplicate_edge_ids: usize,
    pub cites_edges: usize,
    pub total_edges: usize,
}

#[derive(Debug, Serialize)]
pub struct SeedStats {
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_secs: Option<f64>,
    pub graph_dir: String,
    pub embedded_direct_to_neo4j: bool,
    pub local_embedding_file_written: bool,
    pub model: String,
    pub dimension: i32,
    pub total_chunks: usize,
    pub eligible_chunks: usize,
    pub embedded_chunks: usize,
    pub failed_chunks: usize,
    pub skipped_chunks: usize,
    pub batch_count: usize,
    pub total_tokens_estimated: usize,
}

#[derive(Debug, Serialize)]
pub struct SeedFailure {
    pub chunk_id: String,
    pub stage: String,
    pub error_kind: String,
    pub message: String,
    pub retryable: bool,
    pub attempts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcEmbeddingReadiness {
    pub model: String,
    pub dimension: usize,
    pub model_context_tokens: usize,
    pub model_batch_token_limit: usize,
    pub batch_token_safety_limit: usize,
    pub eligible_chunks: usize,
    pub chunks_over_context_limit: usize,
    pub chunks_over_warning_limit: usize,
    pub chunks_missing_input_hash: usize,
    pub chunks_with_invalid_dimension: usize,
    pub estimated_total_tokens: usize,
    pub estimated_batches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcProvisionEmbeddingReadiness {
    pub model: String,
    pub dimension: usize,
    pub eligible_provisions: usize,
    pub estimated_total_tokens: usize,
    pub estimated_batches: usize,
    pub model_context_tokens: usize,
    pub model_batch_token_limit: usize,
    pub batch_token_safety_limit: usize,
    pub provisions_over_context_limit: usize,
    pub provisions_missing_input_hash: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcVersionEmbeddingReadiness {
    pub model: String,
    pub dimension: usize,
    pub eligible_versions: usize,
    pub estimated_total_tokens: usize,
    pub estimated_batches: usize,
    pub model_context_tokens: usize,
    pub model_batch_token_limit: usize,
    pub batch_token_safety_limit: usize,
    pub versions_over_context_limit: usize,
    pub versions_missing_input_hash: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcResolverReadiness {
    pub identity_index_ready: bool,
    pub version_index_ready: bool,
    pub provision_path_index_ready: bool,
    pub chapter_index_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcCoverageStats {
    pub active_versions: usize,
    pub full_statute_chunks: usize,
    pub active_versions_missing_full_statute_chunk: usize,
    pub versions_with_duplicate_full_statute_chunks: usize,
    pub valid_provisions: usize,
    pub contextual_provision_chunks: usize,
    pub provisions_missing_contextual_chunk: usize,
    pub provisions_with_duplicate_contextual_chunks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcExamples {
    pub duplicate_provision_ids: Vec<String>,
    pub orphan_chunks: Vec<String>,
    pub heading_leaks: Vec<String>,
    pub unresolved_citations: Vec<String>,
    pub bad_edges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcGoldenStats {
    pub golden_tests_present: bool,
    pub golden_tests_passed: bool,
    pub golden_files_found: usize,
    pub golden_files_expected: usize,
    pub search_queries_tested: usize,
    pub search_queries_passed: usize,
    pub citation_extraction_tested: usize,
    pub citation_extraction_passed: usize,
    pub citation_resolution_tested: usize,
    pub citation_resolution_passed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QcSemanticStats {
    // Counts by type
    pub status_events: usize,
    pub source_notes: usize,
    pub html_paragraphs: usize,
    pub chapter_front_matter: usize,
    pub title_chapter_entries: usize,
    pub chapter_toc_entries: usize,
    pub reserved_ranges: usize,
    pub parser_diagnostics: usize,
    pub temporal_effects: usize,
    pub lineage_events: usize,
    pub session_laws: usize,
    pub amendments: usize,
    pub defined_terms: usize,
    pub definitions: usize,
    pub definition_scopes: usize,
    pub legal_semantic_nodes: usize,
    pub obligations: usize,
    pub exceptions: usize,
    pub deadlines: usize,
    pub penalties: usize,
    pub remedies: usize,
    pub legal_actors: usize,
    pub legal_actions: usize,
    pub money_amounts: usize,
    pub tax_rules: usize,
    pub rate_limits: usize,
    pub required_notices: usize,
    pub form_texts: usize,
    // Orphan counts (source ref not in provisions/versions)
    pub orphan_definitions: usize,
    pub orphan_legal_semantic_nodes: usize,
    pub orphan_obligations: usize,
    pub orphan_exceptions: usize,
    pub orphan_deadlines: usize,
    pub orphan_penalties: usize,
    pub orphan_remedies: usize,
    pub orphan_source_notes: usize,
    pub orphan_html_paragraphs: usize,
    pub orphan_chapter_front_matter: usize,
    pub orphan_title_chapter_entries: usize,
    pub orphan_temporal_effects: usize,
    pub orphan_lineage_events: usize,
    pub orphan_chapter_toc_entries: usize,
    pub orphan_money_amounts: usize,
    pub orphan_tax_rules: usize,
    pub orphan_rate_limits: usize,
    pub orphan_required_notices: usize,
    pub orphan_form_texts: usize,
    // Missing linkage
    pub status_events_missing_version_id: usize,
    pub temporal_effects_missing_support: usize,
    pub lineage_events_missing_current_canonical_id: usize,
    pub toc_entries_missing_identity: usize,
    pub invalid_definition_scope_types: usize,
    pub penalties_missing_detail: usize,
    pub definitions_missing_defined_term_id: usize,
    pub definitions_missing_scope_id: usize,
    pub semantic_nodes_missing_source_provision_id: usize,
    pub obligations_missing_source_provision_id: usize,
    // Confidence range violations
    pub invalid_confidence_count: usize,
    // Duplicate IDs in derived semantic surfaces
    pub duplicate_defined_term_ids: usize,
    pub duplicate_definition_ids: usize,
    pub duplicate_definition_scope_ids: usize,
    pub duplicate_legal_semantic_node_ids: usize,
}
