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

if [ "${RUN_CRAWLER_ONLY:-false}" = "true" ]; then
    echo "RUN_CRAWLER_ONLY=true, running crawler command instead of API server"
    exec /app/ors-crawler-v0 "$@"
fi

export ORS_API_HOST="${ORS_API_HOST:-0.0.0.0}"
if [ -n "$PORT" ] && [ -z "$ORS_API_PORT" ]; then
    export ORS_API_PORT="$PORT"
fi

# Seed Neo4j if credentials are available
if [ -n "$NEO4J_URI" ] && [ -n "$NEO4J_PASSWORD" ]; then
    if [ "${REBUILD_GRAPH:-true}" = "true" ] && [ -d /app/data/raw/official ]; then
        echo "Rebuilding graph JSONL from cached official HTML..."
        CHAPTERS="$(find /app/data/raw/official -maxdepth 1 -name 'ors*.html' -print \
            | sed -E 's#^.*/ors([0-9]+)\.html#\1#' \
            | sort -n \
            | paste -sd, -)"
        if [ -n "$CHAPTERS" ]; then
            rm -rf /app/data/graph
            mkdir -p /app/data/graph

            CHUNK_SIZE="${PARSE_CHUNK_SIZE:-100}"
            CHAPTER_CHUNK=""
            CHAPTER_COUNT=0
            while IFS= read -r CHAPTER; do
                if [ -z "$CHAPTER_CHUNK" ]; then
                    CHAPTER_CHUNK="$CHAPTER"
                else
                    CHAPTER_CHUNK="$CHAPTER_CHUNK,$CHAPTER"
                fi
                CHAPTER_COUNT=$((CHAPTER_COUNT + 1))

                if [ "$CHAPTER_COUNT" -ge "$CHUNK_SIZE" ]; then
                    PARSE_ARGS=(
                        import-ors-cache
                        --raw-dir /app/data/raw/official
                        --out /app/data
                        --chapters "$CHAPTER_CHUNK"
                        --edition-year 2025
                        --append
                    )
                    if [ "${PARSE_FAIL_ON_QC:-false}" = "true" ]; then
                        PARSE_ARGS+=(--fail-on-qc)
                    fi
                    /app/ors-crawler-v0 "${PARSE_ARGS[@]}" </dev/null
                    CHAPTER_CHUNK=""
                    CHAPTER_COUNT=0
                fi
            done < <(printf '%s' "$CHAPTERS" | tr ',' '\n')

            if [ -n "$CHAPTER_CHUNK" ]; then
                PARSE_ARGS=(
                    import-ors-cache
                    --raw-dir /app/data/raw/official
                    --out /app/data
                    --chapters "$CHAPTER_CHUNK"
                    --edition-year 2025
                    --append
                )
                if [ "${PARSE_FAIL_ON_QC:-false}" = "true" ]; then
                    PARSE_ARGS+=(--fail-on-qc)
                fi
                /app/ors-crawler-v0 "${PARSE_ARGS[@]}" </dev/null
            fi

            /app/ors-crawler-v0 resolve-citations \
                --graph-dir /app/data/graph \
                --edition-year 2025
            echo "Graph JSONL rebuild complete!"
        else
            echo "No cached official ORS HTML files found; using existing graph JSONL"
        fi
    fi

    if [ "${SEED_MODE:-append}" = "replace" ]; then
        echo "Clearing Neo4j before replace seed..."
        /app/ors-crawler-v0 clear-neo4j \
            --neo4j-uri "$NEO4J_URI" \
            --neo4j-user "${NEO4J_USER:-neo4j}" \
            --neo4j-password-env NEO4J_PASSWORD \
            --yes
        echo "Neo4j clear complete!"
    fi

    echo "Seeding Neo4j..."
    /app/ors-crawler-v0 seed-neo4j \
        --graph-dir /app/data/graph \
        --neo4j-uri "$NEO4J_URI" \
        --neo4j-user "${NEO4J_USER:-neo4j}" \
        --neo4j-password-env NEO4J_PASSWORD \
        --edition-year 2025 \
        --node-batch-size "${SEED_NODE_BATCH_SIZE:-5000}" \
        --edge-batch-size "${SEED_EDGE_BATCH_SIZE:-5000}" \
        --relationship-batch-size "${SEED_RELATIONSHIP_BATCH_SIZE:-5000}"
    echo "Neo4j seed complete!"
else
    echo "Neo4j credentials not set, skipping seed"
fi

echo "Starting ORSGraph API..."
exec /app/orsgraph-api "$@"
