// Create structural relationships between corpus, chapters, and sections
// This method creates edges linking corpus editions to chapters, chapters to sections, and chapters to headings
// Parameters: $edition_id, $edition_year

CALL {
    MATCH (cv:ChapterVersion)
    MATCH (e:CorpusEdition {edition_id: cv.edition_id})
    MERGE (e)-[:HAS_CHAPTER]->(cv)
    WITH cv
    OPTIONAL MATCH (sd:SourceDocument)
    WHERE sd.chapter = cv.chapter
      AND sd.edition_year = cv.edition_year
      AND coalesce(sd.corpus_id, cv.corpus_id) = cv.corpus_id
    FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
        MERGE (cv)-[:DERIVED_FROM]->(sd)
    )
    WITH cv
    MATCH (lti:LegalTextIdentity {chapter: cv.chapter})
    WHERE coalesce(lti.corpus_id, cv.corpus_id) = cv.corpus_id
    MERGE (cv)-[:HAS_SECTION]->(lti)
    MERGE (cv)-[:HAS_RULE]->(lti)
    WITH DISTINCT cv
    MATCH (h:ChapterHeading {chapter: cv.chapter})
    WHERE coalesce(h.authority_family, cv.authority_family) = cv.authority_family
    MERGE (cv)-[:HAS_HEADING]->(h)
} IN TRANSACTIONS OF 1000 ROWS
