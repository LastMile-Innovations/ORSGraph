#!/bin/bash
set -e

# Configure rclone with Railway S3 bucket
if [ -n "$S3_ENDPOINT" ]; then
    echo "Configuring S3 sync..."
    rclone config create railway s3 \
        provider=Other \
        env_auth=false \
        endpoint="$S3_ENDPOINT" \
        region="${S3_REGION:-auto}" \
        access_key_id="$S3_ACCESS_KEY_ID" \
        secret_access_key="$S3_SECRET_ACCESS_KEY"

    echo "Syncing data from S3 bucket..."
    rclone sync railway:"$S3_BUCKET"/data /app/data --progress --transfers=8
    echo "Data sync complete!"
fi

# Seed Neo4j if credentials are available
if [ -n "$NEO4J_URI" ] && [ -n "$NEO4J_PASSWORD" ]; then
    echo "Seeding Neo4j..."
    /app/ors-crawler-v0 seed-neo4j \
        --graph-dir /app/data/graph \
        --neo4j-uri "$NEO4J_URI" \
        --neo4j-user "${NEO4J_USER:-neo4j}" \
        --neo4j-password-env NEO4J_PASSWORD \
        --edition-year 2025
    echo "Neo4j seed complete!"
else
    echo "Neo4j credentials not set, skipping seed"
    # Default: just run the crawler binary with any passed arguments
    exec /app/ors-crawler-v0 "$@"
fi
