use neo4rs::{query, Graph};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("FIXING CITES EDGES");
    println!("═══════════════════════════════════════════════════════════\n");

    // Check current state
    let mut result = graph
        .execute(query("MATCH ()-[r:CITES]->() RETURN count(r) AS count"))
        .await?;
    let current_cites = if let Some(row) = result.next().await? {
        row.get::<i64>("count").unwrap_or(0)
    } else {
        0
    };
    println!("Current CITES edges: {}", current_cites);

    if current_cites == 0 {
        println!("\nCreating CITES edges from citations with target_canonical_id...");
        let start = Instant::now();

        // Create CITES for all citations that have a matching LegalTextIdentity
        // Using ANY resolver_status that has a target_canonical_id
        let mut create_result = graph
            .execute(query(
                "CALL {
                MATCH (p:Provision)-[:MENTIONS_CITATION]->(cm:CitationMention)
                WHERE cm.target_canonical_id IS NOT NULL
                MATCH (lti:LegalTextIdentity {canonical_id: cm.target_canonical_id})
                MERGE (p)-[c:CITES]->(lti)
                SET c.citation_mention_id = cm.citation_mention_id,
                    c.resolved_at = datetime(),
                    c.citation_type = cm.citation_type
            } IN TRANSACTIONS OF 5000 ROWS",
            ))
            .await?;
        while create_result.next().await?.is_some() {}

        let elapsed = start.elapsed().as_secs_f64();
        println!("✓ CITES materialized in {:.2}s", elapsed);

        // Verify
        let mut verify = graph
            .execute(query("MATCH ()-[r:CITES]->() RETURN count(r) AS count"))
            .await?;
        if let Some(row) = verify.next().await? {
            let new_count: i64 = row.get::<i64>("count").unwrap();
            println!("\nTotal CITES edges now: {}", new_count);
        }

        // Check for duplicates
        let mut dup_result = graph
            .execute(query(
                "MATCH ()-[r:CITES]->()
            WITH r.citation_mention_id AS cm_id, count(r) AS cnt
            WHERE cnt > 1
            RETURN sum(cnt - 1) AS duplicates",
            ))
            .await?;
        if let Some(row) = dup_result.next().await? {
            let dups: i64 = row.get::<i64>("duplicates").unwrap_or(0);
            if dups > 0 {
                println!("⚠️  Duplicates found: {}", dups);
            } else {
                println!("✓ No duplicates");
            }
        }
    }

    // Summary
    println!("\n═══════════════════════════════════════════════════════════");
    println!("CITES EDGE SUMMARY");
    println!("═══════════════════════════════════════════════════════════");

    let checks = vec![
        ("Total CITES", "MATCH ()-[r:CITES]->() RETURN count(r)"),
        (
            "CITES to LegalTextIdentity",
            "MATCH ()-[r:CITES]->(:LegalTextIdentity) RETURN count(r)",
        ),
        (
            "CITES to Provision",
            "MATCH ()-[r:CITES]->(:Provision) RETURN count(r)",
        ),
        (
            "Distinct citation_mention_ids",
            "MATCH ()-[r:CITES]->() RETURN count(DISTINCT r.citation_mention_id)",
        ),
    ];

    for (desc, cypher) in checks {
        let mut result = graph.execute(query(cypher)).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row
                .get::<i64>("count(r)")
                .or_else(|_| row.get::<i64>("count(DISTINCT r.citation_mention_id)"))
                .unwrap_or(0);
            println!("  {}: {}", desc, count);
        }
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
