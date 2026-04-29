use neo4rs::{query, Graph};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    println!("═══════════════════════════════════════════════════════════");
    println!("FINAL GRAPH INTEGRITY REPAIR");
    println!("═══════════════════════════════════════════════════════════\n");

    // 1. Verify CITES edges
    println!("1. VERIFYING CITES FAST TRAVERSAL EDGES");
    println!("───────────────────────────────────────────────────────────");
    let mut cites_result = graph.execute(query(
        "MATCH ()-[r]->() 
         WHERE type(r) IN ['CITES', 'CITES_VERSION', 'CITES_PROVISION', 'CITES_EXTERNAL', 'CITES_CHAPTER']
         RETURN type(r) AS rel, count(r) AS count 
         ORDER BY count DESC"
    )).await?;

    let mut found_cites = false;
    while let Some(row) = cites_result.next().await? {
        let rel: String = row.get("rel").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {}: {}", rel, count);
        if rel == "CITES" {
            found_cites = true;
        }
    }

    if !found_cites {
        println!("  ⚠️  CITES edges not found - will materialize from CitationMention");
        let start = Instant::now();
        let mut result = graph
            .execute(query(
                "CALL {
                MATCH (cm:CitationMention)
                WHERE cm.resolver_status = 'resolved' AND cm.target_canonical_id IS NOT NULL
                MATCH (source:Provision)-[:MENTIONS_CITATION]->(cm)
                MATCH (target:LegalTextIdentity {canonical_id: cm.target_canonical_id})
                MERGE (source)-[c:CITES]->(target)
                SET c.edge_id = cm.citation_mention_id,
                    c.citation_mention_id = cm.citation_mention_id,
                    c.resolved_at = datetime()
            } IN TRANSACTIONS OF 5000 ROWS",
            ))
            .await?;
        while result.next().await?.is_some() {}
        println!(
            "   ✓ CITES materialized in {:.2}s",
            start.elapsed().as_secs_f64()
        );
    }
    println!();

    // 2. Fix remaining Definition duplicate
    println!("2. FIXING DEFINITION DUPLICATES");
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
        println!("  Total: {}, Distinct: {}", total, distinct);

        if total != distinct {
            println!("  Removing {} duplicates...", total - distinct);
            let start = Instant::now();
            let mut result = graph
                .execute(query(
                    "MATCH (d:Definition)
                 WITH d.definition_id AS id, collect(d) AS nodes
                 WHERE size(nodes) > 1
                 UNWIND nodes[1..] AS dup
                 DETACH DELETE dup",
                ))
                .await?;
            while result.next().await?.is_some() {}
            println!(
                "   ✓ Duplicates removed in {:.2}s",
                start.elapsed().as_secs_f64()
            );
        } else {
            println!("  ✓ No duplicates found");
        }
    }
    println!();

    // 3. Fix LegalSemanticNode support edges
    println!("3. FIXING LEGALSEMANTICNODE SUPPORT EDGES");
    println!("───────────────────────────────────────────────────────────");
    let mut semantic_result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode)
         RETURN count(n) AS total,
                count { (n)-[:SUPPORTED_BY]->(:Provision) } AS supported_out,
                count { (:Provision)-[:EXPRESSES]->(n) } AS expressed_in",
        ))
        .await?;
    if let Some(row) = semantic_result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let supported: i64 = row.get("supported_out").unwrap();
        let expressed: i64 = row.get("expressed_in").unwrap();
        println!(
            "  Total: {}, With SUPPORTED_BY: {}, With incoming EXPRESSES: {}",
            total, supported, expressed
        );

        if expressed > supported {
            println!("  Materializing missing SUPPORTED_BY edges...");
            let start = Instant::now();
            let mut result = graph
                .execute(query(
                    "CALL {
                    MATCH (p:Provision)-[:EXPRESSES]->(n:LegalSemanticNode)
                    WHERE NOT (n)-[:SUPPORTED_BY]->(p)
                    MERGE (n)-[:SUPPORTED_BY]->(p)
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while result.next().await?.is_some() {}
            println!(
                "   ✓ SUPPORTED_BY edges added in {:.2}s",
                start.elapsed().as_secs_f64()
            );
        }
    }

    // Check labels for LegalSemanticNode without support
    let mut label_result = graph
        .execute(query(
            "MATCH (n:LegalSemanticNode)
        WHERE NOT (n)-[:SUPPORTED_BY]->(:Provision)
        RETURN labels(n) AS labels, count(*) AS count
        ORDER BY count DESC
        LIMIT 20",
        ))
        .await?;
    println!("\n  LegalSemanticNode without SUPPORTED_BY by labels:");
    while let Some(row) = label_result.next().await? {
        let labels: Vec<String> = row.get("labels").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("    {:?}: {}", labels, count);
    }
    println!();

    // 4. Fix TemporalEffect support edges
    println!("4. FIXING TEMPORALEFFECT SUPPORT EDGES");
    println!("───────────────────────────────────────────────────────────");
    let mut temporal_result = graph
        .execute(query(
            "MATCH (t:TemporalEffect)
        RETURN count(t) AS total,
               count { (t)-[:SUPPORTED_BY]->(:SourceNote) } AS by_note,
               count { (t)-[:SUPPORTED_BY]->(:Provision) } AS by_provision,
               count { (:LegalTextVersion)-[:HAS_TEMPORAL_EFFECT]->(t) } AS has_version",
        ))
        .await?;
    if let Some(row) = temporal_result.next().await? {
        let total: i64 = row.get("total").unwrap();
        let by_note: i64 = row.get("by_note").unwrap();
        let by_provision: i64 = row.get("by_provision").unwrap();
        let has_version: i64 = row.get("has_version").unwrap();
        println!("  Total: {}, SUPPORTED_BY SourceNote: {}, SUPPORTED_BY Provision: {}, HAS_TEMPORAL_EFFECT: {}", 
                 total, by_note, by_provision, has_version);

        // Materialize SUPPORTED_BY to SourceNote
        if by_note < total {
            println!("  Materializing SUPPORTED_BY -> SourceNote...");
            let start = Instant::now();
            let mut result = graph
                .execute(query(
                    "CALL {
                    MATCH (t:TemporalEffect)
                    WHERE t.source_note_id IS NOT NULL
                    MATCH (sn:SourceNote {source_note_id: t.source_note_id})
                    MERGE (t)-[:SUPPORTED_BY]->(sn)
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while result.next().await?.is_some() {}
            println!(
                "   ✓ SourceNote links added in {:.2}s",
                start.elapsed().as_secs_f64()
            );
        }

        // Materialize SUPPORTED_BY to Provision
        if by_provision < total {
            println!("  Materializing SUPPORTED_BY -> Provision...");
            let start = Instant::now();
            let mut result = graph
                .execute(query(
                    "CALL {
                    MATCH (t:TemporalEffect)
                    WHERE t.source_provision_id IS NOT NULL
                    MATCH (p:Provision {provision_id: t.source_provision_id})
                    MERGE (t)-[:SUPPORTED_BY]->(p)
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while result.next().await?.is_some() {}
            println!(
                "   ✓ Provision links added in {:.2}s",
                start.elapsed().as_secs_f64()
            );
        }

        // Materialize HAS_TEMPORAL_EFFECT from LegalTextVersion
        if has_version < total {
            println!("  Materializing LegalTextVersion -> HAS_TEMPORAL_EFFECT...");
            let start = Instant::now();
            let mut result = graph
                .execute(query(
                    "CALL {
                    MATCH (t:TemporalEffect)
                    WHERE t.version_id IS NOT NULL
                    MATCH (ltv:LegalTextVersion {version_id: t.version_id})
                    MERGE (ltv)-[:HAS_TEMPORAL_EFFECT]->(t)
                } IN TRANSACTIONS OF 5000 ROWS",
                ))
                .await?;
            while result.next().await?.is_some() {}
            println!(
                "   ✓ Version links added in {:.2}s",
                start.elapsed().as_secs_f64()
            );
        }
    }
    println!();

    // 5. Full orphan audit
    println!("5. FULL ORPHAN AUDIT");
    println!("───────────────────────────────────────────────────────────");

    let audits =
        vec![
        ("RetrievalChunk without DERIVED_FROM", 
         "MATCH (c:RetrievalChunk) WHERE NOT (c)-[:DERIVED_FROM]->() RETURN count(c)"),
        ("Provision without PART_OF_VERSION", 
         "MATCH (p:Provision) WHERE NOT (p)-[:PART_OF_VERSION]->() RETURN count(p)"),
        ("CitationMention without source Provision", 
         "MATCH (cm:CitationMention) WHERE NOT ()-[:MENTIONS_CITATION]->(cm) RETURN count(cm)"),
        ("LegalSemanticNode without SUPPORTED_BY", 
         "MATCH (n:LegalSemanticNode) WHERE NOT (n)-[:SUPPORTED_BY]->() RETURN count(n)"),
        ("Definition without DEFINES_TERM", 
         "MATCH (d:Definition) WHERE NOT (d)-[:DEFINES_TERM]->() RETURN count(d)"),
        ("Definition without HAS_SCOPE", 
         "MATCH (d:Definition) WHERE NOT (d)-[:HAS_SCOPE]->() RETURN count(d)"),
        ("Obligation without EXPRESSES", 
         "MATCH (o:Obligation) WHERE NOT ()-[:EXPRESSES]->(o) RETURN count(o)"),
        ("TemporalEffect without SUPPORTED_BY", 
         "MATCH (t:TemporalEffect) WHERE NOT (t)-[:SUPPORTED_BY]->() RETURN count(t)"),
        ("SourceNote without source", 
         "MATCH (sn:SourceNote) WHERE NOT ()-[:HAS_SOURCE_NOTE]->(sn) RETURN count(sn)"),
        ("Amendment without AFFECTS", 
         "MATCH (a:Amendment) WHERE NOT (a)-[:AFFECTS]->() RETURN count(a)"),
        ("SessionLaw without ENACTS", 
         "MATCH (sl:SessionLaw) WHERE NOT ()-[:ENACTS]->(sl) RETURN count(sl)"),
    ];

    for (desc, cypher) in audits {
        let mut result = graph.execute(query(cypher)).await?;
        if let Some(row) = result.next().await? {
            // Try each possible column name
            let count: i64 = row
                .get("count(c)")
                .or_else(|_| row.get("count(p)"))
                .or_else(|_| row.get("count(cm)"))
                .or_else(|_| row.get("count(n)"))
                .or_else(|_| row.get("count(d)"))
                .or_else(|_| row.get("count(o)"))
                .or_else(|_| row.get("count(t)"))
                .or_else(|_| row.get("count(sn)"))
                .or_else(|_| row.get("count(a)"))
                .or_else(|_| row.get("count(sl)"))
                .unwrap_or(0);
            if count > 0 {
                println!("  ⚠️  {}: {}", desc, count);
            } else {
                println!("  ✓ {}: 0", desc);
            }
        }
    }
    println!();

    // 6. Check for remaining CITES duplicates
    println!("6. CHECKING CITES EDGE DUPLICATES");
    println!("───────────────────────────────────────────────────────────");
    let mut dup_result = graph
        .execute(query(
            "MATCH ()-[r:CITES]->()
         WITH r.edge_id AS edge_id, count(r) AS cnt
         WHERE cnt > 1
         RETURN sum(cnt - 1) AS total_duplicates",
        ))
        .await?;
    if let Some(row) = dup_result.next().await? {
        let dups: i64 = row.get("total_duplicates").unwrap_or(0);
        if dups > 0 {
            println!("  ⚠️  CITES duplicates found: {}", dups);
        } else {
            println!("  ✓ No CITES duplicates");
        }
    }
    println!();

    // Final counts
    println!("7. FINAL COUNTS");
    println!("───────────────────────────────────────────────────────────");
    let mut node_result = graph
        .execute(query("MATCH (n) RETURN count(n) AS count"))
        .await?;
    if let Some(row) = node_result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Total nodes: {}", count);
    }

    let mut edge_result = graph
        .execute(query("MATCH ()-[r]->() RETURN count(r) AS count"))
        .await?;
    if let Some(row) = edge_result.next().await? {
        let count: i64 = row.get("count").unwrap();
        println!("  Total edges: {}", count);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("REPAIR COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
