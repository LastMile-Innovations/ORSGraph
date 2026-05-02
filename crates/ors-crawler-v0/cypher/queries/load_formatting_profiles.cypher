UNWIND $rows AS row
MERGE (profile:FormattingProfile {formatting_profile_id: row.formatting_profile_id})
SET profile += row { .name, .source_corpus_id, .source_edition_id, .effective_date, .properties }
SET profile.id = row.formatting_profile_id,
    profile.graph_kind = 'rule_pack',
    profile.schema_version = '1.0.0',
    profile.source_system = 'ors_crawler',
    profile.updated_at = datetime()
SET profile.created_at = coalesce(profile.created_at, datetime())
WITH profile, row
MATCH (c:LegalCorpus {corpus_id: row.source_corpus_id})
MERGE (profile)-[:DERIVED_FROM]->(c)
