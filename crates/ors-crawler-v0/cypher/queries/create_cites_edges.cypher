UNWIND $rows AS row
WITH row
MATCH (source:Provision {provision_id: row.source_provision_id})
MATCH (cm:CitationMention {citation_mention_id: row.citation_mention_id})
OPTIONAL MATCH (target_identity:LegalTextIdentity {canonical_id: row.target_canonical_id})
OPTIONAL MATCH (target_version:LegalTextVersion {version_id: row.target_version_id})
OPTIONAL MATCH (target_provision:Provision {provision_id: row.target_provision_id})
OPTIONAL MATCH (target_chapter:ChapterVersion {chapter_id: row.target_chapter_id})

// CITES edges to LegalTextIdentity
FOREACH (_ IN CASE WHEN target_identity IS NULL THEN [] ELSE [1] END |
    MERGE (source)-[r:CITES {edge_id: row.edge_id}]->(target_identity)
    SET r.via_citation_mention_id = row.citation_mention_id,
        r.edge_type = row.edge_type,
        r.raw_text = cm.raw_text,
        r.normalized_citation = cm.normalized_citation,
        r.citation_type = cm.citation_type,
        r.citation_kind = row.citation_kind,
        r.confidence = cm.confidence,
        r.resolver_status = cm.resolver_status
)

// CITES_VERSION edges to LegalTextVersion
FOREACH (_ IN CASE WHEN target_version IS NULL THEN [] ELSE [1] END |
    MERGE (source)-[r:CITES_VERSION {edge_id: row.edge_id}]->(target_version)
    SET r.via_citation_mention_id = row.citation_mention_id,
        r.edge_type = row.edge_type,
        r.raw_text = cm.raw_text,
        r.normalized_citation = cm.normalized_citation,
        r.citation_type = cm.citation_type,
        r.citation_kind = row.citation_kind,
        r.confidence = cm.confidence,
        r.resolver_status = cm.resolver_status
)

// CITES_PROVISION edges to Provision
FOREACH (_ IN CASE WHEN target_provision IS NULL THEN [] ELSE [1] END |
    MERGE (source)-[r:CITES_PROVISION {edge_id: row.edge_id}]->(target_provision)
    SET r.via_citation_mention_id = row.citation_mention_id,
        r.edge_type = row.edge_type,
        r.raw_text = cm.raw_text,
        r.normalized_citation = cm.normalized_citation,
        r.citation_type = cm.citation_type,
        r.citation_kind = row.citation_kind,
        r.confidence = cm.confidence,
        r.resolver_status = cm.resolver_status
)

// CITES_CHAPTER edges to ChapterVersion
FOREACH (_ IN CASE WHEN target_chapter IS NULL THEN [] ELSE [1] END |
    MERGE (source)-[r:CITES_CHAPTER {edge_id: row.edge_id}]->(target_chapter)
    SET r.via_citation_mention_id = row.citation_mention_id,
        r.edge_type = row.edge_type,
        r.raw_text = cm.raw_text,
        r.normalized_citation = cm.normalized_citation,
        r.citation_type = cm.citation_type,
        r.citation_kind = row.citation_kind,
        r.confidence = cm.confidence,
        r.resolver_status = cm.resolver_status
)

// CITES_RANGE edges to CitationMention itself
FOREACH (_ IN CASE WHEN row.edge_type = 'CITES_RANGE' THEN [1] ELSE [] END |
    MERGE (source)-[r:CITES_RANGE {edge_id: row.edge_id}]->(cm)
    SET r.via_citation_mention_id = row.citation_mention_id,
        r.edge_type = row.edge_type,
        r.raw_text = cm.raw_text,
        r.normalized_citation = cm.normalized_citation,
        r.citation_type = cm.citation_type,
        r.citation_kind = row.citation_kind,
        r.confidence = cm.confidence,
        r.resolver_status = cm.resolver_status
)
