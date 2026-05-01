use crate::artifact_store::ArtifactMetadata;
use crate::graph_batch::GraphBatch;
use crate::source_registry::{SourceKind, SourceRegistryEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QcReportStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcReport {
    pub source_id: String,
    pub status: QcReportStatus,
    pub artifacts: usize,
    pub graph_files: usize,
    pub graph_rows: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl QcReport {
    pub fn is_failure(&self) -> bool {
        self.status == QcReportStatus::Fail
    }
}

pub fn qc_source_batch(
    entry: &SourceRegistryEntry,
    artifacts: &[ArtifactMetadata],
    batch: &GraphBatch,
) -> QcReport {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if artifacts.is_empty() {
        errors.push("no raw artifacts were preserved".to_string());
    }
    if batch.is_empty() {
        errors.push("no graph rows were emitted".to_string());
    }
    for artifact in artifacts {
        if artifact.raw_hash.trim().is_empty() {
            errors.push(format!("{} is missing raw_hash", artifact.item_id));
        }
        if artifact.path.trim().is_empty() {
            errors.push(format!("{} is missing raw artifact path", artifact.item_id));
        }
    }
    if entry.robots_acceptable_use == "needs_review" {
        warnings.push("robots/acceptable-use policy needs review".to_string());
        if entry.source_type == SourceKind::SearchPage {
            errors.push(
                "search-page connector cannot broad-crawl without explicit acceptable-use policy"
                    .to_string(),
            );
        }
    }

    let status = if !errors.is_empty() {
        QcReportStatus::Fail
    } else if !warnings.is_empty() {
        QcReportStatus::Warning
    } else {
        QcReportStatus::Pass
    };

    QcReport {
        source_id: entry.source_id.clone(),
        status,
        artifacts: artifacts.len(),
        graph_files: batch.files.len(),
        graph_rows: batch.row_count(),
        warnings,
        errors,
    }
}
