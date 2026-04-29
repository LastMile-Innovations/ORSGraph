use neo4rs::{query, Graph};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("DETAILED AUDIT");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. LegalSemanticNode breakdown
    println!("1. LEGALSEMANTICNODE BREAKDOWN");
    println!("───────────────────────────────────────────────────────────");

    // Check what labels LegalSemanticNode nodes have
    let mut result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode)
         UNWIND labels(n) AS label
         RETURN label, count(*) AS count
         ORDER BY count DESC
         LIMIT 20",
        ))
        .await?;
    println!("Labels on LegalSemanticNode:");
    while let Some(row) = result.next().await? {
        let label: String = row.get("label").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {}: {}", label, count);
    }

    // Check specialized semantic nodes
    let specialized = vec![
        "Obligation",
        "Deadline",
        "Penalty",
        "Exception",
        "Remedy",
        "RequiredNotice",
        "FormText",
        "TaxRule",
        "RateLimit",
        "MoneyAmount",
    ];
    println!("\nSpecialized semantic nodes with SUPPORTED_BY:");
    for label in &specialized {
        let q = format!(
            "MATCH (n:{}) RETURN count(n) AS total, count {{ (n)-[:SUPPORTED_BY]->() }} AS supported",
            label
        );
        let mut result = graph.execute(query(&q)).await?;
        if let Some(row) = result.next().await? {
            let total: i64 = row.get("total").unwrap();
            let supported: i64 = row.get("supported").unwrap();
            if total > 0 {
                println!("  {}: {}/{} with SUPPORTED_BY", label, supported, total);
            }
        }
    }

    // Check EXPRESSES edges to specialized nodes
    println!("\nEXPRESSES edges to specialized nodes:");
    for label in &specialized {
        let q = format!(
            "MATCH ()-[:EXPRESSES]->(n:{}) RETURN count(*) AS count",
            label
        );
        let mut result = graph.execute(query(&q)).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count").unwrap();
            if count > 0 {
                println!("  -> {}: {}", label, count);
            }
        }
    }

    println!();

    // 2. SessionLaw investigation
    println!("2. SESSIONLAW INVESTIGATION");
    println!("───────────────────────────────────────────────────────────");

    // Check SessionLaw relationships
    let mut result = graph
        .execute(query(
            "MATCH (sl:SessionLaw)
         OPTIONAL MATCH (sl)<-[:ENACTS]-(a:Amendment)
         OPTIONAL MATCH (sl)<-[:REFERENCES_SESSION_LAW]-(p:Provision)
         OPTIONAL MATCH (sl)<-[:MENTIONS_SESSION_LAW]-(cm:CitationMention)
         RETURN count(sl) AS total,
                count(DISTINCT a) AS enacted_by_amendment,
                count(DISTINCT p) AS referenced_by_provision,
                count(DISTINCT cm) AS mentioned_by_citation",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let by_amendment: i64 = row.get("enacted_by_amendment").unwrap();
        let by_provision: i64 = row.get("referenced_by_provision").unwrap();
        let by_citation: i64 = row.get("mentioned_by_citation").unwrap();
        println!("  Total SessionLaw: {}", total);
        println!("  Referenced by Provision: {}", by_provision);
        println!("  Mentioned by CitationMention: {}", by_citation);
        println!("  Enacted by Amendment: {}", by_amendment);
    }

    // 3. CITES edge verification
    println!("\n3. CITES EDGE VERIFICATION");
    println!("───────────────────────────────────────────────────────────");

    let mut result = graph
        .execute(query(
            "MATCH ()-[r:CITES]->()
         RETURN count(r) AS total,
                count(DISTINCT r.edge_id) AS distinct_edges,
                count(DISTINCT startNode(r)) AS source_nodes,
                count(DISTINCT endNode(r)) AS target_nodes",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let distinct: i64 = row.get("distinct_edges").unwrap();
        let sources: i64 = row.get("source_nodes").unwrap();
        let targets: i64 = row.get("target_nodes").unwrap();
        println!("  Total CITES edges: {}", total);
        println!("  Distinct edge_ids: {}", distinct);
        println!("  Source nodes: {}", sources);
        println!("  Target nodes: {}", targets);
        if total != distinct {
            println!("  ⚠️  Duplicates found: {}", total - distinct);
        } else {
            println!("  ✓ No duplicates");
        }
    }

    // 4. Top relationship types by count
    println!("\n4. TOP 20 RELATIONSHIP TYPES BY COUNT");
    println!("───────────────────────────────────────────────────────────");

    let mut result = graph
        .execute(query(
            "MATCH ()-[r]->()
         RETURN type(r) AS rel, count(r) AS count
         ORDER BY count DESC
         LIMIT 20",
        ))
        .await?;
    while let Some(row) = result.next().await? {
        let rel: String = row.get("rel").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {:25} : {:>8}", rel, count);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("AUDIT COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
