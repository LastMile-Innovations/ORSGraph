use crate::models::casebuilder::*;
use crate::services::work_product_ast::now_string;

pub(crate) fn empty_provider_free_ai_patch(
    product: &WorkProduct,
    command: &str,
    created_by: &str,
) -> AstPatch {
    AstPatch {
        patch_id: format!(
            "{}:ai-patch:{}:{}",
            product.work_product_id, command, product.document_ast.updated_at
        ),
        draft_id: product.document_ast.draft_id.clone(),
        work_product_id: Some(product.work_product_id.clone()),
        base_document_hash: None,
        base_snapshot_id: None,
        created_by: created_by.to_string(),
        reason: Some(format!(
            "Provider-free AI command {command} recorded; no text changes proposed."
        )),
        operations: Vec::new(),
        created_at: now_string(),
    }
}
