use orsgraph_api::models::casebuilder::{DocumentVersion, IngestionRun, ObjectBlob, SourceSpan};

fn casebuilder_service_sources() -> String {
    [
        include_str!("../src/services/casebuilder/mod.rs"),
        include_str!("../src/services/casebuilder/authority.rs"),
        include_str!("../src/services/casebuilder/complaints.rs"),
        include_str!("../src/services/casebuilder/documents.rs"),
        include_str!("../src/services/casebuilder/graph_projection.rs"),
        include_str!("../src/services/casebuilder/indexes.rs"),
        include_str!("../src/services/casebuilder/matters.rs"),
        include_str!("../src/services/casebuilder/repository.rs"),
        include_str!("../src/services/casebuilder/storage.rs"),
        include_str!("../src/services/casebuilder/transcription.rs"),
        include_str!("../src/services/casebuilder/work_products.rs"),
    ]
    .join("\n")
}

#[test]
fn api_queries_use_loader_relationship_vocabulary() {
    let service = include_str!("../src/services/neo4j.rs");

    for forbidden in [
        "BELONGS_TO",
        "HAS_CURRENT_VERSION",
        "FROM_SOURCE",
        "PARENT_OF",
    ] {
        assert!(
            !service.contains(forbidden),
            "API query layer still references loader-incompatible relationship {forbidden}"
        );
    }

    for expected in [
        "HAS_VERSION",
        "VERSION_OF",
        "CONTAINS",
        "PART_OF_VERSION",
        "DERIVED_FROM",
        "HAS_PARENT",
        "EXPRESSES",
        "DEFINES",
        "HAS_SOURCE_NOTE",
        "HAS_STATUS_EVENT",
        "HAS_TEMPORAL_EFFECT",
        "CITES_VERSION",
        "CITES_PROVISION",
        "CITES_CHAPTER",
        "CITES_RANGE",
    ] {
        assert!(
            service.contains(expected),
            "API query layer should use loader relationship {expected}"
        );
    }
}

#[test]
fn container_runs_api_by_default_and_keeps_crawler_escape_hatch() {
    let dockerfile = include_str!("../../../Dockerfile");
    let entrypoint = include_str!("../../../docker-entrypoint.sh");

    assert!(dockerfile.contains("/app/target/release/orsgraph-api /app/orsgraph-api"));
    assert!(entrypoint.contains("RUN_CRAWLER_ONLY"));
    assert!(entrypoint.contains("exec /app/orsgraph-api"));
}

#[test]
fn graph_routes_and_frontend_are_wired_end_to_end() {
    let routes = include_str!("../src/routes/mod.rs");
    let graph_routes = include_str!("../src/routes/graph.rs");
    let graph_page = include_str!("../../../frontend/app/graph/page.tsx");
    let graph_viewer = include_str!("../../../frontend/components/graph/GraphViewer.tsx");
    let graph_toolbar = include_str!("../../../frontend/components/graph/GraphToolbar.tsx");
    let path_panel = include_str!("../../../frontend/components/graph/PathFinderPanel.tsx");

    assert!(routes.contains("/graph/neighborhood"));
    assert!(routes.contains("/graph/path"));
    assert!(graph_routes.contains("GraphPathRequest"));
    assert!(graph_page.contains("searchParams"));
    assert!(graph_page.contains("initialFocus"));
    assert!(graph_viewer.contains("updateGraphUrl"));
    assert!(graph_viewer.contains("PathFinderPanel"));
    assert!(graph_viewer.contains("SheetContent"));
    assert!(graph_toolbar.contains("Open graph controls"));
    assert!(graph_toolbar.contains("Open graph inspector"));
    assert!(path_panel.contains("getGraphPath"));
}

#[test]
fn statute_sidebar_routes_use_live_api_contracts_without_mock_fallbacks() {
    let routes = include_str!("../src/routes/mod.rs");
    let sidebar_routes = include_str!("../src/routes/sidebar.rs");
    let service = include_str!("../src/services/neo4j.rs");
    let frontend_api = include_str!("../../../frontend/lib/api.ts");
    let statute_page = include_str!("../../../frontend/app/statutes/[id]/page.tsx");

    for expected in [
        "/statutes",
        "/statutes/:citation/page",
        "/statutes/:citation/provisions",
        "/statutes/:citation/citations",
        "/statutes/:citation/semantics",
        "/statutes/:citation/history",
        "/statutes/:citation/chunks",
    ] {
        assert!(routes.contains(expected), "missing statute route {expected}");
    }

    for expected in [
        "/sidebar",
        "/sidebar/saved-searches",
        "/sidebar/saved-statutes",
        "/sidebar/recent-statutes",
    ] {
        assert!(
            sidebar_routes.contains(expected),
            "missing sidebar route {expected}"
        );
    }

    assert!(
        service.contains("RETURN size(matched) AS total, matched[$offset..$end] AS items"),
        "statute index API should return an empty live result set instead of 404 for no matches"
    );
    assert!(frontend_api.contains("fetchApi<StatuteIndexApiResponse>(`/statutes?${params}`)"));
    assert!(frontend_api.contains("fetchApi<SidebarData>(\"/sidebar\")"));
    assert!(frontend_api.contains("fetchApi<any>(`/statutes/${encodeURIComponent(citationOrCanonicalId)}/page`)"));
    assert!(frontend_api.contains("apiFailureState(\"/statutes\""));
    assert!(frontend_api.contains("apiFailureState(\"/sidebar\", null"));
    assert!(statute_page.contains("state.source === \"empty\""));

    for forbidden in [
        "filterFallbackStatuteIndex",
        "buildFallbackSidebarData",
        "getStatuteByCanonicalId(citationOrCanonicalId)",
        "getStatutePageDataLegacyState",
    ] {
        assert!(
            !frontend_api.contains(forbidden),
            "statute/sidebar frontend should not use fallback helper {forbidden}"
        );
    }
}

#[test]
fn graph_similarity_and_citation_contracts_are_supported() {
    let service = include_str!("../src/services/neo4j.rs");
    let frontend_constants = include_str!("../../../frontend/components/graph/constants.ts");

    for expected in [
        "CITES_VERSION",
        "CITES_PROVISION",
        "CITES_CHAPTER",
        "CITES_RANGE",
        "RESOLVES_TO_CHAPTER",
        "RESOLVES_TO_EXTERNAL",
        "SIMILAR_TO",
    ] {
        assert!(
            service.contains(expected),
            "backend graph contract should include {expected}"
        );
        assert!(
            frontend_constants.contains(expected),
            "frontend graph filters should include {expected}"
        );
    }

    assert!(service.contains("params.include_similarity.unwrap_or(false)"));
    assert!(service.contains("rel.similarity_score, rel.score, rel.weight"));
    assert!(
        !service.contains("Similarity edges are not included by /graph/neighborhood"),
        "similarity mode should be implemented instead of warning as unavailable"
    );
}

#[test]
fn casebuilder_routes_cover_v0_contracts() {
    let routes = include_str!("../src/routes/casebuilder.rs");

    for expected in [
        "/matters",
        "/matters/:matter_id",
        "/matters/:matter_id/graph",
        "/matters/:matter_id/audit",
        "/matters/:matter_id/qc/run",
        "/matters/:matter_id/issues/spot",
        "/matters/:matter_id/files",
        "/matters/:matter_id/files/binary",
        "/matters/:matter_id/files/uploads",
        "/matters/:matter_id/files/uploads/:upload_id/complete",
        "/matters/:matter_id/documents/:document_id/workspace",
        "/matters/:matter_id/documents/:document_id/content",
        "/matters/:matter_id/documents/:document_id/annotations",
        "/matters/:matter_id/documents/:document_id/text",
        "/matters/:matter_id/documents/:document_id/promote-work-product",
        "/matters/:matter_id/documents/:document_id/download-url",
        "/matters/:matter_id/documents/:document_id/transcriptions",
        "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id",
        "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/sync",
        "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/segments/:segment_id",
        "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/speakers/:speaker_id",
        "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/review",
        "/matters/:matter_id/documents/:document_id/extract",
        "/matters/:matter_id/documents/:document_id/import-complaint",
        "/casebuilder/webhooks/assemblyai",
        "/matters/:matter_id/facts/:fact_id/approve",
        "/matters/:matter_id/claims/:claim_id/map-elements",
        "/matters/:matter_id/evidence/:evidence_id/link-fact",
        "/matters/:matter_id/work-products",
        "/matters/:matter_id/work-products/:work_product_id",
        "/matters/:matter_id/work-products/:work_product_id/blocks",
        "/matters/:matter_id/work-products/:work_product_id/blocks/:block_id",
        "/matters/:matter_id/work-products/:work_product_id/links",
        "/matters/:matter_id/work-products/:work_product_id/links/:anchor_id",
        "/matters/:matter_id/work-products/:work_product_id/text-ranges",
        "/matters/:matter_id/work-products/:work_product_id/ast",
        "/matters/:matter_id/work-products/:work_product_id/ast/patch",
        "/matters/:matter_id/work-products/:work_product_id/ast/validate",
        "/matters/:matter_id/work-products/:work_product_id/ast/to-markdown",
        "/matters/:matter_id/work-products/:work_product_id/ast/from-markdown",
        "/matters/:matter_id/work-products/:work_product_id/ast/to-html",
        "/matters/:matter_id/work-products/:work_product_id/ast/to-plain-text",
        "/matters/:matter_id/work-products/:work_product_id/qc/run",
        "/matters/:matter_id/work-products/:work_product_id/qc/findings",
        "/matters/:matter_id/work-products/:work_product_id/qc/findings/:finding_id",
        "/matters/:matter_id/work-products/:work_product_id/preview",
        "/matters/:matter_id/work-products/:work_product_id/export",
        "/matters/:matter_id/work-products/:work_product_id/artifacts/:artifact_id",
        "/matters/:matter_id/work-products/:work_product_id/artifacts/:artifact_id/download",
        "/matters/:matter_id/work-products/:work_product_id/ai/commands",
        "/matters/:matter_id/work-products/:work_product_id/history",
        "/matters/:matter_id/work-products/:work_product_id/change-sets/:change_set_id",
        "/matters/:matter_id/work-products/:work_product_id/snapshots",
        "/matters/:matter_id/work-products/:work_product_id/snapshots/:snapshot_id",
        "/matters/:matter_id/work-products/:work_product_id/compare",
        "/matters/:matter_id/work-products/:work_product_id/restore",
        "/matters/:matter_id/work-products/:work_product_id/export-history",
        "/matters/:matter_id/work-products/:work_product_id/ai-audit",
        "/matters/:matter_id/complaints",
        "/matters/:matter_id/complaints/import",
        "/matters/:matter_id/complaints/:complaint_id",
        "/matters/:matter_id/complaints/:complaint_id/setup",
        "/matters/:matter_id/complaints/:complaint_id/sections",
        "/matters/:matter_id/complaints/:complaint_id/counts",
        "/matters/:matter_id/complaints/:complaint_id/paragraphs",
        "/matters/:matter_id/complaints/:complaint_id/paragraphs/renumber",
        "/matters/:matter_id/complaints/:complaint_id/paragraphs/:paragraph_id",
        "/matters/:matter_id/complaints/:complaint_id/links",
        "/matters/:matter_id/complaints/:complaint_id/qc/run",
        "/matters/:matter_id/complaints/:complaint_id/qc/findings",
        "/matters/:matter_id/complaints/:complaint_id/qc/findings/:finding_id",
        "/matters/:matter_id/complaints/:complaint_id/preview",
        "/matters/:matter_id/complaints/:complaint_id/export",
        "/matters/:matter_id/complaints/:complaint_id/artifacts/:artifact_id",
        "/matters/:matter_id/complaints/:complaint_id/artifacts/:artifact_id/download",
        "/matters/:matter_id/complaints/:complaint_id/ai/commands",
        "/matters/:matter_id/complaints/:complaint_id/history",
        "/matters/:matter_id/complaints/:complaint_id/change-sets/:change_set_id",
        "/matters/:matter_id/complaints/:complaint_id/snapshots",
        "/matters/:matter_id/complaints/:complaint_id/snapshots/:snapshot_id",
        "/matters/:matter_id/complaints/:complaint_id/compare",
        "/matters/:matter_id/complaints/:complaint_id/restore",
        "/matters/:matter_id/complaints/:complaint_id/export-history",
        "/matters/:matter_id/complaints/:complaint_id/ai-audit",
        "/matters/:matter_id/complaints/:complaint_id/filing-packet",
        "/matters/:matter_id/drafts/:draft_id/generate",
        "/matters/:matter_id/drafts/:draft_id/fact-check",
        "/matters/:matter_id/drafts/:draft_id/citation-check",
        "/matters/:matter_id/authority/search",
        "/matters/:matter_id/authority/attach",
        "/matters/:matter_id/authority/detach",
        "/matters/:matter_id/export/docx",
        "/matters/:matter_id/export/pdf",
        "/matters/:matter_id/export/filing-packet",
    ] {
        assert!(
            routes.contains(expected),
            "missing CaseBuilder route {expected}"
        );
    }

    assert!(
        routes.contains("ListWorkProductsParams") && routes.contains("document_ast"),
        "WorkProduct list route should support explicit include=document_ast"
    );
    assert!(
        routes.contains("list_work_product_snapshots_for_api"),
        "snapshot list route should use bounded API summaries"
    );
}

#[test]
fn casebuilder_constraints_cover_core_graph_nodes() {
    let service = casebuilder_service_sources();

    for expected in [
        "casebuilder_matter_id",
        "casebuilder_document_id",
        "casebuilder_fact_id",
        "casebuilder_evidence_id",
        "casebuilder_claim_id",
        "casebuilder_defense_id",
        "casebuilder_element_id",
        "casebuilder_draft_id",
        "casebuilder_draft_paragraph_id",
        "casebuilder_deadline_instance_id",
        "casebuilder_task_id",
        "casebuilder_fact_check_finding_id",
        "casebuilder_citation_check_finding_id",
        "casebuilder_object_blob_id",
        "casebuilder_document_version_id",
        "casebuilder_ingestion_run_id",
        "casebuilder_source_span_id",
        "casebuilder_document_annotation_id",
        "casebuilder_document_annotation_document",
        "casebuilder_transcription_job_id",
        "casebuilder_transcript_segment_id",
        "casebuilder_transcript_speaker_id",
        "casebuilder_transcript_review_change_id",
        "casebuilder_transcription_job_document",
        "casebuilder_transcription_job_provider",
        "casebuilder_transcript_segment_job",
        "casebuilder_transcript_speaker_job",
        "casebuilder_external_authority_id",
        "casebuilder_complaint_id",
        "casebuilder_complaint_section_id",
        "casebuilder_complaint_count_id",
        "casebuilder_pleading_paragraph_id",
        "casebuilder_pleading_sentence_id",
        "casebuilder_citation_use_id",
        "casebuilder_evidence_use_id",
        "casebuilder_exhibit_reference_id",
        "casebuilder_relief_request_id",
        "casebuilder_rule_check_finding_id",
        "casebuilder_export_artifact_id",
        "casebuilder_work_product_id",
        "casebuilder_work_product_block_id",
        "casebuilder_work_product_mark_id",
        "casebuilder_work_product_anchor_id",
        "casebuilder_work_product_finding_id",
        "casebuilder_work_product_artifact_id",
        "casebuilder_work_product_history_event_id",
        "casebuilder_change_set_id",
        "casebuilder_version_snapshot_id",
        "casebuilder_snapshot_manifest_id",
        "casebuilder_snapshot_entity_state_id",
        "casebuilder_version_change_id",
        "casebuilder_version_branch_id",
        "casebuilder_legal_support_use_id",
        "casebuilder_fact_use_id",
        "casebuilder_authority_use_id",
        "casebuilder_element_support_id",
        "casebuilder_ai_edit_audit_id",
        "casebuilder_milestone_id",
        "casebuilder_complaint_fulltext",
        "casebuilder_work_product_fulltext",
    ] {
        assert!(
            service.contains(expected),
            "missing CaseBuilder constraint/index {expected}"
        );
    }
}

#[test]
fn document_workspace_contract_is_oss_only_and_casebuilder_native() {
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let backend_service = casebuilder_service_sources();
    let frontend_types = include_str!("../../../frontend/lib/casebuilder/types.ts");
    let frontend_api = include_str!("../../../frontend/lib/casebuilder/api.ts");
    let frontend_workspace =
        include_str!("../../../frontend/components/casebuilder/document-workspace.tsx");
    let license_gate = include_str!("../../../frontend/scripts/check-oss-licenses.mjs");

    for expected in [
        "struct DocumentWorkspace",
        "struct DocumentCapability",
        "struct DocumentAnnotation",
        "struct DocxPackageManifest",
        "struct DocumentPageRange",
        "struct DocumentTextRange",
        "struct SaveDocumentTextRequest",
        "struct PromoteDocumentWorkProductResponse",
        "struct TranscriptionJob",
        "struct TranscriptSegment",
        "struct TranscriptSpeaker",
        "struct TranscriptReviewChange",
        "struct TranscriptionJobResponse",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend document workspace DTO {expected}"
        );
    }

    for expected in [
        "get_document_workspace",
        "get_document_content_bytes",
        "create_document_annotation",
        "save_document_text",
        "promote_document_work_product",
        "create_transcription",
        "sync_transcription",
        "review_transcription",
        "docx_with_replaced_document_xml",
        "read_zip_package",
        "docx_package_manifest",
        "immutable_pdf_bytes",
        "graph_sidecar",
    ] {
        assert!(
            backend_service.contains(expected),
            "missing backend document workspace behavior {expected}"
        );
    }

    for expected in [
        "interface DocumentWorkspace",
        "interface DocumentCapability",
        "interface DocumentAnnotation",
        "interface DocxPackageManifest",
        "interface TranscriptionJob",
        "interface TranscriptSegment",
        "interface TranscriptionJobResponse",
    ] {
        assert!(
            frontend_types.contains(expected),
            "missing frontend document workspace type {expected}"
        );
    }

    for expected in [
        "getDocumentWorkspace",
        "createDocumentAnnotation",
        "saveDocumentText",
        "promoteDocumentWorkProduct",
        "createTranscription",
        "syncTranscription",
        "reviewTranscription",
        "normalizeDocumentWorkspace",
        "normalizeTranscriptionJobResponse",
    ] {
        assert!(
            frontend_api.contains(expected),
            "missing frontend document workspace API {expected}"
        );
    }

    for expected in [
        "DocumentWorkspace",
        "iframe",
        "Sidecar Annotations",
        "DOCX Package",
        "MediaTranscriptPane",
        "Transcribe",
    ] {
        assert!(
            frontend_workspace.contains(expected),
            "missing frontend document workspace UI marker {expected}"
        );
    }

    for denied in ["onlyoffice", "collabora", "pspdfkit", "pdftron", "aspose"] {
        assert!(
            license_gate.contains(denied),
            "license gate should reject {denied}"
        );
    }
}

#[test]
fn complaint_editor_dtos_and_api_exist_in_backend_and_frontend() {
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let backend_service = casebuilder_service_sources();
    let frontend_types = include_str!("../../../frontend/lib/casebuilder/types.ts");
    let frontend_api = include_str!("../../../frontend/lib/casebuilder/api.ts");
    let frontend_routes = include_str!("../../../frontend/lib/casebuilder/routes.ts");

    for expected in [
        "struct ComplaintDraft",
        "struct ComplaintImportProvenance",
        "struct ComplaintImportRequest",
        "struct ComplaintImportResponse",
        "struct ComplaintSection",
        "struct ComplaintCount",
        "struct PleadingParagraph",
        "struct PleadingSentence",
        "struct CitationUse",
        "struct EvidenceUse",
        "struct ExhibitReference",
        "struct ReliefRequest",
        "struct SignatureBlock",
        "struct CertificateOfService",
        "struct FormattingProfile",
        "struct RulePack",
        "struct RuleCheckFinding",
        "struct ExportArtifact",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend complaint DTO {expected}"
        );
    }

    for expected in [
        "struct WorkProductDocument",
        "struct WorkProductMetadata",
        "struct WorkProductLink",
        "struct WorkProductCitationUse",
        "struct WorkProductExhibitReference",
        "struct AstPatch",
        "enum AstOperation",
        "struct AstValidationResponse",
        "struct AstRenderedResponse",
        "struct ChangeSet",
        "struct VersionChange",
        "struct VersionSnapshot",
        "struct SnapshotManifest",
        "struct SnapshotEntityState",
        "struct VersionBranch",
        "struct LegalImpactSummary",
        "struct VersionChangeSummary",
        "struct VersionLayerDiff",
        "struct AIEditAudit",
        "struct Milestone",
        "struct LegalSupportUse",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend Case History DTO {expected}"
        );
    }
    for expected in [
        "base_document_hash",
        "base_snapshot_id",
        "object_blob_id",
        "full_state_ref",
        "state_ref",
        "storage_ref",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend hybrid AST storage field {expected}"
        );
    }

    for expected in [
        "interface ComplaintDraft",
        "interface ComplaintImportProvenance",
        "interface ComplaintImportResponse",
        "interface ComplaintSection",
        "interface ComplaintCount",
        "interface PleadingParagraph",
        "interface PleadingSentence",
        "interface CitationUse",
        "interface EvidenceUse",
        "interface ExhibitReference",
        "interface ReliefRequest",
        "interface SignatureBlock",
        "interface CertificateOfService",
        "interface FormattingProfile",
        "interface RulePack",
        "interface RuleCheckFinding",
        "interface ExportArtifact",
    ] {
        assert!(
            frontend_types.contains(expected),
            "missing frontend complaint DTO {expected}"
        );
    }

    for expected in [
        "interface WorkProductDocument",
        "interface WorkProductMetadata",
        "interface WorkProductLink",
        "interface WorkProductCitationUse",
        "interface WorkProductExhibitReference",
        "interface AstPatch",
        "type AstOperation",
        "interface AstValidationResponse",
        "interface AstRenderedResponse",
        "interface ChangeSet",
        "interface VersionChange",
        "interface VersionSnapshot",
        "interface SnapshotManifest",
        "interface SnapshotEntityState",
        "interface VersionBranch",
        "interface LegalImpactSummary",
        "interface VersionChangeSummary",
        "interface VersionLayerDiff",
        "interface AIEditAudit",
        "interface CaseHistoryMilestone",
        "interface LegalSupportUse",
    ] {
        assert!(
            frontend_types.contains(expected),
            "missing frontend Case History DTO {expected}"
        );
    }
    for expected in [
        "base_document_hash",
        "base_snapshot_id",
        "object_blob_id",
        "full_state_ref",
        "state_ref",
        "storage_ref",
    ] {
        assert!(
            frontend_types.contains(expected),
            "missing frontend hybrid AST storage field {expected}"
        );
    }

    for expected in [
        "createComplaint",
        "importComplaints",
        "importDocumentComplaint",
        "patchComplaint",
        "createComplaintParagraph",
        "renumberComplaintParagraphs",
        "linkComplaintSupport",
        "runComplaintQc",
        "previewComplaint",
        "exportComplaint",
        "runComplaintAiCommand",
        "getWorkProductAst",
        "patchWorkProductAst",
        "applyWorkProductAstPatch",
        "validateWorkProductAst",
        "workProductAstToMarkdown",
        "workProductAstFromMarkdown",
        "workProductAstToHtml",
        "workProductAstToPlainText",
        "getWorkProductHistory",
        "getWorkProductSnapshots",
        "getWorkProductSnapshot",
        "createWorkProductSnapshot",
        "compareWorkProductVersions",
        "restoreWorkProductVersion",
        "getWorkProductExportHistory",
        "getWorkProductAiAudit",
        "GetWorkProductsOptions",
        "AstPatchConcurrency",
        "normalizeComplaint",
        "buildDemoComplaint",
    ] {
        assert!(
            frontend_api.contains(expected),
            "missing frontend complaint API {expected}"
        );
    }

    for expected in ["matterComplaintHref", "ComplaintWorkspaceSection"] {
        assert!(
            frontend_routes.contains(expected),
            "missing complaint route helper {expected}"
        );
    }

    for expected in [
        "HAS_COMPLAINT",
        "HAS_SECTION",
        "HAS_COUNT",
        "HAS_PARAGRAPH",
        "HAS_SENTENCE",
        "HAS_EVIDENCE_USE",
        "HAS_CITATION_USE",
        "HAS_EXHIBIT_REFERENCE",
        "REQUESTS_RELIEF",
        "SUPPORTED_BY_FACT",
        "SUPPORTED_BY_EVIDENCE",
        "SUPPORTED_BY_AUTHORITY",
        "RESOLVES_TO",
        "DERIVED_FROM",
        "ExternalAuthority",
    ] {
        assert!(
            backend_service.contains(expected),
            "missing complaint graph edge {expected}"
        );
    }

    assert!(
        backend_service.contains("unwrap_or_default()")
            && backend_service.contains("default_complaint_from_matter"),
        "complaint creation should tolerate matters without seeded graph children"
    );
}

#[test]
fn workproduct_ast_canonicalization_contract_is_explicit() {
    let backend_service = casebuilder_service_sources();
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let casebuilder_mod = include_str!("../src/services/casebuilder/mod.rs");
    let work_product_ast = include_str!("../src/services/casebuilder/work_product_ast.rs");
    let ast_validation = include_str!("../src/services/casebuilder/ast_validation.rs");
    let ast_patch = include_str!("../src/services/casebuilder/ast_patch.rs");
    let markdown_adapter = include_str!("../src/services/casebuilder/markdown_adapter.rs");
    let html_renderer = include_str!("../src/services/casebuilder/html_renderer.rs");
    let frontend_types = include_str!("../../../frontend/lib/casebuilder/types.ts");
    let frontend_api = include_str!("../../../frontend/lib/casebuilder/api.ts");

    for expected in [
        "pub draft_id: Option<String>",
        "pub document_type: String",
        "pub tombstones: Vec<WorkProductBlock>",
        "pub sentence_id: Option<String>",
        "pub tombstoned: bool",
        "alias = \"schemaVersion\"",
        "alias = \"matterId\"",
        "alias = \"workProductId\"",
        "alias = \"draftId\"",
        "alias = \"documentType\"",
        "enum NullableStringPatch",
        "Clear,",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend canonical AST model contract {expected}"
        );
    }

    for expected in [
        "mod work_product_ast",
        "mod ast_validation",
        "mod ast_patch",
        "mod ast_diff",
        "mod markdown_adapter",
        "mod html_renderer",
        "mod docx_renderer",
        "mod pdf_renderer",
        "mod rule_engine",
        "mod citation_resolver",
        "mod support_linker",
        "mod ai_patch",
    ] {
        assert!(
            casebuilder_mod.contains(expected),
            "missing dedicated WorkProduct AST service module {expected}"
        );
    }
    assert!(work_product_ast.contains("SUPPORTED_BLOCK_TYPES"));
    assert!(ast_validation.contains("validate_work_product_document"));
    assert!(ast_patch.contains("apply_ast_patch_atomic"));
    assert!(markdown_adapter.contains("wp-ast-block"));
    assert!(html_renderer.contains("data-renderer=\\\"work-product-ast-v1\\\""));
    let backend_ast_sources = [
        backend_service.as_str(),
        work_product_ast,
        ast_validation,
        ast_patch,
        markdown_adapter,
        html_renderer,
    ]
    .join("\n");

    for expected in [
        "\"complaint\"",
        "\"answer\"",
        "\"motion\"",
        "\"declaration\"",
        "\"affidavit\"",
        "\"memo\"",
        "\"notice\"",
        "\"letter\"",
        "\"exhibit_list\"",
        "\"proposed_order\"",
        "\"custom\"",
    ] {
        assert!(
            backend_service.contains(expected) && frontend_types.contains(expected),
            "missing canonical WorkProduct type {expected}"
        );
    }

    for expected in [
        "\"legal_memo\" | \"brief\" => Some(\"memo\")",
        "\"demand_letter\" => Some(\"letter\")",
        "SUPPORTED_WORK_PRODUCT_TYPES",
        "ensure_work_product_ast_valid",
        "validate_optional_text_range",
        "split_ast_document_block",
        "merge_ast_document_blocks",
        "canonical_work_product_blocks(product)",
        "get_work_product_ast",
        "patch_work_product_ast",
    ] {
        assert!(
            backend_ast_sources.contains(expected),
            "missing backend canonical AST behavior {expected}"
        );
    }

    for expected in [
        "draft_id?: string | null",
        "document_type: string",
        "tombstones: WorkProductBlock[]",
        "sentence_id?: string | null",
        "tombstoned: boolean",
        "CANONICAL_WORK_PRODUCT_TYPES",
        "normalizeWorkProductType",
        "input.schemaVersion",
        "input.matterId",
        "input.workProductId",
        "input.draftId",
        "input.documentType",
    ] {
        assert!(
            frontend_types.contains(expected) || frontend_api.contains(expected),
            "missing frontend canonical AST contract {expected}"
        );
    }
}

#[test]
fn workproduct_hybrid_ast_storage_contract_is_bounded_and_object_backed() {
    let backend_service = casebuilder_service_sources();
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let frontend_api = include_str!("../../../frontend/lib/casebuilder/api.ts");
    let object_store = include_str!("../src/services/object_store.rs");

    for expected in [
        "snapshot_full_state_key",
        "snapshot_manifest_key",
        "snapshot_entity_state_key",
        "work_product_export_key",
        "safe_work_product_download_filename",
        "object_blob_id_for_hash",
        "full_state_ref = Some(blob.object_blob_id)",
        "state.state_ref = Some(blob.object_blob_id)",
        "manifest.storage_ref = Some(blob.object_blob_id)",
        "object_blob_id: Some(artifact_blob.object_blob_id.clone())",
        "hydrate_snapshot_full_state",
        "load_json_blob(matter_id, blob_id)",
        "list_work_product_snapshots_for_api",
        "list_work_product_snapshots(matter_id, work_product_id)",
        "summarize_version_snapshot_for_list",
        "version_change_state_summary",
        "validate_ast_patch_matter_references",
        "validate_work_product_matter_references",
        "validate_work_product_link_target",
        "validate_complaint_link_references",
        "diff_work_product_layers",
        "restore_work_product_scope",
        "layer_diffs",
        "\"state_storage\": \"version_snapshot\"",
        "ApiError::Conflict",
    ] {
        assert!(
            backend_service.contains(expected),
            "missing hybrid WorkProduct AST storage behavior {expected}"
        );
    }

    for expected in [
        "base_document_hash",
        "base_snapshot_id",
        "object_blob_id",
        "full_state_ref",
        "state_ref",
        "storage_ref",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend hybrid DTO field {expected}"
        );
    }

    assert!(
        frontend_api.contains("include=document_ast")
            && frontend_api.contains("base_document_hash or base_snapshot_id"),
        "frontend API should expose explicit full-AST list opt-in and AST patch concurrency guard"
    );
    assert!(
        frontend_api.contains("normalizeVersionLayerDiff") && frontend_api.contains("layer_diffs"),
        "frontend API should normalize bounded legal layer diffs"
    );
    assert!(
        object_store.contains("hash_path_segment(document_id.as_bytes(), 24)")
            && !object_store.contains("R2 presign GET failed: {error}"),
        "ObjectStore keys and errors should avoid raw filenames/storage keys"
    );
}

#[test]
fn casebuilder_matter_isolation_contracts_cover_ast_and_object_backed_paths() {
    let backend_service = casebuilder_service_sources();
    let backend_routes = include_str!("../src/routes/casebuilder.rs");

    for expected in [
        "MATCH (:Matter {matter_id: $matter_id})-[:USES_OBJECT_BLOB]->(b:ObjectBlob",
        "product_from_snapshot(matter_id, &from_snapshot)",
        "product_from_snapshot(matter_id, &snapshot)",
        "get_work_product_artifact(matter_id, work_product_id, artifact_id)",
        "validate_ast_patch_matter_references(matter_id, &product, &patch)",
        "validate_work_product_matter_references(matter_id, &product)",
        "validate_work_product_matter_references(matter_id, &restored)",
        "validate_work_product_link_target(matter_id",
        "validate_complaint_link_references(matter_id",
        "require_fact(matter_id",
        "require_evidence(matter_id",
        "require_document(matter_id",
        "require_source_span(matter_id",
        "safe_work_product_download_filename(&artifact)",
    ] {
        assert!(
            backend_service.contains(expected),
            "missing matter-isolation/safe-download behavior {expected}"
        );
    }

    assert!(
        !backend_service.contains("AST patch conflict: patch_id="),
        "CaseBuilder errors should avoid prompt/patch ids and matter ids"
    );
    assert!(
        backend_routes.contains("post(export_work_product)")
            && backend_routes.contains("post(export_matter_docx)")
            && backend_routes.contains("post(export_matter_pdf)")
            && backend_routes.contains("post(export_matter_filing_packet)"),
        "CaseBuilder export routes should use AST-backed export handlers"
    );
}

#[test]
fn casebuilder_provenance_dtos_exist_in_backend_and_frontend() {
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let frontend_types = include_str!("../../../frontend/lib/casebuilder/types.ts");
    let frontend_api = include_str!("../../../frontend/lib/casebuilder/api.ts");

    for expected in [
        "struct ObjectBlob",
        "struct DocumentVersion",
        "struct IngestionRun",
        "struct SourceSpan",
        "struct TranscriptionJob",
        "struct TranscriptSegment",
        "struct TranscriptSpeaker",
        "time_start_ms",
        "speaker_label",
        "struct CaseGraphNode",
        "struct CaseGraphEdge",
        "struct IssueSuggestion",
        "struct QcRun",
        "struct EvidenceGap",
        "struct AuthorityGap",
        "struct Contradiction",
        "struct WorkProductSentence",
        "struct ExportPackage",
        "struct AuditEvent",
        "parser_version",
        "citation_resolver_version",
        "index_version",
    ] {
        assert!(
            backend_models.contains(expected),
            "missing backend DTO {expected}"
        );
    }

    for expected in [
        "interface ObjectBlob",
        "interface DocumentVersion",
        "interface IngestionRun",
        "interface SourceSpan",
        "interface TranscriptionJob",
        "interface TranscriptSegment",
        "interface TranscriptSpeaker",
        "time_start_ms",
        "speaker_label",
        "interface CaseGraphNode",
        "interface CaseGraphEdge",
        "interface IssueSuggestion",
        "interface QcRun",
        "interface EvidenceGap",
        "interface AuthorityGap",
        "interface Contradiction",
        "interface WorkProductSentence",
        "interface ExportPackage",
        "interface AuditEvent",
        "parser_version",
        "citation_resolver_version",
        "index_version",
    ] {
        assert!(
            frontend_types.contains(expected),
            "missing frontend DTO {expected}"
        );
    }

    for expected in [
        "normalizeDocumentVersion",
        "normalizeIngestionRun",
        "normalizeSourceSpan",
        "normalizeTranscriptionJobResponse",
        "normalizeTranscriptSegment",
        "normalizeCaseGraphResponse",
        "normalizeIssueSpotResponse",
        "normalizeQcRun",
        "normalizeExportPackage",
        "normalizeAuditEvent",
        "source_spans",
        "ingestion_run",
        "document_version",
    ] {
        assert!(
            frontend_api.contains(expected),
            "missing frontend provenance normalizer/reference {expected}"
        );
    }
}

#[test]
fn casebuilder_provenance_dtos_serialize_with_matter_safe_ids() {
    let blob = ObjectBlob {
        object_blob_id: "blob:sha256:abcdef".to_string(),
        id: "blob:sha256:abcdef".to_string(),
        sha256: Some("sha256:abcdef".to_string()),
        size_bytes: 42,
        mime_type: Some("text/plain".to_string()),
        storage_provider: "local".to_string(),
        storage_bucket: None,
        storage_key: "casebuilder/documents/doc_opaque/original.txt".to_string(),
        etag: None,
        storage_class: None,
        created_at: "1".to_string(),
        retention_state: "active".to_string(),
    };
    let version = DocumentVersion {
        document_version_id: "version:doc_opaque:original".to_string(),
        id: "version:doc_opaque:original".to_string(),
        matter_id: "matter:test".to_string(),
        document_id: "doc:opaque".to_string(),
        object_blob_id: blob.object_blob_id.clone(),
        role: "original".to_string(),
        artifact_kind: "original_upload".to_string(),
        source_version_id: None,
        created_by: "casebuilder_upload".to_string(),
        current: true,
        created_at: "1".to_string(),
        storage_provider: "local".to_string(),
        storage_bucket: None,
        storage_key: blob.storage_key.clone(),
        sha256: blob.sha256.clone(),
        size_bytes: 42,
        mime_type: Some("text/plain".to_string()),
    };
    let run = IngestionRun {
        ingestion_run_id: "ingestion:doc_opaque:primary".to_string(),
        id: "ingestion:doc_opaque:primary".to_string(),
        matter_id: "matter:test".to_string(),
        document_id: "doc:opaque".to_string(),
        document_version_id: Some(version.document_version_id.clone()),
        object_blob_id: Some(blob.object_blob_id.clone()),
        input_sha256: blob.sha256.clone(),
        status: "stored".to_string(),
        stage: "stored".to_string(),
        mode: "deterministic".to_string(),
        started_at: "1".to_string(),
        completed_at: None,
        error_code: None,
        error_message: None,
        retryable: false,
        produced_node_ids: Vec::new(),
        produced_object_keys: vec![blob.storage_key.clone()],
        parser_id: Some("casebuilder-parser-registry".to_string()),
        parser_version: Some("casebuilder-parser-registry-v1".to_string()),
        chunker_version: Some("casebuilder-line-chunker-v1".to_string()),
        citation_resolver_version: Some("casebuilder-citation-resolver-v1".to_string()),
        index_version: Some("casebuilder-case-graph-index-v1".to_string()),
    };
    let span = SourceSpan {
        source_span_id: "span:doc_opaque:fact:1".to_string(),
        id: "span:doc_opaque:fact:1".to_string(),
        matter_id: "matter:test".to_string(),
        document_id: "doc:opaque".to_string(),
        document_version_id: Some(version.document_version_id.clone()),
        object_blob_id: Some(blob.object_blob_id.clone()),
        ingestion_run_id: Some(run.ingestion_run_id.clone()),
        page: Some(1),
        chunk_id: Some("chunk:doc_opaque:1".to_string()),
        byte_start: Some(0),
        byte_end: Some(10),
        char_start: Some(0),
        char_end: Some(10),
        time_start_ms: None,
        time_end_ms: None,
        speaker_label: None,
        quote: Some("short text".to_string()),
        extraction_method: "deterministic_sentence".to_string(),
        confidence: 1.0,
        review_status: "unreviewed".to_string(),
        unavailable_reason: None,
    };

    let payload = serde_json::json!({
        "blob": blob,
        "version": version,
        "run": run,
        "span": span,
    });
    let text = serde_json::to_string(&payload).expect("provenance DTOs serialize");
    assert!(text.contains("\"matter_id\":\"matter:test\""));
    assert!(!text.contains("Tenant Notice"));
    assert!(!text.contains("../"));
}
