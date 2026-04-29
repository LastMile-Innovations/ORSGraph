use neo4rs::{query, Graph};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("FIXING MISSING RELATIONSHIPS");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. Create MENTIONS_CITATION relationships
    println!("1. Creating MENTIONS_CITATION relationships...");
    let start = Instant::now();
    let mut result = graph
        .execute(query(
            "CALL {
            MATCH (cm:CitationMention)
            MATCH (p:Provision {provision_id: cm.source_provision_id})
            MERGE (p)-[:MENTIONS_CITATION]->(cm)
        } IN TRANSACTIONS OF 5000 ROWS",
        ))
        .await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 2. Create RESOLVES_TO relationships
    println!("2. Creating RESOLVES_TO relationships...");
    let start = Instant::now();
    let mut result = graph
        .execute(query(
            "CALL {
            MATCH (cm:CitationMention)
            WHERE cm.target_canonical_id IS NOT NULL
            MATCH (lti:LegalTextIdentity {canonical_id: cm.target_canonical_id})
            MERGE (cm)-[:RESOLVES_TO]->(lti)
        } IN TRANSACTIONS OF 5000 ROWS",
        ))
        .await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 3. Create RESOLVES_TO_VERSION relationships
    println!("3. Creating RESOLVES_TO_VERSION relationships...");
    let start = Instant::now();
    let mut result = graph
        .execute(query(
            "CALL {
            MATCH (cm:CitationMention)
            WHERE cm.target_canonical_id IS NOT NULL
            MATCH (ltv:LegalTextVersion {canonical_id: cm.target_canonical_id})
            MERGE (cm)-[:RESOLVES_TO_VERSION]->(ltv)
        } IN TRANSACTIONS OF 5000 ROWS",
        ))
        .await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 4. Create RESOLVES_TO_PROVISION relationships
    println!("4. Creating RESOLVES_TO_PROVISION relationships...");
    let start = Instant::now();
    let mut result = graph
        .execute(query(
            "CALL {
            MATCH (cm:CitationMention)
            WHERE cm.target_provision_id IS NOT NULL
            MATCH (p:Provision {provision_id: cm.target_provision_id})
            MERGE (cm)-[:RESOLVES_TO_PROVISION]->(p)
        } IN TRANSACTIONS OF 5000 ROWS",
        ))
        .await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 5. Create provision hierarchy - HAS_PARENT and CONTAINS
    println!("5. Creating provision hierarchy (HAS_PARENT, CONTAINS)...");
    let start = Instant::now();
    let mut result = graph
        .execute(query(
            "CALL {
            MATCH (child:Provision)
            WHERE size(child.local_path) > 1 AND NOT child.local_path = ['root']
            WITH child, child.local_path[..-1] AS parentPath
            MATCH (parent:Provision {version_id: child.version_id, local_path: parentPath})
            MERGE (child)-[:HAS_PARENT]->(parent)
            MERGE (parent)-[:CONTAINS]->(child)
        } IN TRANSACTIONS OF 5000 ROWS",
        ))
        .await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 6. Create NEXT/PREVIOUS relationships
    println!("6. Creating provision ordering (NEXT, PREVIOUS)...");
    let start = Instant::now();
    let mut result = graph.execute(query(
        "CALL {
            MATCH (current:Provision)
            MATCH (next:Provision {version_id: current.version_id, order_index: current.order_index + 1})
            MERGE (current)-[:NEXT]->(next)
            MERGE (next)-[:PREVIOUS]->(current)
        } IN TRANSACTIONS OF 5000 ROWS"
    )).await?;
    while result.next().await?.is_some() {}
    println!("   ✓ Done in {:.2}s\n", start.elapsed().as_secs_f64());

    // 7. Fix Definition duplicates - this is harder, need to delete and recreate
    println!("7. Checking Definition duplicates...");
    let mut result = graph
        .execute(query(
            "MATCH (d:Definition) 
         RETURN count(d) AS total, count(DISTINCT d.definition_id) AS distinct",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let distinct: i64 = row.get("distinct").unwrap();
        if total != distinct {
            println!(
                "   ⚠️ Found {} duplicates. Deleting duplicates...",
                total - distinct
            );
            // Delete duplicates keeping one per definition_id
            let mut del_result = graph
                .execute(query(
                    "MATCH (d:Definition)
                 WITH d.definition_id AS id, collect(d) AS nodes
                 WHERE size(nodes) > 1
                 UNWIND nodes[1..] AS dup
                 DETACH DELETE dup",
                ))
                .await?;
            while del_result.next().await?.is_some() {}
            println!("   ✓ Duplicates removed");
        } else {
            println!("   ✓ No duplicates found");
        }
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("RELATIONSHIP FIX COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
