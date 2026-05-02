#!/bin/bash
set -euo pipefail

sync_data() {
    if [ -z "${S3_ENDPOINT:-}" ]; then
        return
    fi

    for name in S3_BUCKET S3_ACCESS_KEY_ID S3_SECRET_ACCESS_KEY; do
        if [ -z "${!name:-}" ]; then
            echo "$name is required when S3_ENDPOINT is set" >&2
            exit 1
        fi
    done

    echo "Configuring S3 sync..."
    rclone config create railway s3 \
        provider=Other \
        env_auth=false \
        endpoint="$S3_ENDPOINT" \
        region="${S3_REGION:-auto}" \
        access_key_id="$S3_ACCESS_KEY_ID" \
        secret_access_key="$S3_SECRET_ACCESS_KEY"

    echo "Syncing data from S3 bucket..."
    rclone sync railway:"$S3_BUCKET"/data /app/data \
        --progress \
        --transfers="${RCLONE_TRANSFERS:-8}" \
        --checkers="${RCLONE_CHECKERS:-16}"
    echo "Data sync complete!"
}

rebuild_graph_from_cache() {
    if [ "${REBUILD_GRAPH:-false}" != "true" ]; then
        return
    fi

    if [ ! -d /app/data/raw/official ]; then
        echo "REBUILD_GRAPH=true but /app/data/raw/official is missing" >&2
        exit 1
    fi

    echo "Rebuilding graph JSONL from cached official HTML..."
    CHAPTERS="$(find /app/data/raw/official -maxdepth 1 -name 'ors*.html' -print \
        | sed -E 's#^.*/ors([0-9]+)\.html#\1#' \
        | sort -n \
        | paste -sd, -)"

    if [ -z "$CHAPTERS" ]; then
        echo "No cached official ORS HTML files found in /app/data/raw/official" >&2
        exit 1
    fi

    rm -rf /app/data/graph
    mkdir -p /app/data/graph

    CHUNK_SIZE="${PARSE_CHUNK_SIZE:-25}"
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
            parse_chapter_chunk "$CHAPTER_CHUNK"
            CHAPTER_CHUNK=""
            CHAPTER_COUNT=0
        fi
    done < <(printf '%s' "$CHAPTERS" | tr ',' '\n')

    if [ -n "$CHAPTER_CHUNK" ]; then
        parse_chapter_chunk "$CHAPTER_CHUNK"
    fi

    /app/ors-crawler-v0 resolve-citations \
        --graph-dir /app/data/graph \
        --edition-year "${EDITION_YEAR:-2025}"
    echo "Graph JSONL rebuild complete!"
}

parse_chapter_chunk() {
    PARSE_ARGS=(
        import-ors-cache
        --raw-dir /app/data/raw/official
        --out /app/data
        --chapters "$1"
        --edition-year "${EDITION_YEAR:-2025}"
        --append
    )
    if [ "${PARSE_FAIL_ON_QC:-false}" = "true" ]; then
        PARSE_ARGS+=(--fail-on-qc)
    fi
    /app/ors-crawler-v0 "${PARSE_ARGS[@]}" </dev/null
}

if [ "$#" -gt 0 ]; then
    exec /app/ors-crawler-v0 "$@"
fi

sync_data
rebuild_graph_from_cache

if [ ! -d /app/data/graph ]; then
    echo "/app/data/graph is missing. Set REBUILD_GRAPH=true or sync prepared graph data before seeding." >&2
    exit 1
fi

for name in NEO4J_URI NEO4J_PASSWORD; do
    if [ -z "${!name:-}" ]; then
        echo "$name is required for crawler seeding" >&2
        exit 1
    fi
done

case "${SEED_MODE:-append}" in
    skip)
        echo "SEED_MODE=skip, exiting without Neo4j changes"
        exit 0
        ;;
    append)
        ;;
    replace)
        if [ "${ORS_ALLOW_PRODUCTION_REPLACE:-false}" != "true" ]; then
            echo "SEED_MODE=replace requires ORS_ALLOW_PRODUCTION_REPLACE=true after a backup" >&2
            exit 1
        fi
        echo "Clearing Neo4j before replace seed..."
        /app/ors-crawler-v0 clear-neo4j \
            --neo4j-uri "$NEO4J_URI" \
            --neo4j-user "${NEO4J_USER:-neo4j}" \
            --neo4j-password-env NEO4J_PASSWORD \
            --batch-size "${NEO4J_CLEAR_BATCH_SIZE:-100}" \
            --yes
        echo "Neo4j clear complete!"
        ;;
    *)
        echo "Unsupported SEED_MODE=${SEED_MODE}. Use skip, append, or replace." >&2
        exit 64
        ;;
esac

echo "Seeding Neo4j in ${SEED_MODE:-append} mode..."
/app/ors-crawler-v0 seed-neo4j \
    --graph-dir /app/data/graph \
    --neo4j-uri "$NEO4J_URI" \
    --neo4j-user "${NEO4J_USER:-neo4j}" \
    --neo4j-password-env NEO4J_PASSWORD \
    --edition-year "${EDITION_YEAR:-2025}" \
    --node-batch-size "${SEED_NODE_BATCH_SIZE:-1000}" \
    --edge-batch-size "${SEED_EDGE_BATCH_SIZE:-1000}" \
    --relationship-batch-size "${SEED_RELATIONSHIP_BATCH_SIZE:-500}"
echo "Neo4j seed complete!"
