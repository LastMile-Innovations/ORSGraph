use crate::embeddings::EMBEDDING_TARGETS;
use crate::models::QcStatus;
use anyhow::Result;
use neo4rs::{ConfigBuilder, Graph, query};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct QcNeo4jValidator {
    graph: Arc<Graph>,
    require_embeddings: bool,
    embedding_profile: String,
    embedding_model: String,
    embedding_dim: i32,
    embedding_dtype: String,
    graph_dir: Option<PathBuf>,
}

impl QcNeo4jValidator {
    pub async fn new(
        uri: &str,
        user: &str,
        pass: &str,
        require_embeddings: bool,
        embedding_profile: String,
        embedding_model: String,
        embedding_dim: i32,
        embedding_dtype: String,
        graph_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(pass)
            .build()?;
        let graph = Arc::new(Graph::connect(config).await?);
        Ok(Self {
            graph,
            require_embeddings,
            embedding_profile,
            embedding_model,
            embedding_dim,
            embedding_dtype,
            graph_dir,
        })
    }

    pub async fn run(&self) -> Result<QcNeo4jReport> {
        info!("Running Neo4j QC validation...");

        let mut report = QcNeo4jReport::default();
        report.model = self.embedding_model.clone();
        report.dimension = self.embedding_dim as usize;
        report.status = QcStatus::Pass;
        report.expected = self
            .graph_dir
            .as_deref()
            .map(load_expected_counts)
            .transpose()?;

        // 1. Eligible chunks count
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy IN ['embed_primary', 'embed_special']
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.eligible_chunks = row.get::<i64>("count")? as usize;
        }

        // 2. Embedded chunks count
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.embedded_chunks = row.get::<i64>("count")? as usize;
        }

        // 3. Missing primary embeddings
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy = 'embed_primary' AND c.embedding IS NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.missing_primary = row.get::<i64>("count")? as usize;
        }

        // 4. Missing special embeddings
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy = 'embed_special' AND c.embedding IS NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.missing_special = row.get::<i64>("count")? as usize;
        }

        // 5. Dimension mismatches
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND size(c.embedding) <> $dim
                 RETURN count(c) AS count";
        let mut res = self
            .graph
            .execute(query(q).param("dim", self.embedding_dim as i64))
            .await?;
        if let Some(row) = res.next().await? {
            report.dimension_mismatches = row.get::<i64>("count")? as usize;
        }

        // 6. Model mismatches
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_model <> $model
                 RETURN count(c) AS count";
        let mut res = self
            .graph
            .execute(query(q).param("model", self.embedding_model.clone()))
            .await?;
        if let Some(row) = res.next().await? {
            report.model_mismatches = row.get::<i64>("count")? as usize;
        }

        // 7. Missing input hash
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_input_hash IS NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.missing_input_hash = row.get::<i64>("count")? as usize;
        }

        // 8. Input type mismatch
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_input_type <> 'document'
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.input_type_mismatches = row.get::<i64>("count")? as usize;
        }

        // 9. Output dtype mismatch
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_output_dtype <> $dtype
                 RETURN count(c) AS count";
        let mut res = self
            .graph
            .execute(query(q).param("dtype", self.embedding_dtype.clone()))
            .await?;
        if let Some(row) = res.next().await? {
            report.output_dtype_mismatches = row.get::<i64>("count")? as usize;
        }

        // 9a. Missing embedding profile
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_profile IS NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.missing_embedding_profile = row.get::<i64>("count")? as usize;
        }

        // 9b. Embedding profile mismatch
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_profile <> $profile
                 RETURN count(c) AS count";
        let mut res = self
            .graph
            .execute(query(q).param("profile", self.embedding_profile.clone()))
            .await?;
        if let Some(row) = res.next().await? {
            report.embedding_profile_mismatches = row.get::<i64>("count")? as usize;
        }

        // 9c. Missing embedding source dimension
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_source_dimension IS NULL
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.missing_source_dimension = row.get::<i64>("count")? as usize;
        }

        // 9d. Embedding source dimension mismatch
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding IS NOT NULL AND c.embedding_source_dimension <> $dim
                 RETURN count(c) AS count";
        let mut res = self
            .graph
            .execute(query(q).param("dim", self.embedding_dim))
            .await?;
        if let Some(row) = res.next().await? {
            report.source_dimension_mismatches = row.get::<i64>("count")? as usize;
        }

        // 9b. V3 chunk audit metadata
        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy IN ['embed_primary', 'embed_special']
                   AND (c.token_count IS NULL OR c.max_tokens IS NULL OR c.context_window IS NULL
                        OR c.chunking_strategy IS NULL OR c.chunk_version IS NULL
                        OR c.embedding_input_hash IS NULL)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.chunks_missing_audit_metadata = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy IN ['embed_primary', 'embed_special']
                   AND c.token_count > 30000
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.chunks_over_hard_token_limit = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.embedding_policy IN ['embed_primary', 'embed_special']
                   AND (c.part_count IS NULL OR c.part_index IS NULL
                        OR c.part_count < 1 OR c.part_index < 1 OR c.part_index > c.part_count)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.chunks_with_invalid_part_metadata = row.get::<i64>("count")? as usize;
        }

        // 10. Vector index checks
        let q = "SHOW INDEXES YIELD name, type, labelsOrTypes, properties
                 WHERE type = 'VECTOR' AND name = 'retrieval_chunk_embedding_1024'
                 RETURN count(*) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.vector_index_exists = row.get::<i64>("count")? > 0;
        }

        let q = "SHOW INDEXES YIELD name, type, labelsOrTypes, properties
                 WHERE type = 'VECTOR' AND name = 'provision_embedding_1024'
                 RETURN count(*) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_vector_index_exists = row.get::<i64>("count")? > 0;
        }

        let q = "SHOW INDEXES YIELD name, type, labelsOrTypes, properties
                 WHERE type = 'VECTOR' AND name = 'legal_text_version_embedding_1024'
                 RETURN count(*) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.version_vector_index_exists = row.get::<i64>("count")? > 0;
        }

        // 10b. Provision/Version embedding counts
        let q = "MATCH (p:Provision) WHERE (p.text IS NOT NULL AND p.text <> '') RETURN count(p) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.eligible_provisions = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (p:Provision) WHERE p.embedding IS NOT NULL RETURN count(p) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.embedded_provisions = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (v:LegalTextVersion) WHERE (v.text IS NOT NULL AND v.text <> '') RETURN count(v) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.eligible_versions = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (v:LegalTextVersion) WHERE v.embedding IS NOT NULL RETURN count(v) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.embedded_versions = row.get::<i64>("count")? as usize;
        }

        report.embedding_coverage = self.collect_embedding_coverage().await?;
        for coverage in &report.embedding_coverage {
            report.dimension_mismatches += coverage.dimension_mismatches;
            report.model_mismatches += coverage.model_mismatches;
            report.output_dtype_mismatches += coverage.output_dtype_mismatches;
            report.missing_input_hash += coverage.missing_input_hash;
            report.missing_embedding_profile += coverage.missing_embedding_profile;
            report.embedding_profile_mismatches += coverage.profile_mismatches;
            report.missing_source_dimension += coverage.missing_source_dimension;
            report.source_dimension_mismatches += coverage.source_dimension_mismatches;
            report.missing_embedded_at += coverage.missing_embedded_at;
            report.mixed_dimensions += coverage.mixed_dimensions;
        }

        // 11. Topology validation - Node counts by label
        info!("Validating graph topology...");

        let q = "MATCH (n:LegalTextIdentity) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.identity_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:LegalTextVersion) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.version_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:Provision) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:CitationMention) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.citation_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:SourceDocument) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.source_doc_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:ChapterHeading) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.heading_count = row.get::<i64>("count")? as usize;
        }

        // 12. Relationship counts by type
        let q = "MATCH ()-[r:HAS_VERSION]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.has_version_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:LegalTextIdentity)-[r:HAS_VERSION]->(:LegalTextVersion) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.identity_has_version_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:LegalTextVersion)-[r:VERSION_OF]->(:LegalTextIdentity) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.version_of_identity_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:CONTAINS]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.contains_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:LegalTextVersion)-[r:CONTAINS]->(:Provision) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.version_contains_provision_rel_count = row.get::<i64>("count")? as usize;
        }

        let q =
            "MATCH (:Provision)-[r:PART_OF_VERSION]->(:LegalTextVersion) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_part_of_version_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:MENTIONS_CITATION]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.mentions_citation_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:RESOLVES_TO]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.resolves_to_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:CITES]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.cites_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:Provision)-[r:CITES]->(:LegalTextIdentity) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_cites_identity_rel_count = row.get::<i64>("count")? as usize;
        }

        let q =
            "MATCH (:Provision)-[r:CITES_VERSION]->(:LegalTextVersion) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_cites_version_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:Provision)-[r:CITES_PROVISION]->(:Provision) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_cites_provision_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:Provision)-[r:CITES_CHAPTER]->(:ChapterVersion) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_cites_chapter_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:Provision)-[r:CITES_RANGE]->(:CitationMention) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_cites_range_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:DERIVED_FROM]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.derived_from_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:Provision)-[r:HAS_CHUNK]->(:RetrievalChunk) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_has_chunk_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:LegalTextVersion)-[r:HAS_STATUTE_CHUNK]->(:RetrievalChunk) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.version_has_statute_chunk_rel_count = row.get::<i64>("count")? as usize;
        }

        let q =
            "MATCH (:SourceDocument)-[r:SOURCE_FOR]->(:LegalTextVersion) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.source_for_version_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (:SourceDocument)-[r:SOURCE_FOR]->(:Provision) RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.source_for_provision_rel_count = row.get::<i64>("count")? as usize;
        }

        // 13. Orphan detection
        // Chunks without DERIVED_FROM relationship
        let q = "MATCH (c:RetrievalChunk)
                 WHERE NOT (c)-[:DERIVED_FROM]->()
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_chunks = row.get::<i64>("count")? as usize;
        }

        // Provisions without PART_OF_VERSION relationship
        let q = "MATCH (p:Provision)
                 WHERE NOT (p)-[:PART_OF_VERSION]->()
                 RETURN count(p) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_provisions = row.get::<i64>("count")? as usize;
        }

        // CitationMentions without source Provision
        let q = "MATCH (cm:CitationMention)
                 WHERE NOT ()-[:MENTIONS_CITATION]->(cm)
                 RETURN count(cm) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_citations = row.get::<i64>("count")? as usize;
        }

        // 14. Missing relationship detection
        // Versions without CONTAINS (should have provisions)
        let q = "MATCH (ltv:LegalTextVersion)
                 WHERE NOT (ltv)-[:CONTAINS]->()
                 RETURN count(ltv) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.versions_without_provisions = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (lti:LegalTextIdentity)
                 WHERE NOT ()-[:HAS_SECTION]->(lti)
                 RETURN count(lti) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.identities_without_chapter_section = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (lti:LegalTextIdentity)
                 WHERE NOT (lti)-[:HAS_VERSION]->(:LegalTextVersion)
                 RETURN count(lti) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.identities_without_version = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (ltv:LegalTextVersion)
                 WHERE NOT (ltv)-[:VERSION_OF]->(:LegalTextIdentity)
                 RETURN count(ltv) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.versions_without_identity = row.get::<i64>("count")? as usize;
        }

        // Chunks expected vs actual DERIVED_FROM
        let q = "MATCH (c:RetrievalChunk) RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        let total_chunks = if let Some(row) = res.next().await? {
            row.get::<i64>("count")? as usize
        } else {
            0
        };
        report.chunk_count = total_chunks;

        let q = "MATCH (n:ChapterVersion) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.chapter_version_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:LegalCorpus) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.corpus_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:CorpusEdition) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.corpus_edition_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:Jurisdiction {jurisdiction_id: 'or:state'}) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.oregon_jurisdiction_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (n:PublicBody {public_body_id: 'or:legislature'}) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.legislature_public_body_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (h:ChapterHeading)
                 WHERE NOT ()-[:HAS_HEADING]->(h)
                 RETURN count(h) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_headings = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (sd:SourceDocument)
                 WHERE NOT (sd)-[:SOURCE_FOR]->()
                 RETURN count(sd) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_source_documents = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (cv:ChapterVersion)
                 WHERE NOT ()-[:HAS_CHAPTER]->(cv)
                 RETURN count(cv) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.orphan_chapter_versions = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk {chunk_type: 'full_statute'})
                 WHERE NOT (c)-[:DERIVED_FROM]->(:LegalTextVersion)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.full_statute_chunks_without_version_source = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk {chunk_type: 'full_statute'})
                 WHERE NOT (:LegalTextVersion)-[:HAS_STATUTE_CHUNK]->(c)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.full_statute_chunks_without_reverse_link = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.chunk_type <> 'full_statute' AND NOT (c)-[:DERIVED_FROM]->(:Provision)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_chunks_without_provision_source = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (c:RetrievalChunk)
                 WHERE c.chunk_type <> 'full_statute' AND NOT (:Provision)-[:HAS_CHUNK]->(c)
                 RETURN count(c) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provision_chunks_without_reverse_link = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (ltv:LegalTextVersion)
                 WHERE NOT (ltv)-[:DERIVED_FROM]->(:SourceDocument)
                 RETURN count(ltv) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.versions_without_source_document = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (p:Provision)
                 WHERE NOT (p)-[:DERIVED_FROM]->(:SourceDocument)
                 RETURN count(p) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.provisions_without_source_document = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (v:LegalTextVersion {status: 'active'})
                 WHERE NOT (v)-[:HAS_STATUTE_CHUNK]->(:RetrievalChunk {chunk_type: 'full_statute'})
                 RETURN count(v) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.active_versions_without_statute_chunk = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH (p:Provision)
                 WHERE coalesce(p.is_implied, false) = false AND NOT (p)-[:HAS_CHUNK]->(:RetrievalChunk)
                 RETURN count(p) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.valid_provisions_without_chunk = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r]->()
                 WHERE type(r) IN ['CITES','CITES_VERSION','CITES_PROVISION','CITES_CHAPTER','CITES_RANGE']
                   AND r.via_citation_mention_id IS NULL
                 RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.cites_edges_without_mention_id = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r]->()
                 WHERE type(r) IN ['CITES','CITES_VERSION','CITES_PROVISION','CITES_CHAPTER','CITES_RANGE']
                   AND r.via_citation_mention_id IS NOT NULL
                   AND NOT EXISTS { MATCH (:CitationMention {citation_mention_id: r.via_citation_mention_id}) }
                 RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.cites_edges_without_existing_mention = row.get::<i64>("count")? as usize;
        }

        // ── Semantic node counts ────────────────────────────────────────────
        info!("Validating semantic layer...");

        let semantic_labels = [
            ("Definition", &mut report.semantic_definition_count),
            ("DefinedTerm", &mut report.semantic_defined_term_count),
            (
                "DefinitionScope",
                &mut report.semantic_definition_scope_count,
            ),
            ("Obligation", &mut report.semantic_obligation_count),
            ("Exception", &mut report.semantic_exception_count),
            ("Deadline", &mut report.semantic_deadline_count),
            ("Penalty", &mut report.semantic_penalty_count),
            ("Remedy", &mut report.semantic_remedy_count),
            ("StatusEvent", &mut report.semantic_status_event_count),
            ("SourceNote", &mut report.semantic_source_note_count),
            ("TemporalEffect", &mut report.semantic_temporal_effect_count),
            ("LineageEvent", &mut report.semantic_lineage_event_count),
            ("SessionLaw", &mut report.semantic_session_law_count),
            ("Amendment", &mut report.semantic_amendment_count),
            ("HtmlParagraph", &mut report.source_html_paragraph_count),
            (
                "ChapterFrontMatter",
                &mut report.source_chapter_front_matter_count,
            ),
            (
                "TitleChapterEntry",
                &mut report.source_title_chapter_entry_count,
            ),
            (
                "ChapterTocEntry",
                &mut report.semantic_chapter_toc_entry_count,
            ),
            ("ReservedRange", &mut report.semantic_reserved_range_count),
            (
                "ParserDiagnostic",
                &mut report.semantic_parser_diagnostic_count,
            ),
            ("LegalActor", &mut report.semantic_legal_actor_count),
            ("LegalAction", &mut report.semantic_legal_action_count),
            ("MoneyAmount", &mut report.semantic_money_amount_count),
            ("TaxRule", &mut report.semantic_tax_rule_count),
            ("RateLimit", &mut report.semantic_rate_limit_count),
            ("RequiredNotice", &mut report.semantic_required_notice_count),
            ("FormText", &mut report.semantic_form_text_count),
        ];

        for (label, field) in semantic_labels {
            let q = format!("MATCH (n:{label}) RETURN count(n) AS count");
            let mut res = self.graph.execute(query(&q)).await?;
            if let Some(row) = res.next().await? {
                *field = row.get::<i64>("count")? as usize;
            }
        }

        // LegalSemanticNode count (includes sub-labels)
        let q = "MATCH (n:LegalSemanticNode) RETURN count(n) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_legal_semantic_node_count = row.get::<i64>("count")? as usize;
        }

        // ── Semantic relationship counts ────────────────────────────────────
        let q = "MATCH ()-[r:EXPRESSES]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_expresses_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:SUPPORTED_BY]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_supported_by_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:DEFINES]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_defines_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:DEFINES_TERM]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_defines_term_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:HAS_SCOPE]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_has_scope_rel_count = row.get::<i64>("count")? as usize;
        }

        let q = "MATCH ()-[r:REQUIRES_NOTICE]->() RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_requires_notice_rel_count = row.get::<i64>("count")? as usize;
        }

        // ── Duplicate detection ─────────────────────────────────────────────
        let q = "MATCH (n)-[r]->(m)
                 WITH n, m, type(r) as t, count(r) as c
                 WHERE c > 1
                 RETURN sum(c - 1) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.duplicate_relationship_count = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        let q = "MATCH (n)-[r:CITES]->(m)
                 WITH n, m, count(r) as c
                 WHERE c > 1
                 RETURN sum(c - 1) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.duplicate_cites_edge_count = row.get::<i64>("count").unwrap_or(0) as usize;
        }

        // ── Semantic orphan detection ───────────────────────────────────────

        // Definitions without SUPPORTED_BY to Provision
        let q = "MATCH (d:Definition)
                 WHERE NOT (d)-[:SUPPORTED_BY]->(:Provision)
                 RETURN count(d) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_definitions = row.get::<i64>("count")? as usize;
        }

        // Definitions without DEFINES_TERM
        let q = "MATCH (d:Definition)
                 WHERE NOT (d)-[:DEFINES_TERM]->()
                 RETURN count(d) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_definitions_without_term = row.get::<i64>("count")? as usize;
        }

        // Definitions without HAS_SCOPE
        let q = "MATCH (d:Definition)
                 WHERE NOT (d)-[:HAS_SCOPE]->()
                 RETURN count(d) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_definitions_without_scope = row.get::<i64>("count")? as usize;
        }

        // Obligations without incoming EXPRESSES from Provision
        let q = "MATCH (o:Obligation)
                 WHERE NOT (:Provision)-[:EXPRESSES]->(o)
                 RETURN count(o) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_obligations = row.get::<i64>("count")? as usize;
        }

        // Exceptions without incoming EXPRESSES from Provision
        let q = "MATCH (e:Exception)
                 WHERE NOT (:Provision)-[:EXPRESSES]->(e)
                 RETURN count(e) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_exceptions = row.get::<i64>("count")? as usize;
        }

        // Deadlines without incoming EXPRESSES from Provision
        let q = "MATCH (d:Deadline)
                 WHERE NOT (:Provision)-[:EXPRESSES]->(d)
                 RETURN count(d) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_deadlines = row.get::<i64>("count")? as usize;
        }

        // Penalties without incoming EXPRESSES from Provision
        let q = "MATCH (pnl:Penalty)
                 WHERE NOT (:Provision)-[:EXPRESSES]->(pnl)
                 RETURN count(pnl) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_penalties = row.get::<i64>("count")? as usize;
        }

        // Remedies without incoming EXPRESSES from Provision
        let q = "MATCH (r:Remedy)
                 WHERE NOT (:Provision)-[:EXPRESSES]->(r)
                 RETURN count(r) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_remedies = row.get::<i64>("count")? as usize;
        }

        // StatusEvents not connected to LegalTextVersion or LegalTextIdentity
        let q = "MATCH (se:StatusEvent)
                 WHERE NOT (:LegalTextVersion)-[:HAS_STATUS_EVENT]->(se)
                   AND NOT (:LegalTextIdentity)-[:HAS_STATUS_EVENT]->(se)
                 RETURN count(se) AS count";
        let mut res = self.graph.execute(query(q)).await?;
        if let Some(row) = res.next().await? {
            report.semantic_orphan_status_events = row.get::<i64>("count")? as usize;
        }

        let new_orphan_queries = [
            (
                "MATCH (sn:SourceNote) WHERE NOT (sn)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(sn) AS count",
                "source_note",
            ),
            (
                "MATCH (te:TemporalEffect) WHERE NOT (te)-[:SUPPORTED_BY]->() RETURN count(te) AS count",
                "temporal_effect",
            ),
            (
                "MATCH (le:LineageEvent) WHERE NOT (:LegalTextIdentity)-[:HAS_LINEAGE_EVENT]->(le) RETURN count(le) AS count",
                "lineage_event",
            ),
            (
                "MATCH (toc:ChapterTocEntry) WHERE NOT (toc)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(toc) AS count",
                "chapter_toc_entry",
            ),
            (
                "MATCH (rr:ReservedRange) WHERE NOT (rr)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(rr) AS count",
                "reserved_range",
            ),
            (
                "MATCH (pd:ParserDiagnostic) WHERE NOT (pd)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(pd) AS count",
                "parser_diagnostic",
            ),
            (
                "MATCH (hp:HtmlParagraph) WHERE NOT (hp)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(hp) AS count",
                "html_paragraph",
            ),
            (
                "MATCH (fm:ChapterFrontMatter) WHERE NOT (fm)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(fm) AS count",
                "chapter_front_matter",
            ),
            (
                "MATCH (tce:TitleChapterEntry) WHERE NOT (tce)-[:DERIVED_FROM]->(:SourceDocument) RETURN count(tce) AS count",
                "title_chapter_entry",
            ),
            (
                "MATCH (m:MoneyAmount) WHERE NOT (m)-[:SUPPORTED_BY]->(:Provision) RETURN count(m) AS count",
                "money_amount",
            ),
            (
                "MATCH (tr:TaxRule) WHERE NOT (tr)-[:SUPPORTED_BY]->(:Provision) RETURN count(tr) AS count",
                "tax_rule",
            ),
            (
                "MATCH (rl:RateLimit) WHERE NOT (rl)-[:SUPPORTED_BY]->(:Provision) RETURN count(rl) AS count",
                "rate_limit",
            ),
            (
                "MATCH (rn:RequiredNotice) WHERE NOT (rn)-[:SUPPORTED_BY]->(:Provision) RETURN count(rn) AS count",
                "required_notice",
            ),
            (
                "MATCH (ft:FormText) WHERE NOT (ft)-[:SUPPORTED_BY]->(:Provision) RETURN count(ft) AS count",
                "form_text",
            ),
        ];
        for (q, kind) in new_orphan_queries {
            let mut res = self.graph.execute(query(q)).await?;
            let count = if let Some(row) = res.next().await? {
                row.get::<i64>("count")? as usize
            } else {
                0
            };
            match kind {
                "source_note" => report.semantic_orphan_source_notes = count,
                "temporal_effect" => report.semantic_orphan_temporal_effects = count,
                "lineage_event" => report.semantic_orphan_lineage_events = count,
                "chapter_toc_entry" => report.semantic_orphan_chapter_toc_entries = count,
                "reserved_range" => report.semantic_orphan_reserved_ranges = count,
                "parser_diagnostic" => report.semantic_orphan_parser_diagnostics = count,
                "html_paragraph" => report.source_orphan_html_paragraphs = count,
                "chapter_front_matter" => report.source_orphan_chapter_front_matter = count,
                "title_chapter_entry" => report.source_orphan_title_chapter_entries = count,
                "money_amount" => report.semantic_orphan_money_amounts = count,
                "tax_rule" => report.semantic_orphan_tax_rules = count,
                "rate_limit" => report.semantic_orphan_rate_limits = count,
                "required_notice" => report.semantic_orphan_required_notices = count,
                "form_text" => report.semantic_orphan_form_texts = count,
                _ => {}
            }
        }

        if let Some(expected) = &report.expected {
            compare_count(
                &mut report.count_mismatches,
                "LegalTextIdentity",
                expected.identities,
                report.identity_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalTextVersion",
                expected.versions,
                report.version_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision",
                expected.provisions,
                report.provision_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "CitationMention",
                expected.citations,
                report.citation_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "RetrievalChunk",
                expected.chunks,
                report.chunk_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "SourceDocument",
                expected.source_documents,
                report.source_doc_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "ChapterHeading",
                expected.headings,
                report.heading_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "ChapterVersion",
                expected.chapters,
                report.chapter_version_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalTextIdentity->HAS_VERSION->LegalTextVersion",
                expected.versions,
                report.identity_has_version_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalTextVersion->VERSION_OF->LegalTextIdentity",
                expected.versions,
                report.version_of_identity_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalTextVersion->CONTAINS->Provision",
                expected.provisions,
                report.version_contains_provision_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->PART_OF_VERSION->LegalTextVersion",
                expected.provisions,
                report.provision_part_of_version_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->MENTIONS_CITATION->CitationMention",
                expected.citations,
                report.mentions_citation_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->HAS_CHUNK->RetrievalChunk",
                expected.provision_chunks,
                report.provision_has_chunk_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalTextVersion->HAS_STATUTE_CHUNK->RetrievalChunk",
                expected.full_statute_chunks,
                report.version_has_statute_chunk_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "SourceDocument->SOURCE_FOR->LegalTextVersion",
                expected.versions,
                report.source_for_version_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "SourceDocument->SOURCE_FOR->Provision",
                expected.provisions,
                report.source_for_provision_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->CITES->LegalTextIdentity",
                expected.cites_identity_edges,
                report.provision_cites_identity_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->CITES_VERSION->LegalTextVersion",
                expected.cites_version_edges,
                report.provision_cites_version_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->CITES_PROVISION->Provision",
                expected.cites_provision_edges,
                report.provision_cites_provision_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->CITES_CHAPTER->ChapterVersion",
                expected.cites_chapter_edges,
                report.provision_cites_chapter_rel_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Provision->CITES_RANGE->CitationMention",
                expected.cites_range_edges,
                report.provision_cites_range_rel_count,
            );

            // Semantic count comparisons
            compare_count(
                &mut report.count_mismatches,
                "Definition",
                expected.definitions,
                report.semantic_definition_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "DefinedTerm",
                expected.defined_terms,
                report.semantic_defined_term_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "DefinitionScope",
                expected.definition_scopes,
                report.semantic_definition_scope_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Obligation",
                expected.obligations,
                report.semantic_obligation_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Exception",
                expected.exceptions,
                report.semantic_exception_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Deadline",
                expected.deadlines,
                report.semantic_deadline_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Penalty",
                expected.penalties,
                report.semantic_penalty_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "Remedy",
                expected.remedies,
                report.semantic_remedy_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "StatusEvent",
                expected.status_events,
                report.semantic_status_event_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalActor",
                expected.legal_actors,
                report.semantic_legal_actor_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "LegalAction",
                expected.legal_actions,
                report.semantic_legal_action_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "HtmlParagraph",
                expected.html_paragraphs,
                report.source_html_paragraph_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "ChapterFrontMatter",
                expected.chapter_front_matter,
                report.source_chapter_front_matter_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "TitleChapterEntry",
                expected.title_chapter_entries,
                report.source_title_chapter_entry_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "MoneyAmount",
                expected.money_amounts,
                report.semantic_money_amount_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "TaxRule",
                expected.tax_rules,
                report.semantic_tax_rule_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "RateLimit",
                expected.rate_limits,
                report.semantic_rate_limit_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "RequiredNotice",
                expected.required_notices,
                report.semantic_required_notice_count,
            );
            compare_count(
                &mut report.count_mismatches,
                "FormText",
                expected.form_texts,
                report.semantic_form_text_count,
            );
        }

        // Summary logging
        info!(
            "Topology: {} identities, {} versions, {} provisions, {} chunks, {} citations, {} headings, {} source_docs",
            report.identity_count,
            report.version_count,
            report.provision_count,
            total_chunks,
            report.citation_count,
            report.heading_count,
            report.source_doc_count
        );
        info!(
            "Relationships: {} HAS_VERSION, {} CONTAINS, {} MENTIONS_CITATION, {} RESOLVES_TO, {} CITES, {} DERIVED_FROM",
            report.has_version_rel_count,
            report.contains_rel_count,
            report.mentions_citation_rel_count,
            report.resolves_to_rel_count,
            report.cites_rel_count,
            report.derived_from_rel_count
        );
        if report.orphan_chunks > 0 {
            warn!(
                "{} orphan chunks (no DERIVED_FROM relationship)",
                report.orphan_chunks
            );
        }
        if report.orphan_provisions > 0 {
            warn!(
                "{} orphan provisions (no PART_OF_VERSION relationship)",
                report.orphan_provisions
            );
        }
        if report.orphan_citations > 0 {
            warn!(
                "{} orphan citations (no incoming MENTIONS_CITATION relationship)",
                report.orphan_citations
            );
        }

        // Final status aggregation
        if report.missing_primary > 0 {
            if self.require_embeddings {
                set_fail(&mut report.status);
                error!(
                    "FAIL: {} embed_primary chunks missing embeddings",
                    report.missing_primary
                );
            } else {
                set_warning(&mut report.status);
                warn!(
                    "WARN: {} embed_primary chunks missing embeddings",
                    report.missing_primary
                );
            }
        }

        if report.dimension_mismatches > 0
            || report.model_mismatches > 0
            || report.missing_input_hash > 0
            || report.input_type_mismatches > 0
            || report.output_dtype_mismatches > 0
            || report.missing_embedding_profile > 0
            || report.embedding_profile_mismatches > 0
            || report.missing_source_dimension > 0
            || report.source_dimension_mismatches > 0
            || report.missing_embedded_at > 0
            || report.mixed_dimensions > 0
            || report.chunks_missing_audit_metadata > 0
            || report.chunks_over_hard_token_limit > 0
            || report.chunks_with_invalid_part_metadata > 0
        {
            set_fail(&mut report.status);
            error!("FAIL: Embedding metadata mismatches found");
        }

        if !report.vector_index_exists {
            if self.require_embeddings {
                set_fail(&mut report.status);
                error!("FAIL: Vector index 'retrieval_chunk_embedding_1024' not found");
            } else {
                set_warning(&mut report.status);
                warn!("WARN: Vector index 'retrieval_chunk_embedding_1024' not found");
            }
        }

        if report.oregon_jurisdiction_count != 1
            || report.legislature_public_body_count != 1
            || report.corpus_count == 0
            || report.corpus_edition_count == 0
            || report.orphan_chunks > 0
            || report.orphan_provisions > 0
            || report.orphan_citations > 0
            || report.orphan_headings > 0
            || report.orphan_source_documents > 0
            || report.orphan_chapter_versions > 0
            || report.identities_without_chapter_section > 0
            || report.identities_without_version > 0
            || report.versions_without_identity > 0
            || report.versions_without_source_document > 0
            || report.provisions_without_source_document > 0
            || report.full_statute_chunks_without_version_source > 0
            || report.full_statute_chunks_without_reverse_link > 0
            || report.provision_chunks_without_provision_source > 0
            || report.provision_chunks_without_reverse_link > 0
            || report.active_versions_without_statute_chunk > 0
            || report.valid_provisions_without_chunk > 0
            || report.cites_edges_without_mention_id > 0
            || report.cites_edges_without_existing_mention > 0
            || report.semantic_orphan_definitions > 0
            || report.semantic_orphan_obligations > 0
            || report.semantic_orphan_exceptions > 0
            || report.semantic_orphan_deadlines > 0
            || report.semantic_orphan_penalties > 0
            || report.semantic_orphan_remedies > 0
            || report.semantic_orphan_status_events > 0
            || report.source_orphan_html_paragraphs > 0
            || report.source_orphan_chapter_front_matter > 0
            || report.source_orphan_title_chapter_entries > 0
            || report.semantic_orphan_money_amounts > 0
            || report.semantic_orphan_tax_rules > 0
            || report.semantic_orphan_rate_limits > 0
            || report.semantic_orphan_required_notices > 0
            || report.semantic_orphan_form_texts > 0
            || !report.count_mismatches.is_empty()
        {
            set_fail(&mut report.status);
            error!("FAIL: Neo4j topology validation failed");
        }

        info!("Neo4j QC Status: {:?}", report.status);
        Ok(report)
    }

    async fn collect_embedding_coverage(&self) -> Result<Vec<EmbeddingCoverageByLabel>> {
        let mut rows = Vec::new();
        for spec in EMBEDDING_TARGETS {
            let profile = spec.profile;
            if profile.output_dimension != self.embedding_dim
                || profile.model != self.embedding_model
                || profile.output_dtype != self.embedding_dtype
            {
                continue;
            }

            let q = format!(
                "
                MATCH (n:{label})
                OPTIONAL MATCH (n)-[:SUPPORTED_BY]->(p:Provision)
                WHERE {where_clause}
                WITH count(n) AS total,
                     count(CASE WHEN n.embedding IS NOT NULL THEN 1 END) AS embedded,
                     count(CASE WHEN n.embedding IS NOT NULL AND size(n.embedding) <> $dim THEN 1 END) AS dimension_mismatches,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_model <> $model THEN 1 END) AS model_mismatches,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_output_dtype <> $dtype THEN 1 END) AS output_dtype_mismatches,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_input_hash IS NULL THEN 1 END) AS missing_input_hash,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedded_at IS NULL THEN 1 END) AS missing_embedded_at,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_profile IS NULL THEN 1 END) AS missing_embedding_profile,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_profile <> $profile THEN 1 END) AS profile_mismatches,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_source_dimension IS NULL THEN 1 END) AS missing_source_dimension,
                     count(CASE WHEN n.embedding IS NOT NULL AND n.embedding_source_dimension <> $dim THEN 1 END) AS source_dimension_mismatches,
                     count(DISTINCT CASE WHEN n.embedding IS NOT NULL THEN n.embedding_dim END) AS distinct_dimension_count
                RETURN total, embedded, dimension_mismatches, model_mismatches, output_dtype_mismatches,
                       missing_input_hash, missing_embedded_at, missing_embedding_profile,
                       profile_mismatches, missing_source_dimension, source_dimension_mismatches,
                       distinct_dimension_count
                ",
                label = spec.label,
                where_clause = spec.where_clause
            );

            let mut res = self
                .graph
                .execute(
                    query(&q)
                        .param("dim", self.embedding_dim as i64)
                        .param("model", self.embedding_model.clone())
                        .param("dtype", self.embedding_dtype.clone())
                        .param("profile", profile.name.to_string()),
                )
                .await?;
            if let Some(row) = res.next().await? {
                let total = row.get::<i64>("total")? as usize;
                let embedded = row.get::<i64>("embedded")? as usize;
                let distinct_dimension_count = row.get::<i64>("distinct_dimension_count")? as usize;
                rows.push(EmbeddingCoverageByLabel {
                    label: profile.label.to_string(),
                    profile: profile.name.to_string(),
                    total,
                    embedded,
                    pending: total.saturating_sub(embedded),
                    dimension_mismatches: row.get::<i64>("dimension_mismatches")? as usize,
                    model_mismatches: row.get::<i64>("model_mismatches")? as usize,
                    output_dtype_mismatches: row.get::<i64>("output_dtype_mismatches")? as usize,
                    missing_input_hash: row.get::<i64>("missing_input_hash")? as usize,
                    missing_embedded_at: row.get::<i64>("missing_embedded_at")? as usize,
                    missing_embedding_profile: row.get::<i64>("missing_embedding_profile")?
                        as usize,
                    profile_mismatches: row.get::<i64>("profile_mismatches")? as usize,
                    missing_source_dimension: row.get::<i64>("missing_source_dimension")? as usize,
                    source_dimension_mismatches: row.get::<i64>("source_dimension_mismatches")?
                        as usize,
                    mixed_dimensions: usize::from(distinct_dimension_count > 1),
                });
            }
        }
        Ok(rows)
    }
}

fn set_warning(status: &mut QcStatus) {
    if matches!(status, QcStatus::Pass) {
        *status = QcStatus::Warning;
    }
}

fn set_fail(status: &mut QcStatus) {
    *status = QcStatus::Fail;
}

#[derive(Debug, Default, serde::Serialize)]
pub struct QcNeo4jReport {
    pub status: QcStatus,
    pub model: String,
    pub dimension: usize,
    pub expected: Option<ExpectedGraphCounts>,
    pub count_mismatches: Vec<String>,
    // Embedding stats
    pub eligible_chunks: usize,
    pub embedded_chunks: usize,
    pub missing_primary: usize,
    pub missing_special: usize,

    pub eligible_provisions: usize,
    pub embedded_provisions: usize,

    pub eligible_versions: usize,
    pub embedded_versions: usize,

    pub dimension_mismatches: usize,
    pub model_mismatches: usize,
    pub missing_input_hash: usize,
    pub input_type_mismatches: usize,
    pub output_dtype_mismatches: usize,
    pub missing_embedding_profile: usize,
    pub embedding_profile_mismatches: usize,
    pub missing_source_dimension: usize,
    pub source_dimension_mismatches: usize,
    pub missing_embedded_at: usize,
    pub mixed_dimensions: usize,
    pub chunks_missing_audit_metadata: usize,
    pub chunks_over_hard_token_limit: usize,
    pub chunks_with_invalid_part_metadata: usize,

    pub vector_index_exists: bool,
    pub provision_vector_index_exists: bool,
    pub version_vector_index_exists: bool,
    pub embedding_coverage: Vec<EmbeddingCoverageByLabel>,
    // Topology - Node counts
    pub identity_count: usize,
    pub version_count: usize,
    pub provision_count: usize,
    pub chunk_count: usize,
    pub citation_count: usize,
    pub source_doc_count: usize,
    pub heading_count: usize,
    pub chapter_version_count: usize,
    pub corpus_count: usize,
    pub corpus_edition_count: usize,
    pub oregon_jurisdiction_count: usize,
    pub legislature_public_body_count: usize,
    // Topology - Relationship counts
    pub has_version_rel_count: usize,
    pub identity_has_version_rel_count: usize,
    pub version_of_identity_rel_count: usize,
    pub contains_rel_count: usize,
    pub version_contains_provision_rel_count: usize,
    pub provision_part_of_version_rel_count: usize,
    pub mentions_citation_rel_count: usize,
    pub resolves_to_rel_count: usize,
    pub cites_rel_count: usize,
    pub provision_cites_identity_rel_count: usize,
    pub provision_cites_version_rel_count: usize,
    pub provision_cites_provision_rel_count: usize,
    pub provision_cites_chapter_rel_count: usize,
    pub provision_cites_range_rel_count: usize,
    pub derived_from_rel_count: usize,
    pub provision_has_chunk_rel_count: usize,
    pub version_has_statute_chunk_rel_count: usize,
    pub source_for_version_rel_count: usize,
    pub source_for_provision_rel_count: usize,
    // Topology - Orphan detection
    pub orphan_chunks: usize,
    pub orphan_provisions: usize,
    pub orphan_citations: usize,
    pub orphan_headings: usize,
    pub orphan_source_documents: usize,
    pub orphan_chapter_versions: usize,
    // Topology - Missing relationships
    pub versions_without_provisions: usize,
    pub identities_without_chapter_section: usize,
    pub identities_without_version: usize,
    pub versions_without_identity: usize,
    pub versions_without_source_document: usize,
    pub provisions_without_source_document: usize,
    pub full_statute_chunks_without_version_source: usize,
    pub full_statute_chunks_without_reverse_link: usize,
    pub provision_chunks_without_provision_source: usize,
    pub provision_chunks_without_reverse_link: usize,
    pub active_versions_without_statute_chunk: usize,
    pub valid_provisions_without_chunk: usize,
    pub cites_edges_without_mention_id: usize,
    pub cites_edges_without_existing_mention: usize,
    // Semantic - Node counts
    pub semantic_definition_count: usize,
    pub semantic_defined_term_count: usize,
    pub semantic_definition_scope_count: usize,
    pub semantic_legal_semantic_node_count: usize,
    pub semantic_obligation_count: usize,
    pub semantic_exception_count: usize,
    pub semantic_deadline_count: usize,
    pub semantic_penalty_count: usize,
    pub semantic_remedy_count: usize,
    pub semantic_status_event_count: usize,
    pub semantic_source_note_count: usize,
    pub semantic_temporal_effect_count: usize,
    pub semantic_lineage_event_count: usize,
    pub semantic_session_law_count: usize,
    pub semantic_amendment_count: usize,
    pub source_html_paragraph_count: usize,
    pub source_chapter_front_matter_count: usize,
    pub source_title_chapter_entry_count: usize,
    pub semantic_chapter_toc_entry_count: usize,
    pub semantic_reserved_range_count: usize,
    pub semantic_parser_diagnostic_count: usize,
    pub semantic_legal_actor_count: usize,
    pub semantic_legal_action_count: usize,
    pub semantic_money_amount_count: usize,
    pub semantic_tax_rule_count: usize,
    pub semantic_rate_limit_count: usize,
    pub semantic_required_notice_count: usize,
    pub semantic_form_text_count: usize,
    // Semantic - Relationship counts
    pub semantic_expresses_rel_count: usize,
    pub semantic_supported_by_rel_count: usize,
    pub semantic_defines_rel_count: usize,
    pub semantic_defines_term_rel_count: usize,
    pub semantic_has_scope_rel_count: usize,
    pub semantic_has_status_event_rel_count: usize,
    pub semantic_has_temporal_effect_rel_count: usize,
    pub semantic_has_lineage_event_rel_count: usize,
    pub semantic_affects_identity_rel_count: usize,
    pub semantic_enacts_amendment_rel_count: usize,
    pub semantic_mentions_sl_rel_count: usize,
    pub semantic_imposed_on_rel_count: usize,
    pub semantic_requires_action_rel_count: usize,
    pub semantic_has_deadline_rel_count: usize,
    pub semantic_has_form_text_rel_count: usize,
    pub semantic_requires_notice_rel_count: usize,
    // Duplicate detection
    pub duplicate_relationship_count: usize,
    pub duplicate_cites_edge_count: usize,
    // Semantic - Orphan detection
    pub semantic_orphan_definitions: usize,
    pub semantic_definitions_without_term: usize,
    pub semantic_definitions_without_scope: usize,
    pub semantic_orphan_obligations: usize,
    pub semantic_orphan_exceptions: usize,
    pub semantic_orphan_deadlines: usize,
    pub semantic_orphan_penalties: usize,
    pub semantic_orphan_remedies: usize,
    pub semantic_orphan_status_events: usize,
    pub semantic_orphan_source_notes: usize,
    pub semantic_orphan_temporal_effects: usize,
    pub semantic_orphan_lineage_events: usize,
    pub semantic_orphan_chapter_toc_entries: usize,
    pub semantic_orphan_reserved_ranges: usize,
    pub semantic_orphan_parser_diagnostics: usize,
    pub source_orphan_html_paragraphs: usize,
    pub source_orphan_chapter_front_matter: usize,
    pub source_orphan_title_chapter_entries: usize,
    pub semantic_orphan_money_amounts: usize,
    pub semantic_orphan_tax_rules: usize,
    pub semantic_orphan_rate_limits: usize,
    pub semantic_orphan_required_notices: usize,
    pub semantic_orphan_form_texts: usize,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct EmbeddingCoverageByLabel {
    pub label: String,
    pub profile: String,
    pub total: usize,
    pub embedded: usize,
    pub pending: usize,
    pub dimension_mismatches: usize,
    pub model_mismatches: usize,
    pub output_dtype_mismatches: usize,
    pub missing_input_hash: usize,
    pub missing_embedded_at: usize,
    pub missing_embedding_profile: usize,
    pub profile_mismatches: usize,
    pub missing_source_dimension: usize,
    pub source_dimension_mismatches: usize,
    pub mixed_dimensions: usize,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ExpectedGraphCounts {
    pub source_documents: usize,
    pub identities: usize,
    pub versions: usize,
    pub provisions: usize,
    pub citations: usize,
    pub chunks: usize,
    pub full_statute_chunks: usize,
    pub provision_chunks: usize,
    pub headings: usize,
    pub chapters: usize,
    pub cites_edges: usize,
    pub cites_identity_edges: usize,
    pub cites_version_edges: usize,
    pub cites_provision_edges: usize,
    pub cites_chapter_edges: usize,
    pub cites_range_edges: usize,
    // Semantic expected counts
    pub definitions: usize,
    pub defined_terms: usize,
    pub definition_scopes: usize,
    pub obligations: usize,
    pub exceptions: usize,
    pub deadlines: usize,
    pub penalties: usize,
    pub remedies: usize,
    pub status_events: usize,
    pub legal_actors: usize,
    pub legal_actions: usize,
    pub html_paragraphs: usize,
    pub chapter_front_matter: usize,
    pub title_chapter_entries: usize,
    pub money_amounts: usize,
    pub tax_rules: usize,
    pub rate_limits: usize,
    pub required_notices: usize,
    pub form_texts: usize,
    pub temporal_effects: usize,
    pub lineage_events: usize,
    pub amendments: usize,
}

fn load_expected_counts(graph_dir: &Path) -> Result<ExpectedGraphCounts> {
    let chapters =
        count_distinct_jsonl_field(&graph_dir.join("legal_text_versions.jsonl"), "chapter")?;
    let chunk_counts = count_retrieval_chunk_kinds(graph_dir)?;
    let cites_counts = count_cites_edge_kinds(&graph_dir.join("cites_edges.jsonl"))?;
    Ok(ExpectedGraphCounts {
        source_documents: count_jsonl(&graph_dir.join("source_documents.jsonl"))?,
        identities: count_jsonl(&graph_dir.join("legal_text_identities.jsonl"))?,
        versions: count_jsonl(&graph_dir.join("legal_text_versions.jsonl"))?,
        provisions: count_jsonl(&graph_dir.join("provisions.jsonl"))?,
        citations: count_jsonl(&graph_dir.join("citation_mentions.jsonl"))?,
        chunks: count_retrieval_chunks(graph_dir)?,
        full_statute_chunks: chunk_counts.full_statute,
        provision_chunks: chunk_counts.provision,
        headings: count_jsonl(&graph_dir.join("chapter_headings.jsonl"))?,
        chapters,
        cites_edges: count_jsonl(&graph_dir.join("cites_edges.jsonl"))?,
        cites_identity_edges: cites_counts.identity,
        cites_version_edges: cites_counts.version,
        cites_provision_edges: cites_counts.provision,
        cites_chapter_edges: cites_counts.chapter,
        cites_range_edges: cites_counts.range,
        // Semantic counts from JSONL
        definitions: count_jsonl(&graph_dir.join("definitions.jsonl"))?,
        defined_terms: count_jsonl(&graph_dir.join("defined_terms.jsonl"))?,
        definition_scopes: count_jsonl(&graph_dir.join("definition_scopes.jsonl"))?,
        obligations: count_jsonl(&graph_dir.join("obligations.jsonl"))?,
        exceptions: count_jsonl(&graph_dir.join("exceptions.jsonl"))?,
        deadlines: count_jsonl(&graph_dir.join("deadlines.jsonl"))?,
        penalties: count_jsonl(&graph_dir.join("penalties.jsonl"))?,
        remedies: count_jsonl(&graph_dir.join("remedies.jsonl"))?,
        status_events: count_jsonl(&graph_dir.join("status_events.jsonl"))?,
        legal_actors: count_jsonl(&graph_dir.join("legal_actors.jsonl"))?,
        legal_actions: count_jsonl(&graph_dir.join("legal_actions.jsonl"))?,
        html_paragraphs: count_jsonl(&graph_dir.join("html_paragraphs.debug.jsonl"))?,
        chapter_front_matter: count_jsonl(&graph_dir.join("chapter_front_matter.jsonl"))?,
        title_chapter_entries: count_jsonl(&graph_dir.join("title_chapter_entries.jsonl"))?,
        money_amounts: count_jsonl(&graph_dir.join("money_amounts.jsonl"))?,
        tax_rules: count_jsonl(&graph_dir.join("tax_rules.jsonl"))?,
        rate_limits: count_jsonl(&graph_dir.join("rate_limits.jsonl"))?,
        required_notices: count_jsonl(&graph_dir.join("required_notices.jsonl"))?,
        form_texts: count_jsonl(&graph_dir.join("form_texts.jsonl"))?,
        temporal_effects: count_jsonl(&graph_dir.join("temporal_effects.jsonl"))?,
        lineage_events: count_jsonl(&graph_dir.join("lineage_events.jsonl"))?,
        amendments: count_jsonl(&graph_dir.join("amendments.jsonl"))?,
    })
}

fn count_jsonl(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count())
}

fn count_retrieval_chunks(graph_dir: &Path) -> Result<usize> {
    let mut total = 0;
    count_retrieval_chunks_inner(graph_dir, &mut total)?;
    Ok(total)
}

fn count_retrieval_chunks_inner(dir: &Path, total: &mut usize) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            count_retrieval_chunks_inner(&path, total)?;
        } else if path
            .file_name()
            .map_or(false, |name| name == "retrieval_chunks.jsonl")
        {
            *total += count_jsonl(&path)?;
        }
    }
    Ok(())
}

#[derive(Default)]
struct RetrievalChunkKindCounts {
    full_statute: usize,
    provision: usize,
}

fn count_retrieval_chunk_kinds(graph_dir: &Path) -> Result<RetrievalChunkKindCounts> {
    let mut counts = RetrievalChunkKindCounts::default();
    count_retrieval_chunk_kinds_inner(graph_dir, &mut counts)?;
    Ok(counts)
}

fn count_retrieval_chunk_kinds_inner(
    dir: &Path,
    counts: &mut RetrievalChunkKindCounts,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            count_retrieval_chunk_kinds_inner(&path, counts)?;
        } else if path
            .file_name()
            .map_or(false, |name| name == "retrieval_chunks.jsonl")
        {
            for line in fs::read_to_string(path)?.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = serde_json::from_str(line)?;
                if value
                    .get("chunk_type")
                    .and_then(|v| v.as_str())
                    .map_or(false, |chunk_type| chunk_type == "full_statute")
                {
                    counts.full_statute += 1;
                } else {
                    counts.provision += 1;
                }
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct CitesEdgeKindCounts {
    identity: usize,
    version: usize,
    provision: usize,
    chapter: usize,
    range: usize,
}

fn count_cites_edge_kinds(path: &Path) -> Result<CitesEdgeKindCounts> {
    let mut counts = CitesEdgeKindCounts::default();
    if !path.exists() {
        return Ok(counts);
    }
    for line in fs::read_to_string(path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let edge_type = value
            .get("edge_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        match edge_type {
            "CITES" => {
                if value
                    .get("target_canonical_id")
                    .filter(|v| !v.is_null())
                    .is_some()
                {
                    counts.identity += 1;
                }
                if value
                    .get("target_version_id")
                    .filter(|v| !v.is_null())
                    .is_some()
                {
                    counts.version += 1;
                }
                if value
                    .get("target_provision_id")
                    .filter(|v| !v.is_null())
                    .is_some()
                {
                    counts.provision += 1;
                }
            }
            "CITES_CHAPTER" => counts.chapter += 1,
            "CITES_RANGE" => counts.range += 1,
            _ => {}
        }
    }
    Ok(counts)
}

fn count_distinct_jsonl_field(path: &Path, field: &str) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let mut values = std::collections::HashSet::new();
    for line in fs::read_to_string(path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        if let Some(v) = value.get(field).and_then(|v| v.as_str()) {
            values.insert(v.to_string());
        }
    }
    Ok(values.len())
}

fn compare_count(mismatches: &mut Vec<String>, label: &str, expected: usize, actual: usize) {
    if expected != actual {
        mismatches.push(format!("{label}: expected {expected}, actual {actual}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_count_jsonl() {
        let temp_dir =
            std::env::temp_dir().join(format!("orsgraph-qc-neo4j-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("test.jsonl");
        fs::write(&path, "{\"a\":1}\n\n{\"b\":2}\n").unwrap();

        assert_eq!(count_jsonl(&path).unwrap(), 2);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_compare_count() {
        let mut mismatches = Vec::new();
        compare_count(&mut mismatches, "Label", 10, 5);
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("expected 10, actual 5"));
    }

    #[test]
    fn test_count_retrieval_chunks() {
        let temp_dir =
            std::env::temp_dir().join(format!("orsgraph-qc-neo4j-chunks-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let sub_dir = temp_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        fs::write(temp_dir.join("retrieval_chunks.jsonl"), "{\"a\":1}\n").unwrap();
        fs::write(sub_dir.join("retrieval_chunks.jsonl"), "{\"b\":2}\n").unwrap();

        assert_eq!(count_retrieval_chunks(&temp_dir).unwrap(), 2);
        let _ = fs::remove_dir_all(temp_dir);
    }
}
