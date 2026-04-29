use anyhow::Result;
use neo4rs::{query, ConfigBuilder, Graph};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "orsgraph2025".to_string());

    let config = ConfigBuilder::default()
        .uri(uri)
        .user(user)
        .password(pass)
        .build()?;
    let graph = Graph::connect(config).await?;

    println!("Checking APOC availability...");
    let q = "CALL apoc.help('apoc.create.addLabels')";
    match graph.execute(query(q)).await {
        Ok(mut res) => {
            if let Some(row) = res.next().await? {
                let name: String = row.get("name")?;
                println!("✓ APOC is available: {}", name);
            } else {
                println!("✗ APOC is NOT available (no rows returned).");
            }
        }
        Err(e) => {
            println!("✗ APOC is NOT available: {}", e);
        }
    }

    Ok(())
}
