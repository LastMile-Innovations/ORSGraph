// Create structural relationships between corpus, chapters, and sections
// This method creates edges linking corpus editions to chapters, chapters to sections, and chapters to headings
// Parameters: $edition_id, $edition_year

CALL {
    MATCH (e:CorpusEdition {edition_id: $edition_id})
    MATCH (cv:ChapterVersion)
    WHERE cv.edition_year = $edition_year
    MERGE (e)-[:HAS_CHAPTER]->(cv)
    WITH cv
    OPTIONAL MATCH (sd:SourceDocument {chapter: cv.chapter, edition_year: cv.edition_year})
    FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
        MERGE (cv)-[:DERIVED_FROM]->(sd)
    )
    WITH cv
    MATCH (lti:LegalTextIdentity {chapter: cv.chapter})
    MERGE (cv)-[:HAS_SECTION]->(lti)
    WITH DISTINCT cv
    MATCH (h:ChapterHeading {chapter: cv.chapter})
    MERGE (cv)-[:HAS_HEADING]->(h)
} IN TRANSACTIONS OF 1000 ROWS
