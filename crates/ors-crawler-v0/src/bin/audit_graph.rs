use neo4rs::{query, Graph};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get connection details from environment or use defaults
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://neo4j:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "orsgraph2025".to_string());

    println!("═══════════════════════════════════════════════════════════");
    println!("NEO4J GRAPH AUDIT & QC REPORT");
    println!("═══════════════════════════════════════════════════════════");
    println!("Connecting to: {}", uri);

    let graph = Graph::new(&uri, &user, &password).await?;
    println!("✓ Connected successfully\n");

    // 1. Node counts by label
    println!("1. NODE COUNTS BY LABEL");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (n) RETURN labels(n)[0] as label, count(n) as count ORDER BY count DESC",
        ))
        .await?;

    let mut total_nodes = 0;
    while let Some(row) = result.next().await? {
        let label: String = row.get("label").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {:30}: {:>10}", label, count);
        total_nodes += count;
    }
    println!("  {:30}: {:>10}", "TOTAL", total_nodes);

    // 2. Relationship counts by type
    println!("\n2. RELATIONSHIP COUNTS BY TYPE");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH ()-[r]->() RETURN type(r) as type, count(r) as count ORDER BY count DESC",
        ))
        .await?;

    let mut total_rels = 0;
    while let Some(row) = result.next().await? {
        let rel_type: String = row.get("type").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {:30}: {:>10}", rel_type, count);
        total_rels += count;
    }
    println!("  {:30}: {:>10}", "TOTAL", total_rels);

    // 3. Check for duplicates - LegalTextIdentity
    println!("\n3. DUPLICATE CHECK: LegalTextIdentity by citation");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph.execute(query(
        "MATCH (n:LegalTextIdentity) WITH n.citation as citation, count(n) as cnt WHERE cnt > 1 RETURN citation, cnt ORDER BY cnt DESC LIMIT 10"
    )).await?;

    let mut found_dups = false;
    while let Some(row) = result.next().await? {
        let citation: String = row.get("citation").unwrap();
        let cnt: i64 = row.get("cnt").unwrap();
        println!("  ⚠️  {}: {} duplicates", citation, cnt);
        found_dups = true;
    }
    if !found_dups {
        println!("  ✓ No duplicates found");
    }

    // 4. Check for duplicates - Provision
    println!("\n4. DUPLICATE CHECK: Provision by provision_id");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph.execute(query(
        "MATCH (n:Provision) WITH n.provision_id as pid, count(n) as cnt WHERE cnt > 1 RETURN pid, cnt ORDER BY cnt DESC LIMIT 10"
    )).await?;

    found_dups = false;
    while let Some(row) = result.next().await? {
        let pid: String = row.get("pid").unwrap();
        let cnt: i64 = row.get("cnt").unwrap();
        println!("  ⚠️  {}: {} duplicates", pid, cnt);
        found_dups = true;
    }
    if !found_dups {
        println!("  ✓ No duplicates found");
    }

    // 5. Orphan check - Provisions without PART_OF_VERSION
    println!("\n5. ORPHAN CHECK: Provisions without PART_OF_VERSION");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (p:Provision) WHERE NOT (p)-[:PART_OF_VERSION]->() RETURN count(p) as cnt",
        ))
        .await?;

    if let Some(row) = result.next().await? {
        let cnt: i64 = row.get("cnt").unwrap();
        if cnt > 0 {
            println!("  ⚠️  {} orphan provisions", cnt);
        } else {
            println!("  ✓ All provisions connected to version");
        }
    }

    // 6. Orphan check - RetrievalChunks without DERIVED_FROM
    println!("\n6. ORPHAN CHECK: RetrievalChunks without DERIVED_FROM");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (c:RetrievalChunk) WHERE NOT (c)-[:DERIVED_FROM]->() RETURN count(c) as cnt",
        ))
        .await?;

    if let Some(row) = result.next().await? {
        let cnt: i64 = row.get("cnt").unwrap();
        if cnt > 0 {
            println!("  ⚠️  {} orphan chunks", cnt);
        } else {
            println!("  ✓ All chunks connected to provision");
        }
    }

    // 7. Orphan check - CitationMentions without source
    println!("\n7. ORPHAN CHECK: CitationMentions without source");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph.execute(query(
        "MATCH (cm:CitationMention) WHERE NOT ()-[:MENTIONS_CITATION]->(cm) RETURN count(cm) as cnt"
    )).await?;

    if let Some(row) = result.next().await? {
        let cnt: i64 = row.get("cnt").unwrap();
        if cnt > 0 {
            println!("  ⚠️  {} orphan citation mentions", cnt);
        } else {
            println!("  ✓ All citation mentions connected");
        }
    }

    // 8. Check provision hierarchy
    println!("\n8. PROVISION HIERARCHY COMPLETENESS");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph.execute(query(
        "MATCH (p:Provision) OPTIONAL MATCH (p)-[:HAS_PARENT]->(parent) OPTIONAL MATCH (p)-[:PART_OF_VERSION]->(version) RETURN count(p) as total, count(parent) as with_parent, count(version) as with_version"
    )).await?;

    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let with_parent: i64 = row.get("with_parent").unwrap();
        let with_version: i64 = row.get("with_version").unwrap();
        println!("  Total provisions:      {:>10}", total);
        println!(
            "  With parent:           {:>10} ({:.1}%)",
            with_parent,
            (with_parent as f64 / total as f64) * 100.0
        );
        println!(
            "  With version:          {:>10} ({:.1}%)",
            with_version,
            (with_version as f64 / total as f64) * 100.0
        );
        if with_version == total {
            println!("  ✓ All provisions connected to version");
        }
    }

    // 9. Check for duplicate CITES relationships
    println!("\n9. DUPLICATE RELATIONSHIP CHECK: CITES");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph.execute(query(
        "MATCH ()-[r:CITES]->() WITH count(r) as total MATCH ()-[r:CITES]->() WITH total, count(DISTINCT {s: startNode(r).provision_id, e: endNode(r).provision_id}) as unique RETURN total, unique, total - unique as duplicates"
    )).await?;

    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let unique: i64 = row.get("unique").unwrap();
        let dups: i64 = row.get("duplicates").unwrap();
        println!("  Total CITES:     {}", total);
        println!("  Unique pairs:    {}", unique);
        if dups > 0 {
            println!("  ⚠️  Duplicates:   {}", dups);
        } else {
            println!("  ✓ No duplicates");
        }
    }

    // 10. Index check
    println!("\n10. INDEX STATUS");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "SHOW INDEXES YIELD name, type, state RETURN name, type, state ORDER BY type, name",
        ))
        .await?;

    let mut index_count = 0;
    while let Some(row) = result.next().await? {
        let name: String = row.get("name").unwrap();
        let idx_type: String = row.get("type").unwrap();
        let state: String = row.get("state").unwrap();
        let status = if state == "ONLINE" { "✓" } else { "⚠️" };
        println!("  {} {:40} {:15} {}", status, name, idx_type, state);
        index_count += 1;
    }
    println!("  Total indexes: {}", index_count);

    println!("\n═══════════════════════════════════════════════════════════");
    println!("AUDIT COMPLETE");
    println!("═══════════════════════════════════════════════════════════");
    println!("Total Nodes: {} | Total Edges: {}", total_nodes, total_rels);

    Ok(())
}
