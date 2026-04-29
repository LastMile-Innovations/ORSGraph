use neo4rs::{query, Graph};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("FIXING REMAINING RELATIONSHIPS");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. Fix CITES edges - create from Provision -> LegalTextIdentity via CitationMention
    println!("1. FIXING CITES EDGES");
    println!("───────────────────────────────────────────────────────────");

    // Check current state
    let mut result = graph
        .execute(query("MATCH ()-[r:CITES]->() RETURN count(r) AS count"))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Current CITES edges: {}", count);

        if count == 0 {
            println!("  Creating CITES edges from resolved citations...");
            let start = Instant::now();

            // Create CITES from Provision to LegalTextIdentity
            let mut create_result = graph
                .execute(query(
                    "CALL {
                    MATCH (p:Provision)-[:MENTIONS_CITATION]->(cm:CitationMention)
                    WHERE cm.resolver_status IN ['resolved', 'resolved_provision', 'resolved_range']
                          AND cm.target_canonical_id IS NOT NULL
                    MATCH (lti:LegalTextIdentity {canonical_id: cm.target_canonical_id})
                    MERGE (p)-[c:CITES]->(lti)
                    SET c.citation_mention_id = cm.citation_mention_id,
                        c.resolved_at = datetime()
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while create_result.next().await?.is_some() {}

            let elapsed = start.elapsed().as_secs_f64();
            println!("   ✓ CITES edges created in {:.2}s", elapsed);

            // Verify
            let mut verify = graph
                .execute(query("MATCH ()-[r:CITES]->() RETURN count(r) AS count"))
                .await?;
            if let Some(row) = verify.next().await? {
                let new_count: i64 = row.get("count").unwrap();
                println!("   Total CITES edges now: {}", new_count);
            }
        }
    }
    println!();

    // 2. Fix LegalSemanticNode SUPPORTED_BY edges
    println!("2. FIXING LEGALSEMANTICNODE SUPPORTED_BY");
    println!("───────────────────────────────────────────────────────────");

    // Check current state
    let mut result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode)
         RETURN count(n) AS total,
                count { (n)-[:SUPPORTED_BY]->() } AS supported",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let supported: i64 = row.get("supported").unwrap();
        println!("  Total: {}, With SUPPORTED_BY: {}", total, supported);

        if supported < total {
            println!("  Creating missing SUPPORTED_BY edges...");
            let start = Instant::now();

            let mut create_result = graph
                .execute(query(
                    "CALL {
                    MATCH (p:Provision)-[:EXPRESSES]->(n:LegalSemanticNode)
                    WHERE NOT (n)-[:SUPPORTED_BY]->(p)
                    MERGE (n)-[:SUPPORTED_BY]->(p)
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while create_result.next().await?.is_some() {}

            let elapsed = start.elapsed().as_secs_f64();
            println!("   ✓ SUPPORTED_BY edges created in {:.2}s", elapsed);

            // Verify
            let mut verify = graph
                .execute(query(
                    "MATCH (n:LegalSemanticNode)
                RETURN count { (n)-[:SUPPORTED_BY]->() } AS supported",
                ))
                .await?;
            if let Some(row) = verify.next().await? {
                let new_supported: i64 = row.get("supported").unwrap();
                println!("   Total SUPPORTED_BY edges now: {}", new_supported);
            }
        }
    }
    println!();

    // 3. Fix SessionLaw relationships
    println!("3. FIXING SESSIONLAW RELATIONSHIPS");
    println!("───────────────────────────────────────────────────────────");

    // Check if SessionLaw has any relationships
    let mut result = graph
        .execute(query(
            "MATCH (sl:SessionLaw)
        OPTIONAL MATCH (sl)<-[r:ENACTS|REFERENCES_SESSION_LAW|MENTIONS_SESSION_LAW]-()
        RETURN count(DISTINCT sl) AS total, count(r) AS rel_count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let rel_count: i64 = row.get("rel_count").unwrap();
        println!(
            "  Total SessionLaw: {}, Existing relationships: {}",
            total, rel_count
        );

        if rel_count == 0 {
            println!("  SessionLaw nodes have no relationships.");
            println!("  This is expected - SessionLaw is a target node enacted by Amendments.");
            println!("  Checking Amendment -> ENACTS -> SessionLaw...");

            let mut amend_check = graph
                .execute(query(
                    "MATCH (a:Amendment)
                WHERE a.session_law_id IS NOT NULL
                RETURN count(a) AS with_session_law_id",
                ))
                .await?;
            if let Some(row) = amend_check.next().await? {
                let count: i64 = row.get("with_session_law_id").unwrap();
                println!("  Amendments with session_law_id: {}", count);

                if count > 0 {
                    println!("  Creating ENACTS relationships...");
                    let start = Instant::now();

                    let mut create_result = graph
                        .execute(query(
                            "CALL {
                            MATCH (a:Amendment)
                            WHERE a.session_law_id IS NOT NULL
                            MATCH (sl:SessionLaw {session_law_id: a.session_law_id})
                            MERGE (a)-[:ENACTS]->(sl)
                        } IN TRANSACTIONS OF 5000 ROWS",
                        ))
                        .await?;
                    while create_result.next().await?.is_some() {}

                    let elapsed = start.elapsed().as_secs_f64();
                    println!("   ✓ ENACTS edges created in {:.2}s", elapsed);
                }
            }
        }
    }
    println!();

    // 4. Final verification
    println!("4. FINAL VERIFICATION");
    println!("───────────────────────────────────────────────────────────");

    let mut result = graph
        .execute(query(
            "MATCH (n)
        RETURN count(n) AS nodes",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let nodes: i64 = row.get("nodes").unwrap();
        println!("  Total nodes: {}", nodes);
    }

    let mut result = graph
        .execute(query(
            "MATCH ()-[r]->()
        RETURN count(r) AS edges",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let edges: i64 = row.get("edges").unwrap();
        println!("  Total edges: {}", edges);
    }

    let mut result = graph
        .execute(query("MATCH ()-[r:CITES]->() RETURN count(r) AS count"))
        .await?;
    if let Some(row) = result.next().await? {
        let cites: i64 = row.get("count").unwrap();
        println!("  CITES edges: {}", cites);
    }

    let mut result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode) RETURN count { (n)-[:SUPPORTED_BY]->() } AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let supported: i64 = row.get("count").unwrap();
        println!("  LegalSemanticNode SUPPORTED_BY: {}", supported);
    }

    let mut result = graph
        .execute(query(
            "MATCH ()-[r:ENACTS]->(:SessionLaw) RETURN count(r) AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let enacts: i64 = row.get("count").unwrap();
        println!("  Amendment ENACTS SessionLaw: {}", enacts);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("FIX COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
