use neo4rs::{query, Graph};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("GRAPH INTEGRITY AUDIT");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. Citation graph verification
    println!("1. CITATION GRAPH VERIFICATION");
    println!("───────────────────────────────────────────────────────────");
    let mut citation_result = graph
        .execute(query(
            "MATCH ()-[r]->() 
         WHERE type(r) CONTAINS 'CITE' OR type(r) CONTAINS 'CITATION' 
         RETURN type(r) AS rel, count(r) AS count 
         ORDER BY count DESC",
        ))
        .await?;

    let mut found_citation_rels = false;
    while let Some(row) = citation_result.next().await? {
        let rel: String = row.get("rel").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {}: {}", rel, count);
        found_citation_rels = true;
    }
    if !found_citation_rels {
        println!("  ⚠️  NO CITATION RELATIONSHIPS FOUND!");
    }
    println!();

    // 2. Provision hierarchy edges
    println!("2. PROVISION HIERARCHY EDGES");
    println!("───────────────────────────────────────────────────────────");
    let hierarchy_types = vec!["HAS_PARENT", "NEXT", "PREVIOUS"];
    for rel_type in &hierarchy_types {
        let q = format!("MATCH ()-[r:{}]->() RETURN count(r) AS count", rel_type);
        let mut result = graph.execute(query(&q)).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count").unwrap();
            if count > 0 {
                println!("  {}: {}", rel_type, count);
            } else {
                println!("  ⚠️  {}: {} (MISSING)", rel_type, count);
            }
        }
    }
    println!();

    // 3. HtmlParagraph investigation
    println!("3. HTML PARAGRAPH INVESTIGATION");
    println!("───────────────────────────────────────────────────────────");
    let mut para_result = graph
        .execute(query(
            "MATCH (n) WHERE 'HtmlParagraph' IN labels(n) RETURN count(n) AS count",
        ))
        .await?;
    if let Some(row) = para_result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  HtmlParagraph nodes: {}", count);
    }

    let mut para_label_result = graph
        .execute(query(
            "MATCH (s:SourceDocument)-[:HAS_PARAGRAPH]->(p) 
         RETURN labels(p) AS paragraph_labels, count(*) AS count 
         ORDER BY count DESC LIMIT 20",
        ))
        .await?;
    println!("\n  Paragraph targets by label:");
    while let Some(row) = para_label_result.next().await? {
        let labels: Vec<String> = row.get("paragraph_labels").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("    {:?}: {}", labels, count);
    }
    println!();

    // 4. Definition duplication check
    println!("4. DEFINITION DUPLICATION CHECK");
    println!("───────────────────────────────────────────────────────────");
    let mut def_result = graph
        .execute(query(
            "MATCH (d:Definition) 
         RETURN count(d) AS total, 
                count(DISTINCT d.definition_id) AS distinct_ids",
        ))
        .await?;
    if let Some(row) = def_result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let distinct: i64 = row.get("distinct_ids").unwrap();
        println!("  Total Definition nodes: {}", total);
        println!("  Distinct definition_ids: {}", distinct);
        if total != distinct {
            println!(
                "  ⚠️  DUPLICATES FOUND: {} duplicate nodes",
                total - distinct
            );
        } else {
            println!("  ✓ No duplicates");
        }
    }
    println!();

    // 5. Correct node/edge counts
    println!("5. CORRECTED NODE & EDGE COUNTS");
    println!("───────────────────────────────────────────────────────────");
    let mut node_type_result = graph
        .execute(query(
            "MATCH (n) 
         WITH labels(n)[0] AS label 
         RETURN count(DISTINCT label) AS type_count, 
                collect(DISTINCT label) AS labels",
        ))
        .await?;
    if let Some(row) = node_type_result.next().await? {
        let type_count: i64 = row.get("type_count").unwrap();
        println!("  Unique node labels: {}", type_count);
    }

    let mut edge_type_result = graph
        .execute(query(
            "MATCH ()-[r]->() 
         RETURN count(DISTINCT type(r)) AS type_count",
        ))
        .await?;
    if let Some(row) = edge_type_result.next().await? {
        let type_count: i64 = row.get("type_count").unwrap();
        println!("  Unique relationship types: {}", type_count);
    }
    println!();

    // 6. Orphan audit
    println!("6. ORPHAN AUDIT");
    println!("───────────────────────────────────────────────────────────");

    // RetrievalChunk without DERIVED_FROM
    let mut orphan1 = graph
        .execute(query(
            "MATCH (c:RetrievalChunk) 
         WHERE NOT (c)-[:DERIVED_FROM]->() 
         RETURN count(c) AS count",
        ))
        .await?;
    if let Some(row) = orphan1.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  RetrievalChunk without DERIVED_FROM: {}", count);
    }

    // Provision without PART_OF_VERSION
    let mut orphan2 = graph
        .execute(query(
            "MATCH (p:Provision) 
         WHERE NOT (p)-[:PART_OF_VERSION]->() 
         RETURN count(p) AS count",
        ))
        .await?;
    if let Some(row) = orphan2.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Provision without PART_OF_VERSION: {}", count);
    }

    // CitationMention without source Provision
    let mut orphan3 = graph
        .execute(query(
            "MATCH (cm:CitationMention) 
         WHERE NOT ()-[:MENTIONS_CITATION]->(cm) 
         RETURN count(cm) AS count",
        ))
        .await?;
    if let Some(row) = orphan3.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  CitationMention without source: {}", count);
    }

    // LegalSemanticNode without SUPPORTED_BY
    let mut orphan4 = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode) 
         WHERE NOT (n)<-[:SUPPORTED_BY]-() 
         RETURN count(n) AS count",
        ))
        .await?;
    if let Some(row) = orphan4.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  LegalSemanticNode without SUPPORTED_BY: {}", count);
    }

    // Definition without DEFINES_TERM
    let mut orphan5 = graph
        .execute(query(
            "MATCH (d:Definition) 
         WHERE NOT (d)-[:DEFINES_TERM]->() 
         RETURN count(d) AS count",
        ))
        .await?;
    if let Some(row) = orphan5.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Definition without DEFINES_TERM: {}", count);
    }

    // Amendment without AFFECTS
    let mut orphan6 = graph
        .execute(query(
            "MATCH (a:Amendment) 
         WHERE NOT (a)-[:AFFECTS]->() 
         RETURN count(a) AS count",
        ))
        .await?;
    if let Some(row) = orphan6.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Amendment without AFFECTS: {}", count);
    }

    // SourceNote without source
    let mut orphan7 = graph
        .execute(query(
            "MATCH (sn:SourceNote) 
         WHERE NOT ()-[:HAS_SOURCE_NOTE]->(sn) 
         RETURN count(sn) AS count",
        ))
        .await?;
    if let Some(row) = orphan7.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  SourceNote without source: {}", count);
    }

    // TemporalEffect without SUPPORTED_BY
    let mut orphan8 = graph
        .execute(query(
            "MATCH (te:TemporalEffect) 
         WHERE NOT (te)<-[:SUPPORTED_BY]-() 
         RETURN count(te) AS count",
        ))
        .await?;
    if let Some(row) = orphan8.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  TemporalEffect without SUPPORTED_BY: {}", count);
    }
    println!();

    // 7. Duplicate relationship check
    println!("7. DUPLICATE RELATIONSHIP CHECK (sample)");
    println!("───────────────────────────────────────────────────────────");

    let dup_checks = vec![
        "CITES",
        "HAS_VERSION",
        "PART_OF_VERSION",
        "HAS_CHUNK",
        "HAS_STATUTE_CHUNK",
        "SUPPORTED_BY",
        "EXPRESSES",
        "DEFINES",
        "HAS_SCOPE",
    ];

    for rel_type in &dup_checks {
        let q = format!(
            "MATCH ()-[r:{}]->() 
             WITH {{startNode: startNode(r).provision_id ?? startNode(r).chunk_id ?? id(startNode), 
                   endNode: endNode(r).provision_id ?? endNode(r).chunk_id ?? id(endNode)}} AS key 
             RETURN count(*) AS total, 
                    count(DISTINCT key) AS unique_keys",
            rel_type
        );
        let mut result = graph.execute(query(&q)).await?;
        if let Some(row) = result.next().await? {
            let total: i64 = row.get("total").unwrap();
            let unique: i64 = row.get("unique_keys").unwrap();
            if total != unique {
                println!(
                    "  ⚠️  {}: {} total, {} unique ({} dups)",
                    rel_type,
                    total,
                    unique,
                    total - unique
                );
            } else {
                println!("  ✓ {}: {} (no duplicates)", rel_type, total);
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("AUDIT COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
