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
        "/matters/:matter_id/facts/:fact_id/approve",
        "/matters/:matter_id/claims/:claim_id/map-elements",
        "/matters/:matter_id/evidence/:evidence_id/link-fact",
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
    ] {
        assert!(
            service.contains(expected),
            "missing CaseBuilder constraint/index {expected}"
        );
    }
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
