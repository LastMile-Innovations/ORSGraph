// Create chapter version nodes from loaded legal text versions
// This method aggregates legal text versions by chapter and creates ChapterVersion nodes

MATCH (ltv:LegalTextVersion)
WITH DISTINCT ltv.chapter AS chapter,
     ltv.edition_year AS edition_year,
     coalesce(ltv.edition_id, CASE
        WHEN coalesce(ltv.authority_family, 'ORS') = 'UTCR' THEN 'or:utcr@' || toString(ltv.edition_year)
        WHEN coalesce(ltv.authority_family, 'ORS') = 'SLR' THEN coalesce(ltv.corpus_id, 'or:linn:slr') || '@' || toString(ltv.edition_year)
        ELSE 'or:ors@' || toString(ltv.edition_year)
     END) AS edition_id,
     coalesce(ltv.corpus_id, CASE
        WHEN coalesce(ltv.authority_family, 'ORS') = 'UTCR' THEN 'or:utcr'
        WHEN coalesce(ltv.authority_family, 'ORS') = 'SLR' THEN 'or:linn:slr'
        ELSE 'or:ors'
     END) AS corpus_id,
     coalesce(ltv.authority_family, 'ORS') AS authority_family,
     coalesce(ltv.authority_type, CASE WHEN coalesce(ltv.authority_family, 'ORS') IN ['UTCR', 'SLR'] THEN 'court_rule' ELSE 'statute' END) AS authority_type,
     coalesce(ltv.authority_level, CASE
        WHEN coalesce(ltv.authority_family, 'ORS') = 'UTCR' THEN 80
        WHEN coalesce(ltv.authority_family, 'ORS') = 'SLR' THEN 75
        ELSE 90
     END) AS authority_level,
     ltv.title AS sampleTitle
WITH chapter, edition_year, edition_id, corpus_id, authority_family, authority_type, authority_level,
     collect(sampleTitle)[0] AS sampleTitle,
     CASE
          WHEN authority_family = 'UTCR' THEN 'or:utcr:chapter:' || chapter || '@' || toString(edition_year)
          WHEN authority_family = 'SLR' THEN corpus_id || ':chapter:' || chapter || '@' || toString(edition_year)
          ELSE 'or:ors:chapter:' || chapter || '@' || toString(edition_year)
     END AS chapter_id
MERGE (cv:ChapterVersion {chapter_id: chapter_id})
SET cv += {
    id: chapter_id, graph_kind: 'authority', schema_version: '1.0.0',
    source_system: 'ors_crawler', chapter: chapter,
    citation: CASE
        WHEN authority_family = 'UTCR' THEN 'UTCR Chapter ' || chapter
        WHEN authority_family = 'SLR' THEN 'SLR Chapter ' || chapter
        ELSE 'ORS Chapter ' || chapter
    END,
    title: coalesce(cv.title, sampleTitle), corpus_id: corpus_id, edition_id: edition_id,
    edition_year: edition_year,
    jurisdiction_id: CASE WHEN authority_family = 'SLR' THEN split(corpus_id, ':slr')[0] ELSE 'or:state' END,
    authority_family: authority_family, authority_type: authority_type,
    authority_level: authority_level, current: true,
    official_status: CASE WHEN authority_family IN ['UTCR', 'SLR'] THEN 'official_pdf' ELSE 'official_online_not_official_print' END,
    disclaimer_required: NOT (authority_family IN ['UTCR', 'SLR']), updated_at: datetime()
}
SET cv.created_at = coalesce(cv.created_at, datetime())
FOREACH (_ IN CASE WHEN authority_family IN ['UTCR', 'SLR'] THEN [1] ELSE [] END | SET cv:CourtRuleChapter)
FOREACH (_ IN CASE WHEN authority_family = 'SLR' THEN [1] ELSE [] END | SET cv:SLRChapter:SupplementaryLocalRuleChapter)
