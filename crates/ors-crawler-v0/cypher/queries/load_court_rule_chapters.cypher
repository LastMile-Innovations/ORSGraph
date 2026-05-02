UNWIND $rows AS row
WITH row,
     CASE WHEN row.corpus_id ENDS WITH ':slr' THEN 'SLR' ELSE 'UTCR' END AS authority_family,
     CASE WHEN row.corpus_id ENDS WITH ':slr' THEN split(row.corpus_id, ':slr')[0] ELSE 'or:state' END AS jurisdiction_id,
     CASE WHEN row.corpus_id ENDS WITH ':slr' THEN 75 ELSE 80 END AS authority_level
MERGE (ch:ChapterVersion:CourtRuleChapter {chapter_id: row.chapter_id})
SET ch += row { .corpus_id, .edition_id, .chapter, .title, .citation, .edition_year,
              .effective_date, .source_page_start, .source_page_end }
SET ch.id = row.chapter_id,
    ch.graph_kind = 'authority',
    ch.schema_version = '1.0.0',
    ch.source_system = 'ors_crawler',
    ch.jurisdiction_id = jurisdiction_id,
    ch.authority_family = authority_family,
    ch.authority_type = 'court_rule',
    ch.authority_level = authority_level,
    ch.current = true,
    ch.updated_at = datetime()
SET ch.created_at = coalesce(ch.created_at, datetime())
FOREACH (_ IN CASE WHEN authority_family = 'UTCR' THEN [1] ELSE [] END | SET ch:UTCRChapter)
FOREACH (_ IN CASE WHEN authority_family = 'SLR' THEN [1] ELSE [] END | SET ch:SLRChapter:SupplementaryLocalRuleChapter)
WITH ch, row
MATCH (e:CorpusEdition {edition_id: row.edition_id})
MERGE (e)-[:HAS_CHAPTER]->(ch)
