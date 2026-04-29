use neo4rs::{query, Graph};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("DEBUGGING DATA ISSUES");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. Check CitationMention resolver_status values
    println!("1. CITATIONMENTION RESOLVER STATUS VALUES");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (cm:CitationMention)
         RETURN cm.resolver_status AS status, count(*) AS count
         ORDER BY count DESC
         LIMIT 10",
        ))
        .await?;
    while let Some(row) = result.next().await? {
        let status: Option<String> = row.get("status").ok();
        let count: i64 = row.get("count").unwrap();
        println!("  {:?}: {}", status, count);
    }
    println!();

    // 2. Check if CitationMention has target_canonical_id
    println!("2. CITATIONMENTION WITH target_canonical_id");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (cm:CitationMention)
         WHERE cm.target_canonical_id IS NOT NULL
         RETURN count(*) AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  With target_canonical_id: {}", count);
    }

    // Check sample
    let mut result = graph.execute(query(
        "MATCH (cm:CitationMention)
         WHERE cm.target_canonical_id IS NOT NULL
         RETURN cm.citation_mention_id AS id, cm.target_canonical_id AS target, cm.resolver_status AS status
         LIMIT 3"
    )).await?;
    while let Some(row) = result.next().await? {
        let id: String = row.get("id").unwrap();
        let target: String = row.get("target").unwrap();
        let status: Option<String> = row.get("status").ok();
        println!("  {} -> {} (status: {:?})", id, target, status);
    }
    println!();

    // 3. Check Provision -> MENTIONS_CITATION -> CitationMention
    println!("3. PROVISION -> MENTIONS_CITATION -> CITATIONMENTION");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (p:Provision)-[:MENTIONS_CITATION]->(cm:CitationMention)
         RETURN count(*) AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Total MENTIONS_CITATION edges: {}", count);
    }
    println!();

    // 4. Check if LegalTextIdentity exists with matching canonical_id
    println!("4. LEGALTEXTIDENTITY AVAILABILITY");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (cm:CitationMention)
         WHERE cm.target_canonical_id IS NOT NULL
         WITH cm.target_canonical_id AS target_id
         LIMIT 5
         MATCH (lti:LegalTextIdentity {canonical_id: target_id})
         RETURN target_id, count(lti) AS found",
        ))
        .await?;
    let mut found_matches = 0;
    while let Some(row) = result.next().await? {
        let target: String = row.get("target_id").unwrap();
        let found: i64 = row.get("found").unwrap();
        if found > 0 {
            found_matches += 1;
        }
        println!("  {}: {} matches", target, found);
    }
    if found_matches == 0 {
        println!("  ⚠️ No matching LegalTextIdentity nodes found!");
    }
    println!();

    // 5. Check LegalSemanticNode - Provision connection via EXPRESSES
    println!("5. LEGALSEMANTICNODE - PROVISION CONNECTION");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (p:Provision)-[:EXPRESSES]->(n:LegalSemanticNode)
         RETURN count(*) AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Provision EXPRESSES LegalSemanticNode: {}", count);
    }

    // Check if reverse exists
    let mut result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode)-[:SUPPORTED_BY]->(p:Provision)
         RETURN count(*) AS count",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  LegalSemanticNode SUPPORTED_BY Provision: {}", count);
    }
    println!();

    // 6. Check Amendment -> SessionLaw via session_law_id
    println!("6. AMENDMENT -> SESSIONLAW CONNECTION");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (a:Amendment)
         WHERE a.session_law_id IS NOT NULL
         RETURN count(*) AS with_id",
        ))
        .await?;
    if let Some(row) = result.next().await? {
        let count: i64 = row.get("with_id").unwrap();
        println!("  Amendments with session_law_id: {}", count);
    }

    // Check if matching SessionLaw exists
    let mut result = graph
        .execute(query(
            "MATCH (a:Amendment)
         WHERE a.session_law_id IS NOT NULL
         WITH a.session_law_id AS sid
         LIMIT 5
         MATCH (sl:SessionLaw {session_law_id: sid})
         RETURN sid, count(sl) AS found",
        ))
        .await?;
    let mut found_matches = 0;
    while let Some(row) = result.next().await? {
        let sid: String = row.get("sid").unwrap();
        let found: i64 = row.get("found").unwrap();
        if found > 0 {
            found_matches += 1;
        }
        println!("  {}: {} SessionLaw matches", sid, found);
    }
    if found_matches == 0 {
        println!("  ⚠️ No matching SessionLaw nodes found!");
    }
    println!();

    // 7. Check SessionLaw relationships more carefully
    println!("7. SESSIONLAW RELATIONSHIP DETAILS");
    println!("───────────────────────────────────────────────────────────");
    let mut result = graph
        .execute(query(
            "MATCH (sl:SessionLaw)<-[r]-(n)
         RETURN type(r) AS rel, labels(n) AS source_labels, count(*) AS count
         ORDER BY count DESC
         LIMIT 10",
        ))
        .await?;
    while let Some(row) = result.next().await? {
        let rel: String = row.get("rel").unwrap();
        let labels: Vec<String> = row.get("source_labels").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {:?} -> {}: {}", labels, rel, count);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("DEBUG COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
