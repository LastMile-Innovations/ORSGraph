use orsgraph_api::models::casebuilder::{DocumentVersion, IngestionRun, ObjectBlob, SourceSpan};

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
fn casebuilder_routes_cover_v0_contracts() {
    let routes = include_str!("../src/routes/casebuilder.rs");

    for expected in [
        "/matters",
        "/matters/:matter_id",
        "/matters/:matter_id/files",
        "/matters/:matter_id/files/binary",
        "/matters/:matter_id/files/uploads",
        "/matters/:matter_id/files/uploads/:upload_id/complete",
        "/matters/:matter_id/documents/:document_id/download-url",
        "/matters/:matter_id/documents/:document_id/extract",
        "/matters/:matter_id/documents/:document_id/import-complaint",
        "/matters/:matter_id/facts/:fact_id/approve",
        "/matters/:matter_id/claims/:claim_id/map-elements",
        "/matters/:matter_id/evidence/:evidence_id/link-fact",
        "/matters/:matter_id/work-products",
        "/matters/:matter_id/work-products/:work_product_id",
        "/matters/:matter_id/work-products/:work_product_id/blocks",
        "/matters/:matter_id/work-products/:work_product_id/blocks/:block_id",
        "/matters/:matter_id/work-products/:work_product_id/links",
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
    let service = include_str!("../src/services/casebuilder.rs");

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
fn complaint_editor_dtos_and_api_exist_in_backend_and_frontend() {
    let backend_models = include_str!("../src/models/casebuilder.rs");
    let backend_service = include_str!("../src/services/casebuilder.rs");
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
fn workproduct_hybrid_ast_storage_contract_is_bounded_and_object_backed() {
    let backend_service = include_str!("../src/services/casebuilder.rs");
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
    let backend_service = include_str!("../src/services/casebuilder.rs");
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
        !backend_service.contains("AST patch conflict: patch_id=")
            && backend_routes.contains(
                "Export is deferred for CaseBuilder V0; DOCX/PDF/filing packets are V0.2+."
            ),
        "CaseBuilder errors should avoid prompt/patch ids and matter ids"
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
