// Create chapter version nodes from loaded legal text versions
// This method aggregates legal text versions by chapter and creates ChapterVersion nodes

MATCH (ltv:LegalTextVersion)
WITH DISTINCT ltv.chapter AS chapter, ltv.edition_year AS edition_year, ltv.title AS sampleTitle
WITH chapter, edition_year, collect(sampleTitle)[0] AS sampleTitle,
     'or:ors:chapter:' || chapter || '@' || toString(edition_year) AS chapter_id
MERGE (cv:ChapterVersion {chapter_id: chapter_id})
SET cv += {
    id: chapter_id, graph_kind: 'authority', schema_version: '1.0.0',
    source_system: 'ors_crawler', chapter: chapter,
    citation: 'ORS Chapter ' || chapter, title: sampleTitle,
    edition_year: edition_year, jurisdiction_id: 'or:state',
    authority_family: 'ORS', authority_level: 90, current: true,
    official_status: 'official_online_not_official_print',
    disclaimer_required: true, updated_at: datetime()
}
SET cv.created_at = coalesce(cv.created_at, datetime())
