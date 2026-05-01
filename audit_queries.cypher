// ═══════════════════════════════════════════════════════════
// NEO4J GRAPH AUDIT & QC QUERIES
// Run these in Neo4j Browser at: neo4j-production-299e.up.railway.app:7474
// Login: neo4j / orsgraph2025
// ═══════════════════════════════════════════════════════════

// 1. NODE COUNTS BY LABEL
MATCH (n) 
RETURN labels(n)[0] as label, count(n) as count 
ORDER BY count DESC;

// 2. RELATIONSHIP COUNTS BY TYPE
MATCH ()-[r]->() 
RETURN type(r) as type, count(r) as count 
ORDER BY count DESC;

// 3. DUPLICATE CHECK: LegalTextIdentity by citation
MATCH (n:LegalTextIdentity)
WITH n.citation as citation, count(n) as cnt
WHERE cnt > 1
RETURN citation, cnt as duplicates
ORDER BY cnt DESC
LIMIT 10;

// 4. DUPLICATE CHECK: Provision by provision_id
MATCH (n:Provision)
WITH n.provision_id as pid, count(n) as cnt
WHERE cnt > 1
RETURN pid, cnt as duplicates
ORDER BY cnt DESC
LIMIT 10;

// 5. ORPHAN CHECK: Provisions without PART_OF_VERSION
MATCH (p:Provision)
WHERE NOT (p)-[:PART_OF_VERSION]->()
RETURN count(p) as orphan_provisions;

// 6. ORPHAN CHECK: RetrievalChunks without DERIVED_FROM
MATCH (c:RetrievalChunk)
WHERE NOT (c)-[:DERIVED_FROM]->()
RETURN count(c) as orphan_chunks;

// 7. ORPHAN CHECK: CitationMentions without source
MATCH (cm:CitationMention)
WHERE NOT ()-[:MENTIONS_CITATION]->(cm)
RETURN count(cm) as orphan_citations;

// 8. PROVISION HIERARCHY COMPLETENESS
MATCH (p:Provision)
OPTIONAL MATCH (p)-[:HAS_PARENT]->(parent)
OPTIONAL MATCH (p)-[:PART_OF_VERSION]->(version)
RETURN 
  count(p) as total_provisions,
  count(parent) as with_parent,
  count(version) as with_version,
  count(p) - count(version) as missing_version;

// 9. CITATION GRAPH INTEGRITY
MATCH ()-[r:CITES]->()
WITH count(r) as total
MATCH ()-[r:CITES]->()
WITH total, count(DISTINCT {s: startNode(r).provision_id, e: endNode(r).provision_id}) as unique
RETURN total, unique, total - unique as duplicates;

// 10. CHECK MAJOR RELATIONSHIPS EXIST
UNWIND [
  "CITES", "HAS_VERSION", "PART_OF_VERSION", "HAS_CHUNK", 
  "DERIVED_FROM", "MENTIONS_CITATION", "SUPPORTED_BY", 
  "DEFINES", "HAS_SCOPE", "AFFECTS"
] AS relType
CALL {
  WITH relType
  CALL apoc.cypher.run("MATCH ()-[r:" + relType + "]->() RETURN count(r) as cnt", {})
  YIELD value
  RETURN value.cnt as count
}
RETURN relType, count
ORDER BY count DESC;

// 11. INDEX STATUS
SHOW INDEXES 
YIELD name, type, state 
RETURN name, type, state 
ORDER BY type, name;

// 12. TOTAL NODES AND EDGES SUMMARY
CALL {
  MATCH (n) RETURN count(n) as total_nodes
}
CALL {
  MATCH ()-[r]->() RETURN count(r) as total_edges
}
RETURN total_nodes, total_edges, total_nodes + total_edges as total_elements;
