use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::ast_validation::validate_work_product_document;
use crate::services::work_product_ast::{
    find_ast_block, find_ast_block_mut, flatten_work_product_blocks, now_string, push_unique,
    validate_optional_text_range,
};

pub(crate) fn apply_ast_patch_atomic(
    product: &WorkProduct,
    patch: &AstPatch,
) -> ApiResult<WorkProductDocument> {
    let mut document = product.document_ast.clone();
    for operation in &patch.operations {
        apply_ast_operation(&mut document, operation)?;
    }
    let mut candidate = product.clone();
    candidate.document_ast = document;
    candidate.blocks = flatten_work_product_blocks(&candidate.document_ast.blocks);
    let validation = validate_work_product_document(&candidate);
    if !validation.errors.is_empty() {
        let codes = validation
            .errors
            .iter()
            .map(|issue| issue.code.clone())
            .collect::<Vec<_>>()
            .join(",");
        return Err(ApiError::BadRequest(format!(
            "AST patch failed validation: issue_codes={codes}"
        )));
    }
    Ok(candidate.document_ast)
}

pub(crate) fn apply_ast_operation(
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
        AstOperation::DeleteBlock {
            block_id,
            tombstone,
        } => {
            let mut deleted = delete_ast_block(&mut document.blocks, block_id)
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
            remove_records_for_deleted_block(document, block_id);
            if *tombstone {
                deleted.tombstoned = true;
                deleted.deleted_at = Some(now_string());
                document
                    .tombstones
                    .retain(|block| block.block_id != deleted.block_id);
                document.tombstones.push(deleted);
            }
            rebuild_document_block_refs(document);
            Ok(())
        }
        AstOperation::MoveBlock {
            block_id,
            parent_id,
            after_block_id,
        } => {
            if parent_id.as_deref() == Some(block_id) {
                return Err(ApiError::BadRequest(
                    "AST block cannot be moved under itself.".to_string(),
                ));
            }
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
    let mut value =
        serde_json::to_value(&*block).map_err(|error| ApiError::Internal(error.to_string()))?;
    merge_json_objects(&mut value, patch);
    let mut updated: WorkProductBlock =
        serde_json::from_value(value).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    if updated.block_id.is_empty() {
        updated.block_id = block.block_id.clone();
    }
    if updated.id.is_empty() {
        updated.id = updated.block_id.clone();
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

fn merge_ast_document_blocks(
    document: &mut WorkProductDocument,
    first_block_id: &str,
    second_block_id: &str,
) -> ApiResult<()> {
    let first = find_ast_block(&document.blocks, first_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {first_block_id} not found")))?;
    let second = find_ast_block(&document.blocks, second_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {second_block_id} not found")))?;
    if first.parent_block_id != second.parent_block_id {
        return Err(ApiError::BadRequest(
            "AST merge requires sibling blocks.".to_string(),
        ));
    }
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

fn remove_records_for_deleted_block(document: &mut WorkProductDocument, block_id: &str) {
    document
        .links
        .retain(|link| link.source_block_id != block_id);
    document
        .citations
        .retain(|citation| citation.source_block_id != block_id);
    document
        .exhibits
        .retain(|exhibit| exhibit.source_block_id != block_id);
    document
        .rule_findings
        .retain(|finding| finding.target_id != block_id);
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
                | "procedural"
                | "background"
                | "argument"
                | "fact"
                | "custom"
        ) {
            block.paragraph_number = Some(*next);
            block
                .provenance
                .get_or_insert_with(Default::default)
                .insert("last_renumbered_at".to_string(), now_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> WorkProductDocument {
        WorkProductDocument {
            schema_version: default_work_product_schema_version(),
            document_id: "wp:test:document".to_string(),
            work_product_id: "wp:test".to_string(),
            matter_id: "matter:test".to_string(),
            document_type: "custom".to_string(),
            product_type: "custom".to_string(),
            title: "Test".to_string(),
            blocks: vec![WorkProductBlock {
                block_id: "b1".to_string(),
                id: "b1".to_string(),
                matter_id: "matter:test".to_string(),
                work_product_id: "wp:test".to_string(),
                block_type: "paragraph".to_string(),
                role: "custom".to_string(),
                title: "Block".to_string(),
                text: "Hello world".to_string(),
                ordinal: 1,
                ..WorkProductBlock::default()
            }],
            ..WorkProductDocument::default()
        }
    }

    #[test]
    fn update_block_merges_in_place() {
        let mut document = document();
        apply_ast_operation(
            &mut document,
            &AstOperation::UpdateBlock {
                block_id: "b1".to_string(),
                before: None,
                after: serde_json::json!({ "text": "Updated" }),
            },
        )
        .expect("patch applies");
        assert_eq!(document.blocks[0].text, "Updated");
    }

    #[test]
    fn tombstoned_delete_preserves_deleted_block() {
        let mut document = document();
        apply_ast_operation(
            &mut document,
            &AstOperation::DeleteBlock {
                block_id: "b1".to_string(),
                tombstone: true,
            },
        )
        .expect("delete applies");
        assert!(document.blocks.is_empty());
        assert_eq!(document.tombstones[0].block_id, "b1");
        assert!(document.tombstones[0].tombstoned);
    }

    #[test]
    fn split_rejects_straddled_range() {
        let mut document = document();
        document.links.push(WorkProductLink {
            link_id: "l1".to_string(),
            source_block_id: "b1".to_string(),
            source_text_range: Some(TextRange {
                start_offset: 0,
                end_offset: 5,
                quote: Some("Hello".to_string()),
            }),
            target_type: "fact".to_string(),
            target_id: "fact:1".to_string(),
            relation: "supports".to_string(),
            confidence: None,
            created_by: "test".to_string(),
            created_at: "1".to_string(),
        });
        let error = apply_ast_operation(
            &mut document,
            &AstOperation::SplitBlock {
                block_id: "b1".to_string(),
                offset: 3,
                new_block_id: "b2".to_string(),
            },
        )
        .expect_err("straddled range rejected");
        assert!(matches!(error, ApiError::BadRequest(_)));
    }
}
