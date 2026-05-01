use super::*;

impl CaseBuilderService {
    pub async fn attach_authority(
        &self,
        matter_id: &str,
        request: AuthorityAttachmentRequest,
    ) -> ApiResult<AuthorityAttachmentResponse> {
        self.require_matter(matter_id).await?;
        let authority = AuthorityRef {
            citation: request.citation,
            canonical_id: request.canonical_id,
            reason: request.reason,
            pinpoint: request.pinpoint,
        };
        match request.target_type.as_str() {
            "claim" => {
                let mut claim = self
                    .get_node::<CaseClaim>(matter_id, claim_spec(), &request.target_id)
                    .await?;
                push_authority(&mut claim.authorities, authority.clone());
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "element" => {
                let mut claim = self
                    .claim_for_element(matter_id, &request.target_id)
                    .await?;
                let element = claim
                    .elements
                    .iter_mut()
                    .find(|element| {
                        element.element_id == request.target_id || element.id == request.target_id
                    })
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Element {} not found", request.target_id))
                    })?;
                push_authority(&mut element.authorities, authority.clone());
                if element.authority.is_none() {
                    element.authority = Some(authority.citation.clone());
                }
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "draft_paragraph" => {
                let mut draft = self
                    .draft_for_paragraph(matter_id, &request.target_id)
                    .await?;
                let paragraph = draft
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == request.target_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!(
                            "Draft paragraph {} not found",
                            request.target_id
                        ))
                    })?;
                push_authority(&mut paragraph.authorities, authority.clone());
                let draft = self
                    .merge_node(matter_id, draft_spec(), &draft.draft_id, &draft)
                    .await?;
                self.materialize_draft_edges(&draft).await?;
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported authority target_type {value}"
                )));
            }
        }

        Ok(AuthorityAttachmentResponse {
            matter_id: matter_id.to_string(),
            target_type: request.target_type,
            target_id: request.target_id,
            authority,
            attached: true,
        })
    }

    pub async fn detach_authority(
        &self,
        matter_id: &str,
        request: AuthorityAttachmentRequest,
    ) -> ApiResult<AuthorityAttachmentResponse> {
        self.require_matter(matter_id).await?;
        let authority = AuthorityRef {
            citation: request.citation,
            canonical_id: request.canonical_id,
            reason: request.reason,
            pinpoint: request.pinpoint,
        };
        match request.target_type.as_str() {
            "claim" => {
                let mut claim = self
                    .get_node::<CaseClaim>(matter_id, claim_spec(), &request.target_id)
                    .await?;
                remove_authority(&mut claim.authorities, &authority);
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.detach_authority_edge("Claim", "claim_id", &claim.claim_id, &authority)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "element" => {
                let mut claim = self
                    .claim_for_element(matter_id, &request.target_id)
                    .await?;
                let element = claim
                    .elements
                    .iter_mut()
                    .find(|element| {
                        element.element_id == request.target_id || element.id == request.target_id
                    })
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Element {} not found", request.target_id))
                    })?;
                remove_authority(&mut element.authorities, &authority);
                if element.authority.as_deref() == Some(authority.citation.as_str()) {
                    element.authority = element
                        .authorities
                        .first()
                        .map(|item| item.citation.clone());
                }
                let element_id = element.element_id.clone();
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.detach_authority_edge("Element", "element_id", &element_id, &authority)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "draft_paragraph" => {
                let mut draft = self
                    .draft_for_paragraph(matter_id, &request.target_id)
                    .await?;
                if let Some(paragraph) = draft
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == request.target_id)
                {
                    remove_authority(&mut paragraph.authorities, &authority);
                }
                let draft = self
                    .merge_node(matter_id, draft_spec(), &draft.draft_id, &draft)
                    .await?;
                self.detach_authority_edge(
                    "DraftParagraph",
                    "paragraph_id",
                    &request.target_id,
                    &authority,
                )
                .await?;
                self.materialize_draft_edges(&draft).await?;
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported authority target_type {value}"
                )));
            }
        }

        Ok(AuthorityAttachmentResponse {
            matter_id: matter_id.to_string(),
            target_type: request.target_type,
            target_id: request.target_id,
            authority,
            attached: false,
        })
    }

    pub(super) async fn list_fact_check_findings(
        &self,
        matter_id: &str,
        draft_id: Option<&str>,
    ) -> ApiResult<Vec<FactCheckFinding>> {
        let mut findings: Vec<FactCheckFinding> = self
            .list_nodes(matter_id, fact_check_finding_spec())
            .await?;
        if let Some(draft_id) = draft_id {
            findings.retain(|finding| finding.draft_id == draft_id);
        }
        Ok(findings)
    }

    pub(super) async fn list_citation_check_findings(
        &self,
        matter_id: &str,
        draft_id: Option<&str>,
    ) -> ApiResult<Vec<CitationCheckFinding>> {
        let mut findings: Vec<CitationCheckFinding> = self
            .list_nodes(matter_id, citation_check_finding_spec())
            .await?;
        if let Some(draft_id) = draft_id {
            findings.retain(|finding| finding.draft_id == draft_id);
        }
        Ok(findings)
    }

    pub(super) async fn claim_for_element(
        &self,
        matter_id: &str,
        element_id: &str,
    ) -> ApiResult<CaseClaim> {
        self.list_claims(matter_id)
            .await?
            .into_iter()
            .find(|claim| {
                claim
                    .elements
                    .iter()
                    .any(|element| element.element_id == element_id || element.id == element_id)
            })
            .ok_or_else(|| ApiError::NotFound(format!("Element {element_id} not found")))
    }

    pub(super) async fn draft_for_paragraph(
        &self,
        matter_id: &str,
        paragraph_id: &str,
    ) -> ApiResult<CaseDraft> {
        self.list_drafts(matter_id)
            .await?
            .into_iter()
            .find(|draft| {
                draft
                    .paragraphs
                    .iter()
                    .any(|paragraph| paragraph.paragraph_id == paragraph_id)
            })
            .ok_or_else(|| ApiError::NotFound(format!("Draft paragraph {paragraph_id} not found")))
    }
}
