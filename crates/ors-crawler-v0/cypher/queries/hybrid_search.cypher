CALL {
  // Branch 1: Vector Search
  CALL {
    MATCH (n:RetrievalChunk)
    SEARCH n IN (
      VECTOR INDEX retrieval_chunk_embedding_1024
      FOR $embedding
      LIMIT 100
    ) SCORE AS score
    WITH n ORDER BY score DESC
    WITH collect(n) AS nodes
    UNWIND range(0, size(nodes)-1) AS i
    RETURN nodes[i] AS n, i + 1 AS rank_vec, null AS rank_ft
  }
  RETURN n, rank_vec, rank_ft
UNION ALL
  // Branch 2: Full-text Search
  CALL {
    CALL db.index.fulltext.queryNodes('legalTextFulltext', $query_text) YIELD node AS n, score
    WHERE n:RetrievalChunk
    WITH n ORDER BY score DESC
    WITH collect(n) AS nodes
    UNWIND range(0, size(nodes)-1) AS j
    RETURN nodes[j] AS n, null AS rank_vec, j + 1 AS rank_ft
    LIMIT 100
  }
  RETURN n, rank_vec, rank_ft
}
WITH n, min(rank_vec) AS r_v, min(rank_ft) AS r_f
WITH n,
     (CASE WHEN r_v IS NOT NULL THEN 1.0 / ($k + r_v) ELSE 0.0 END) +
     (CASE WHEN r_f IS NOT NULL THEN 1.0 / ($k + r_f) ELSE 0.0 END) AS rrf_score
ORDER BY rrf_score DESC
RETURN n.chunk_id AS chunk_id, n.text AS text, n.citation AS citation, rrf_score
LIMIT $limit
