UNWIND $rows AS row
MERGE (pack:WorkProductRulePack {rule_pack_id: row.rule_pack_id})
SET pack += row { .name, .jurisdiction, .court_system, .effective_date, .source_corpus_id,
                .source_edition_id, .work_product_types, .inherits, .description }
SET pack.id = row.rule_pack_id,
    pack.graph_kind = 'rule_pack',
    pack.schema_version = '1.0.0',
    pack.source_system = 'ors_crawler',
    pack.updated_at = datetime()
SET pack.created_at = coalesce(pack.created_at, datetime())
WITH pack, row
MATCH (c:LegalCorpus {corpus_id: row.source_corpus_id})
MERGE (pack)-[:DERIVED_FROM]->(c)
