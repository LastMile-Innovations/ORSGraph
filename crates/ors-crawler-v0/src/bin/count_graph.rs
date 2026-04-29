use neo4rs::{query, Graph};
use tokio::main;

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "orsgraph2025";

    let graph = Graph::new(uri, user, password).await?;

    // Count nodes
    let mut node_result = graph
        .execute(query("MATCH (n) RETURN count(n) as count"))
        .await?;
    let node_row = node_result.next().await?.unwrap();
    let node_count: i64 = node_row.get("count").unwrap();

    // Count edges
    let mut edge_result = graph
        .execute(query("MATCH ()-[r]->() RETURN count(r) as count"))
        .await?;
    let edge_row = edge_result.next().await?.unwrap();
    let edge_count: i64 = edge_row.get("count").unwrap();

    // Count by node type
    let mut type_result = graph
        .execute(query(
            "MATCH (n) RETURN labels(n)[0] as label, count(n) as count ORDER BY count DESC",
        ))
        .await?;
    println!("\nNode counts by type:");
    while let Some(row) = type_result.next().await? {
        let label: String = row.get("label").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {}: {}", label, count);
    }

    // Count by relationship type
    let mut rel_result = graph
        .execute(query(
            "MATCH ()-[r]->() RETURN type(r) as type, count(r) as count ORDER BY count DESC",
        ))
        .await?;
    println!("\nEdge counts by type:");
    while let Some(row) = rel_result.next().await? {
        let rel_type: String = row.get("type").unwrap();
        let count: i64 = row.get("count").unwrap();
        println!("  {}: {}", rel_type, count);
    }

    println!("\nTotal nodes: {}", node_count);
    println!("Total edges: {}", edge_count);

    Ok(())
}
