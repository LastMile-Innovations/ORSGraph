// Create relationships between chapter headings and the sections they contain
// Links headings to sections that fall within their range

CALL {
    MATCH (h:ChapterHeading)
    MATCH (lti:LegalTextIdentity {chapter: h.chapter})
    MATCH (:LegalTextVersion {canonical_id: lti.canonical_id})-[:CONTAINS]->(p:Provision)
    WITH h, lti, min(p.order_index) AS sectionOrder
    WHERE sectionOrder >= h.order_index
    OPTIONAL MATCH (next:ChapterHeading {chapter: h.chapter})
    WHERE next.order_index > h.order_index AND next.order_index <= sectionOrder
    WITH h, lti, count(next) AS intervening_headings
    WHERE intervening_headings = 0
    MERGE (h)-[:CONTAINS_SECTION]->(lti)
} IN TRANSACTIONS OF 1000 ROWS
