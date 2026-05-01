// ── Constraints ────────────────────────────────────────────────────────────────────────

CREATE CONSTRAINT jurisdiction_id IF NOT EXISTS FOR (n:Jurisdiction) REQUIRE n.jurisdiction_id IS UNIQUE;
CREATE CONSTRAINT public_body_id IF NOT EXISTS FOR (n:PublicBody) REQUIRE n.public_body_id IS UNIQUE;
CREATE CONSTRAINT corpus_id IF NOT EXISTS FOR (n:LegalCorpus) REQUIRE n.corpus_id IS UNIQUE;
CREATE CONSTRAINT edition_id IF NOT EXISTS FOR (n:CorpusEdition) REQUIRE n.edition_id IS UNIQUE;
CREATE CONSTRAINT chapter_id IF NOT EXISTS FOR (n:ChapterVersion) REQUIRE n.chapter_id IS UNIQUE;
CREATE CONSTRAINT heading_id IF NOT EXISTS FOR (n:ChapterHeading) REQUIRE n.heading_id IS UNIQUE;
CREATE CONSTRAINT source_document_id IF NOT EXISTS FOR (n:SourceDocument) REQUIRE n.source_document_id IS UNIQUE;
CREATE CONSTRAINT source_page_id IF NOT EXISTS FOR (n:SourcePage) REQUIRE n.source_page_id IS UNIQUE;
CREATE CONSTRAINT source_toc_entry_id IF NOT EXISTS FOR (n:SourceTocEntry) REQUIRE n.source_toc_entry_id IS UNIQUE;
CREATE CONSTRAINT html_paragraph_id IF NOT EXISTS FOR (n:HtmlParagraph) REQUIRE n.paragraph_id IS UNIQUE;
CREATE CONSTRAINT chapter_front_matter_id IF NOT EXISTS FOR (n:ChapterFrontMatter) REQUIRE n.front_matter_id IS UNIQUE;
CREATE CONSTRAINT title_chapter_entry_id IF NOT EXISTS FOR (n:TitleChapterEntry) REQUIRE n.title_chapter_entry_id IS UNIQUE;
CREATE CONSTRAINT legal_identity_id IF NOT EXISTS FOR (n:LegalTextIdentity) REQUIRE n.canonical_id IS UNIQUE;
CREATE CONSTRAINT legal_version_id IF NOT EXISTS FOR (n:LegalTextVersion) REQUIRE n.version_id IS UNIQUE;
CREATE CONSTRAINT provision_id IF NOT EXISTS FOR (n:Provision) REQUIRE n.provision_id IS UNIQUE;
CREATE CONSTRAINT citation_mention_id IF NOT EXISTS FOR (n:CitationMention) REQUIRE n.citation_mention_id IS UNIQUE;
CREATE CONSTRAINT retrieval_chunk_id IF NOT EXISTS FOR (n:RetrievalChunk) REQUIRE n.chunk_id IS UNIQUE;
CREATE CONSTRAINT source_note_id IF NOT EXISTS FOR (n:SourceNote) REQUIRE n.source_note_id IS UNIQUE;
CREATE CONSTRAINT chapter_toc_entry_id IF NOT EXISTS FOR (n:ChapterTocEntry) REQUIRE n.toc_entry_id IS UNIQUE;
CREATE CONSTRAINT reserved_range_id IF NOT EXISTS FOR (n:ReservedRange) REQUIRE n.reserved_range_id IS UNIQUE;
CREATE CONSTRAINT parser_diagnostic_id IF NOT EXISTS FOR (n:ParserDiagnostic) REQUIRE n.parser_diagnostic_id IS UNIQUE;
CREATE CONSTRAINT status_event_id IF NOT EXISTS FOR (n:StatusEvent) REQUIRE n.status_event_id IS UNIQUE;
CREATE CONSTRAINT temporal_effect_id IF NOT EXISTS FOR (n:TemporalEffect) REQUIRE n.temporal_effect_id IS UNIQUE;
CREATE CONSTRAINT lineage_event_id IF NOT EXISTS FOR (n:LineageEvent) REQUIRE n.lineage_event_id IS UNIQUE;
CREATE CONSTRAINT amendment_id IF NOT EXISTS FOR (n:Amendment) REQUIRE n.amendment_id IS UNIQUE;
CREATE CONSTRAINT session_law_id IF NOT EXISTS FOR (n:SessionLaw) REQUIRE n.session_law_id IS UNIQUE;
CREATE CONSTRAINT time_interval_id IF NOT EXISTS FOR (n:TimeInterval) REQUIRE n.time_interval_id IS UNIQUE;
CREATE CONSTRAINT defined_term_id IF NOT EXISTS FOR (n:DefinedTerm) REQUIRE n.defined_term_id IS UNIQUE;
CREATE CONSTRAINT definition_id IF NOT EXISTS FOR (n:Definition) REQUIRE n.definition_id IS UNIQUE;
CREATE CONSTRAINT definition_scope_id IF NOT EXISTS FOR (n:DefinitionScope) REQUIRE n.definition_scope_id IS UNIQUE;
CREATE CONSTRAINT semantic_id IF NOT EXISTS FOR (n:LegalSemanticNode) REQUIRE n.semantic_id IS UNIQUE;
CREATE CONSTRAINT obligation_id IF NOT EXISTS FOR (n:Obligation) REQUIRE n.obligation_id IS UNIQUE;
CREATE CONSTRAINT exception_id IF NOT EXISTS FOR (n:Exception) REQUIRE n.exception_id IS UNIQUE;
CREATE CONSTRAINT deadline_id IF NOT EXISTS FOR (n:Deadline) REQUIRE n.deadline_id IS UNIQUE;
CREATE CONSTRAINT penalty_id IF NOT EXISTS FOR (n:Penalty) REQUIRE n.penalty_id IS UNIQUE;
CREATE CONSTRAINT remedy_id IF NOT EXISTS FOR (n:Remedy) REQUIRE n.remedy_id IS UNIQUE;
CREATE CONSTRAINT legal_actor_id IF NOT EXISTS FOR (n:LegalActor) REQUIRE n.actor_id IS UNIQUE;
CREATE CONSTRAINT legal_action_id IF NOT EXISTS FOR (n:LegalAction) REQUIRE n.action_id IS UNIQUE;
CREATE CONSTRAINT money_amount_id IF NOT EXISTS FOR (n:MoneyAmount) REQUIRE n.money_amount_id IS UNIQUE;
CREATE CONSTRAINT tax_rule_id IF NOT EXISTS FOR (n:TaxRule) REQUIRE n.tax_rule_id IS UNIQUE;
CREATE CONSTRAINT rate_limit_id IF NOT EXISTS FOR (n:RateLimit) REQUIRE n.rate_limit_id IS UNIQUE;
CREATE CONSTRAINT required_notice_id IF NOT EXISTS FOR (n:RequiredNotice) REQUIRE n.required_notice_id IS UNIQUE;
CREATE CONSTRAINT form_text_id IF NOT EXISTS FOR (n:FormText) REQUIRE n.form_text_id IS UNIQUE;
CREATE CONSTRAINT external_legal_citation_id IF NOT EXISTS FOR (n:ExternalLegalCitation) REQUIRE n.external_citation_id IS UNIQUE;
CREATE CONSTRAINT reporter_note_id IF NOT EXISTS FOR (n:ReporterNote) REQUIRE n.reporter_note_id IS UNIQUE;
CREATE CONSTRAINT commentary_id IF NOT EXISTS FOR (n:Commentary) REQUIRE n.commentary_id IS UNIQUE;
CREATE CONSTRAINT procedural_requirement_id IF NOT EXISTS FOR (n:ProceduralRequirement) REQUIRE n.requirement_id IS UNIQUE;
CREATE CONSTRAINT work_product_rule_pack_id IF NOT EXISTS FOR (n:WorkProductRulePack) REQUIRE n.rule_pack_id IS UNIQUE;
CREATE CONSTRAINT formatting_profile_id IF NOT EXISTS FOR (n:FormattingProfile) REQUIRE n.formatting_profile_id IS UNIQUE;
CREATE CONSTRAINT rule_pack_membership_id IF NOT EXISTS FOR (n:RulePackMembership) REQUIRE n.membership_id IS UNIQUE;
CREATE CONSTRAINT court_id IF NOT EXISTS FOR (n:Court) REQUIRE n.court_id IS UNIQUE;
CREATE CONSTRAINT court_rules_registry_source_id IF NOT EXISTS FOR (n:CourtRulesRegistrySource) REQUIRE n.registry_source_id IS UNIQUE;
CREATE CONSTRAINT court_rules_registry_snapshot_id IF NOT EXISTS FOR (n:CourtRulesRegistrySnapshot) REQUIRE n.registry_snapshot_id IS UNIQUE;
CREATE CONSTRAINT rule_publication_entry_id IF NOT EXISTS FOR (n:RulePublicationEntry) REQUIRE n.publication_entry_id IS UNIQUE;
CREATE CONSTRAINT rule_authority_document_id IF NOT EXISTS FOR (n:RuleAuthorityDocument) REQUIRE n.authority_document_id IS UNIQUE;
CREATE CONSTRAINT effective_interval_id IF NOT EXISTS FOR (n:EffectiveInterval) REQUIRE n.effective_interval_id IS UNIQUE;
CREATE CONSTRAINT rule_topic_id IF NOT EXISTS FOR (n:RuleTopic) REQUIRE n.rule_topic_id IS UNIQUE;
CREATE CONSTRAINT work_product_rule_pack_authority_id IF NOT EXISTS FOR (n:WorkProductRulePackAuthority) REQUIRE n.rule_pack_authority_id IS UNIQUE;

// Edge constraints for re-seed idempotency
CREATE CONSTRAINT cites_edge_id IF NOT EXISTS FOR ()-[r:CITES]-() REQUIRE r.edge_id IS UNIQUE;
CREATE CONSTRAINT cites_version_edge_id IF NOT EXISTS FOR ()-[r:CITES_VERSION]-() REQUIRE r.edge_id IS UNIQUE;
CREATE CONSTRAINT cites_provision_edge_id IF NOT EXISTS FOR ()-[r:CITES_PROVISION]-() REQUIRE r.edge_id IS UNIQUE;
CREATE CONSTRAINT cites_chapter_edge_id IF NOT EXISTS FOR ()-[r:CITES_CHAPTER]-() REQUIRE r.edge_id IS UNIQUE;
CREATE CONSTRAINT cites_range_edge_id IF NOT EXISTS FOR ()-[r:CITES_RANGE]-() REQUIRE r.edge_id IS UNIQUE;
CREATE CONSTRAINT retrieval_chunk_vector_type IF NOT EXISTS FOR (n:RetrievalChunk) REQUIRE n.embedding IS :: VECTOR<FLOAT32>({DIMENSION});
CREATE CONSTRAINT provision_vector_type IF NOT EXISTS FOR (p:Provision) REQUIRE p.embedding IS :: VECTOR<FLOAT32>({DIMENSION});

// ── Indexes ───────────────────────────────────────────────────────────────────────────

CREATE INDEX legalIdentityCitation IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.citation);
CREATE INDEX legalIdentityStatus IF NOT EXISTS FOR (n:LegalTextIdentity) ON (n.status, n.authority_family);
CREATE INDEX legalVersionLookup IF NOT EXISTS FOR (n:LegalTextVersion) ON (n.canonical_id, n.edition_year, n.current);
CREATE INDEX provisionLookup IF NOT EXISTS FOR (n:Provision) ON (n.version_id, n.display_citation, n.order_index);
CREATE INDEX provisionPathLookup IF NOT EXISTS FOR (n:Provision) ON (n.version_id, n.local_path);
CREATE INDEX provisionCanonicalLookup IF NOT EXISTS FOR (n:Provision) ON (n.canonical_id);
CREATE INDEX provisionSignals IF NOT EXISTS FOR (n:Provision) ON (n.is_definition_candidate, n.is_exception_candidate, n.is_deadline_candidate, n.is_penalty_candidate);
CREATE INDEX chunkPolicy IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.embedding_policy, n.answer_policy, n.chunk_type);
CREATE INDEX chunkEmbeddingStatus IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.embedding_policy, n.embedding_input_hash);
CREATE INDEX chunkEmbeddingModel IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.embedding_model, n.embedding_dim);
CREATE INDEX chunkSourceLookup IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.source_kind, n.source_id);
CREATE INDEX chunkProvisionSourceLookup IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.source_provision_id, n.parent_version_id);
CREATE INDEX chunkVersionSourceLookup IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.source_version_id);
CREATE INDEX retrieval_chunk_token_count IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.token_count);
CREATE INDEX retrieval_chunk_version_strategy IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.chunk_version, n.chunking_strategy);
CREATE INDEX retrieval_chunk_type_tokens IF NOT EXISTS FOR (n:RetrievalChunk) ON (n.chunk_type, n.token_count);
CREATE INDEX citationStatus IF NOT EXISTS FOR (n:CitationMention) ON (n.resolver_status, n.citation_type);
CREATE INDEX citationSourceLookup IF NOT EXISTS FOR (n:CitationMention) ON (n.source_provision_id);
CREATE INDEX sourceNoteLookup IF NOT EXISTS FOR (n:SourceNote) ON (n.canonical_id, n.note_type);
CREATE INDEX chapterTocLookup IF NOT EXISTS FOR (n:ChapterTocEntry) ON (n.chapter, n.edition_year, n.canonical_id);
CREATE INDEX temporalEffectLookup IF NOT EXISTS FOR (n:TemporalEffect) ON (n.canonical_id, n.effect_type);
CREATE INDEX lineageEventLookup IF NOT EXISTS FOR (n:LineageEvent) ON (n.current_canonical_id, n.lineage_type);
CREATE INDEX chapterLookup IF NOT EXISTS FOR (n:ChapterVersion) ON (n.chapter);
CREATE INDEX chapterVersionLookup IF NOT EXISTS FOR (n:ChapterVersion) ON (n.chapter, n.edition_year);
CREATE INDEX sourceDocumentLookup IF NOT EXISTS FOR (n:SourceDocument) ON (n.chapter, n.edition_year);
CREATE INDEX sourceDocumentCorpusLookup IF NOT EXISTS FOR (n:SourceDocument) ON (n.corpus_id, n.edition_id, n.authority_family);
CREATE INDEX sourcePageLookup IF NOT EXISTS FOR (n:SourcePage) ON (n.source_document_id, n.page_number);
CREATE INDEX sourceTocLookup IF NOT EXISTS FOR (n:SourceTocEntry) ON (n.source_document_id, n.entry_type, n.citation);
CREATE INDEX htmlParagraphSourceLookup IF NOT EXISTS FOR (n:HtmlParagraph) ON (n.source_document_id, n.order_index);
CREATE INDEX frontMatterSourceLookup IF NOT EXISTS FOR (n:ChapterFrontMatter) ON (n.source_document_id, n.source_paragraph_order);
CREATE INDEX titleChapterEntryLookup IF NOT EXISTS FOR (n:TitleChapterEntry) ON (n.title_number, n.chapter_number);
CREATE INDEX statusEventLookup IF NOT EXISTS FOR (n:StatusEvent) ON (n.canonical_id, n.status_type);
CREATE INDEX amendmentLookup IF NOT EXISTS FOR (n:Amendment) ON (n.canonical_id, n.amendment_type);
CREATE INDEX definitionTermLookup IF NOT EXISTS FOR (n:Definition) ON (n.normalized_term, n.source_provision_id);
CREATE INDEX semanticSourceLookup IF NOT EXISTS FOR (n:LegalSemanticNode) ON (n.semantic_type, n.source_provision_id);
CREATE INDEX moneyAmountSourceLookup IF NOT EXISTS FOR (n:MoneyAmount) ON (n.source_provision_id, n.amount_type);
CREATE INDEX taxRuleSourceLookup IF NOT EXISTS FOR (n:TaxRule) ON (n.source_provision_id, n.tax_type);
CREATE INDEX rateLimitSourceLookup IF NOT EXISTS FOR (n:RateLimit) ON (n.source_provision_id, n.rate_type);
CREATE INDEX requiredNoticeSourceLookup IF NOT EXISTS FOR (n:RequiredNotice) ON (n.source_provision_id, n.notice_type);
CREATE INDEX formTextSourceLookup IF NOT EXISTS FOR (n:FormText) ON (n.source_provision_id, n.form_type);
CREATE INDEX proceduralRequirementSourceLookup IF NOT EXISTS FOR (n:ProceduralRequirement) ON (n.semantic_type, n.source_provision_id);
CREATE INDEX proceduralRequirementTypeLookup IF NOT EXISTS FOR (n:ProceduralRequirement) ON (n.requirement_type, n.severity_default);
CREATE INDEX rulePackMembershipLookup IF NOT EXISTS FOR (n:RulePackMembership) ON (n.rule_pack_id, n.requirement_id);
CREATE INDEX ruleAuthorityApplicabilityLookup IF NOT EXISTS FOR (n:RuleAuthorityDocument) ON (n.jurisdiction_id, n.effective_start_date, n.effective_end_date);
CREATE INDEX ruleAuthorityKindLookup IF NOT EXISTS FOR (n:RuleAuthorityDocument) ON (n.authority_kind, n.date_status);
CREATE INDEX rulePublicationEntryLookup IF NOT EXISTS FOR (n:RulePublicationEntry) ON (n.jurisdiction_id, n.publication_bucket);
CREATE INDEX courtRulesRegistrySnapshotLookup IF NOT EXISTS FOR (n:CourtRulesRegistrySnapshot) ON (n.jurisdiction_id, n.snapshot_date);

// ── Fulltext Index ───────────────────────────────────────────────────────────────────

CREATE FULLTEXT INDEX legalTextFulltext IF NOT EXISTS
FOR (n:LegalTextVersion|Provision|RetrievalChunk)
ON EACH [n.citation, n.display_citation, n.title, n.text, n.breadcrumb];

// ── Vector Index ─────────────────────────────────────────────────────────────────────

// Vector index for semantic search on embeddings (Cypher 25 syntax).
// Compatible with both Community and Enterprise Edition.
// In Community Edition, store embeddings as LIST<FLOAT> properties.
// In Enterprise Edition, embeddings can be stored as VECTOR type.
// Note: These indexes are created conditionally by the Rust code with the correct dimension parameter.
// The dimension placeholder {DIMENSION} is replaced at runtime.
CREATE VECTOR INDEX retrieval_chunk_embedding_1024 IF NOT EXISTS
FOR (n:RetrievalChunk)
ON n.embedding
WITH [n.citation, n.chunk_type, n.answer_policy, n.edition_year, n.authority_level, n.is_definition_candidate, n.is_exception_candidate]
OPTIONS {indexConfig: {`vector.dimensions`: {DIMENSION}, `vector.similarity_function`: 'cosine'}};

// Example query using modern SEARCH syntax with metadata filtering and similarity scores:
// MATCH (n:RetrievalChunk)
//   SEARCH n IN (
//     VECTOR INDEX retrieval_chunk_embedding_1024
//     FOR $embedding
//     WHERE n.citation IS NOT NULL AND n.answer_policy = 'answerable'
//     LIMIT 10
//   ) SCORE AS similarityScore
// RETURN n.chunk_id, n.text, n.citation, n.answer_policy, similarityScore
// ORDER BY similarityScore DESC
