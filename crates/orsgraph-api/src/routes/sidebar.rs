use crate::error::{ApiError, ApiResult};
use crate::models::api::{
    SaveSidebarSearchRequest, SidebarChapter, SidebarCorpus, SidebarMatter, SidebarResponse,
    SidebarSavedSearch, SidebarStatute, SidebarStatuteRequest, StatuteIndexItem,
};
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use neo4rs::{Row, query};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const SIDEBAR_CHAPTER_ITEM_LIMIT: usize = 8;
const SIDEBAR_SAVED_LIMIT: i64 = 12;
const SIDEBAR_RECENT_LIMIT: i64 = 12;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sidebar", get(get_sidebar))
        .route("/sidebar/saved-searches", post(save_sidebar_search))
        .route(
            "/sidebar/saved-searches/:saved_search_id",
            delete(delete_sidebar_search),
        )
        .route("/sidebar/saved-statutes", post(save_sidebar_statute))
        .route(
            "/sidebar/saved-statutes/:statute_id",
            delete(delete_sidebar_statute),
        )
        .route("/sidebar/recent-statutes", post(record_recent_statute))
}

async fn get_sidebar(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<SidebarResponse>> {
    let scope = sidebar_scope(&headers);
    let corpus = fetch_sidebar_corpus(&state).await?;
    let saved_searches = fetch_saved_searches(&state, &scope).await?;
    let saved_statutes = fetch_sidebar_statutes(&state, &scope, "SavedStatute", "saved_at").await?;
    let recent_statutes =
        fetch_sidebar_statutes(&state, &scope, "RecentStatute", "opened_at").await?;
    let active_matter = state
        .casebuilder_service
        .list_matters()
        .await?
        .into_iter()
        .next()
        .map(|matter| SidebarMatter {
            matter_id: matter.matter_id,
            name: matter.name,
            status: matter.status,
            updated_at: matter.updated_at,
            open_task_count: matter.open_task_count,
        });

    Ok(Json(SidebarResponse {
        corpus,
        saved_searches,
        saved_statutes,
        recent_statutes,
        active_matter,
        updated_at: now_string(),
    }))
}

async fn save_sidebar_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SaveSidebarSearchRequest>,
) -> ApiResult<Json<SidebarSavedSearch>> {
    let scope = sidebar_scope(&headers);
    let query_text = request.query.trim();
    if query_text.is_empty() {
        return Err(ApiError::BadRequest(
            "Saved search query cannot be empty".to_string(),
        ));
    }

    let query_key = query_text.to_ascii_lowercase();
    let now = now_string();
    let saved_search_id = format!("saved-search:{}:{}", slug(&scope), slug(&query_key));
    let results = request.results.unwrap_or(0) as i64;
    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "MERGE (s:SavedSearch {scope: $scope, query_key: $query_key})
                 ON CREATE SET s.saved_search_id = $saved_search_id,
                               s.created_at = $now
                 SET s.query = $query,
                     s.results = $results,
                     s.updated_at = $now
                 RETURN s.saved_search_id AS saved_search_id,
                        s.query AS query,
                        coalesce(s.results, 0) AS results,
                        s.created_at AS created_at,
                        s.updated_at AS updated_at",
            )
            .param("scope", scope)
            .param("query_key", query_key)
            .param("saved_search_id", saved_search_id)
            .param("query", query_text.to_string())
            .param("results", results)
            .param("now", now),
        )
        .await?;

    rows.into_iter()
        .next()
        .map(row_to_saved_search)
        .transpose()?
        .map(Json)
        .ok_or_else(|| ApiError::Internal("Saved search was not returned".to_string()))
}

async fn delete_sidebar_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(saved_search_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let scope = sidebar_scope(&headers);
    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "OPTIONAL MATCH (s:SavedSearch {scope: $scope})
                 WHERE s.saved_search_id = $saved_search_id
                    OR s.query_key = $saved_search_id
                 WITH collect(s) AS nodes
                 FOREACH (node IN nodes | DETACH DELETE node)
                 RETURN size(nodes) AS deleted",
            )
            .param("scope", scope)
            .param("saved_search_id", saved_search_id),
        )
        .await?;
    Ok(Json(serde_json::json!({
        "deleted": rows.first().map(|row| row_u64(row, "deleted")).unwrap_or(0) > 0
    })))
}

async fn save_sidebar_statute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SidebarStatuteRequest>,
) -> ApiResult<Json<SidebarStatute>> {
    let scope = sidebar_scope(&headers);
    let statute = resolve_sidebar_statute(&state, &request.canonical_id).await?;
    let now = now_string();
    let saved_statute_id = format!(
        "saved-statute:{}:{}",
        slug(&scope),
        slug(&statute.canonical_id)
    );
    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "MERGE (s:SavedStatute {scope: $scope, canonical_id: $canonical_id})
                 ON CREATE SET s.saved_at = $now
                 SET s.saved_statute_id = $saved_statute_id,
                     s.citation = $citation,
                     s.title = $title,
                     s.chapter = $chapter,
                     s.status = $status,
                     s.edition_year = $edition_year,
                     s.updated_at = $now
                 RETURN {
                   canonical_id: s.canonical_id,
                   citation: s.citation,
                   title: s.title,
                   chapter: s.chapter,
                   status: s.status,
                   edition_year: s.edition_year,
                   saved_at: s.saved_at
                 } AS item",
            )
            .param("scope", scope)
            .param("canonical_id", statute.canonical_id)
            .param("saved_statute_id", saved_statute_id)
            .param("citation", statute.citation)
            .param("title", statute.title)
            .param("chapter", statute.chapter)
            .param("status", statute.status)
            .param("edition_year", statute.edition_year as i64)
            .param("now", now),
        )
        .await?;

    rows.into_iter()
        .next()
        .and_then(|row| row.get::<Value>("item").ok())
        .map(|value| Json(sidebar_statute_from_value(&value)))
        .ok_or_else(|| ApiError::Internal("Saved statute was not returned".to_string()))
}

async fn delete_sidebar_statute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(statute_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let scope = sidebar_scope(&headers);
    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "OPTIONAL MATCH (s:SavedStatute {scope: $scope})
                 WHERE s.canonical_id = $statute_id
                    OR s.citation = $statute_id
                    OR s.saved_statute_id = $statute_id
                 WITH collect(s) AS nodes
                 FOREACH (node IN nodes | DETACH DELETE node)
                 RETURN size(nodes) AS deleted",
            )
            .param("scope", scope)
            .param("statute_id", statute_id),
        )
        .await?;
    Ok(Json(serde_json::json!({
        "deleted": rows.first().map(|row| row_u64(row, "deleted")).unwrap_or(0) > 0
    })))
}

async fn record_recent_statute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SidebarStatuteRequest>,
) -> ApiResult<Json<SidebarStatute>> {
    let scope = sidebar_scope(&headers);
    let statute = resolve_sidebar_statute(&state, &request.canonical_id).await?;
    let now = now_string();
    let recent_statute_id = format!(
        "recent-statute:{}:{}",
        slug(&scope),
        slug(&statute.canonical_id)
    );
    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "MERGE (r:RecentStatute {scope: $scope, canonical_id: $canonical_id})
                 SET r.recent_statute_id = $recent_statute_id,
                     r.citation = $citation,
                     r.title = $title,
                     r.chapter = $chapter,
                     r.status = $status,
                     r.edition_year = $edition_year,
                     r.opened_at = $now
                 RETURN {
                   canonical_id: r.canonical_id,
                   citation: r.citation,
                   title: r.title,
                   chapter: r.chapter,
                   status: r.status,
                   edition_year: r.edition_year,
                   opened_at: r.opened_at
                 } AS item",
            )
            .param("scope", scope.clone())
            .param("canonical_id", statute.canonical_id)
            .param("recent_statute_id", recent_statute_id)
            .param("citation", statute.citation)
            .param("title", statute.title)
            .param("chapter", statute.chapter)
            .param("status", statute.status)
            .param("edition_year", statute.edition_year as i64)
            .param("now", now),
        )
        .await?;

    state
        .neo4j_service
        .run_rows(
            query(
                "MATCH (r:RecentStatute {scope: $scope})
                 WITH r ORDER BY r.opened_at DESC
                 SKIP $limit
                 DETACH DELETE r",
            )
            .param("scope", scope)
            .param("limit", SIDEBAR_RECENT_LIMIT),
        )
        .await?;

    rows.into_iter()
        .next()
        .and_then(|row| row.get::<Value>("item").ok())
        .map(|value| Json(sidebar_statute_from_value(&value)))
        .ok_or_else(|| ApiError::Internal("Recent statute was not returned".to_string()))
}

async fn fetch_sidebar_corpus(state: &AppState) -> ApiResult<SidebarCorpus> {
    let rows = state
        .neo4j_service
        .run_rows(query(&format!(
            "MATCH (i:LegalTextIdentity)
             WITH i
             ORDER BY coalesce(i.chapter, ''), coalesce(i.citation, '')
             WITH coalesce(i.chapter, 'unknown') AS chapter,
                  count(i) AS item_count,
                  collect({{
                    canonical_id: i.canonical_id,
                    citation: i.citation,
                    title: i.title,
                    chapter: coalesce(i.chapter, 'unknown'),
                    status: coalesce(i.status, 'active'),
                    edition_year: coalesce(i.edition_year, 2025)
                  }})[0..{SIDEBAR_CHAPTER_ITEM_LIMIT}] AS items,
                  max(coalesce(i.edition_year, 2025)) AS edition_year
             WITH collect({{
                    chapter: chapter,
                    label: 'Chapter ' + chapter,
                    count: item_count,
                    items: items
                  }}) AS chapters,
                  sum(item_count) AS total,
                  max(edition_year) AS edition_year
             RETURN chapters, total, coalesce(edition_year, 2025) AS edition_year",
        )))
        .await?;

    let Some(row) = rows.first() else {
        return Err(ApiError::NotFound(
            "No sidebar corpus data found".to_string(),
        ));
    };

    let chapters = row
        .get::<Vec<Value>>("chapters")
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|value| sidebar_chapter_from_value(&value))
        .filter(|chapter| !chapter.chapter.is_empty())
        .collect();

    Ok(SidebarCorpus {
        jurisdiction: "Oregon".to_string(),
        corpus: "ORS".to_string(),
        edition_year: row_i32(row, "edition_year", 2025),
        total_statutes: row_u64(row, "total"),
        chapters,
    })
}

async fn fetch_saved_searches(state: &AppState, scope: &str) -> ApiResult<Vec<SidebarSavedSearch>> {
    state
        .neo4j_service
        .run_rows(
            query(
                "MATCH (s:SavedSearch {scope: $scope})
                 RETURN s.saved_search_id AS saved_search_id,
                        s.query AS query,
                        coalesce(s.results, 0) AS results,
                        s.created_at AS created_at,
                        s.updated_at AS updated_at
                 ORDER BY s.updated_at DESC
                 LIMIT $limit",
            )
            .param("scope", scope.to_string())
            .param("limit", SIDEBAR_SAVED_LIMIT),
        )
        .await?
        .into_iter()
        .map(row_to_saved_search)
        .collect()
}

async fn fetch_sidebar_statutes(
    state: &AppState,
    scope: &str,
    label: &str,
    timestamp_field: &str,
) -> ApiResult<Vec<SidebarStatute>> {
    let statement = format!(
        "MATCH (s:{label} {{scope: $scope}})
         RETURN {{
           canonical_id: s.canonical_id,
           citation: s.citation,
           title: s.title,
           chapter: s.chapter,
           status: s.status,
           edition_year: s.edition_year,
           {timestamp_field}: s.{timestamp_field}
         }} AS item
         ORDER BY s.{timestamp_field} DESC
         LIMIT $limit"
    );

    Ok(state
        .neo4j_service
        .run_rows(
            query(&statement)
                .param("scope", scope.to_string())
                .param("limit", SIDEBAR_SAVED_LIMIT),
        )
        .await?
        .into_iter()
        .filter_map(|row| row.get::<Value>("item").ok())
        .map(|value| sidebar_statute_from_value(&value))
        .collect())
}

async fn resolve_sidebar_statute(state: &AppState, statute_id: &str) -> ApiResult<SidebarStatute> {
    let normalized = statute_id.trim();
    if normalized.is_empty() {
        return Err(ApiError::BadRequest(
            "Statute id cannot be empty".to_string(),
        ));
    }

    let rows = state
        .neo4j_service
        .run_rows(
            query(
                "MATCH (i:LegalTextIdentity)
                 WHERE i.canonical_id = $statute_id OR i.citation = $statute_id
                 RETURN {
                   canonical_id: i.canonical_id,
                   citation: i.citation,
                   title: i.title,
                   chapter: coalesce(i.chapter, ''),
                   status: coalesce(i.status, 'active'),
                   edition_year: coalesce(i.edition_year, 2025)
                 } AS item
                 LIMIT 1",
            )
            .param("statute_id", normalized.to_string()),
        )
        .await?;

    rows.into_iter()
        .next()
        .and_then(|row| row.get::<Value>("item").ok())
        .map(|value| sidebar_statute_from_value(&value))
        .ok_or_else(|| ApiError::NotFound(format!("Statute not found: {normalized}")))
}

fn row_to_saved_search(row: Row) -> ApiResult<SidebarSavedSearch> {
    Ok(SidebarSavedSearch {
        saved_search_id: row.get("saved_search_id").unwrap_or_default(),
        query: row.get("query").unwrap_or_default(),
        results: row_u64(&row, "results"),
        created_at: row.get("created_at").unwrap_or_default(),
        updated_at: row.get("updated_at").unwrap_or_default(),
    })
}

fn sidebar_chapter_from_value(value: &Value) -> SidebarChapter {
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(statute_item_from_value).collect())
        .unwrap_or_default();

    SidebarChapter {
        chapter: value_string(value, "chapter"),
        label: value_string(value, "label"),
        count: value_u64(value, "count"),
        items,
    }
}

fn statute_item_from_value(value: &Value) -> StatuteIndexItem {
    StatuteIndexItem {
        canonical_id: value_string(value, "canonical_id"),
        citation: value_string(value, "citation"),
        title: value_optional_string(value, "title"),
        chapter: value_string(value, "chapter"),
        status: value_string_or(value, "status", "active"),
        edition_year: value_i32(value, "edition_year", 2025),
    }
}

fn sidebar_statute_from_value(value: &Value) -> SidebarStatute {
    SidebarStatute {
        canonical_id: value_string(value, "canonical_id"),
        citation: value_string(value, "citation"),
        title: value_optional_string(value, "title"),
        chapter: value_string(value, "chapter"),
        status: value_string_or(value, "status", "active"),
        edition_year: value_i32(value, "edition_year", 2025),
        saved_at: value_optional_string(value, "saved_at"),
        opened_at: value_optional_string(value, "opened_at"),
    }
}

fn sidebar_scope(headers: &HeaderMap) -> String {
    headers
        .get("x-ors-user")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|value| value.to_str().ok())
        .map(clean_scope)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "workspace".to_string())
}

fn clean_scope(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '@'))
        .take(80)
        .collect::<String>()
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 80 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed
    }
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn row_u64(row: &Row, key: &str) -> u64 {
    row.get::<i64>(key).ok().unwrap_or(0).max(0) as u64
}

fn row_i32(row: &Row, key: &str, fallback: i32) -> i32 {
    row.get::<i64>(key)
        .ok()
        .map(|value| value as i32)
        .unwrap_or(fallback)
}

fn value_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn value_string_or(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn value_optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn value_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0).max(0) as u64
}

fn value_i32(value: &Value, key: &str, fallback: i32) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|value| value as i32)
        .unwrap_or(fallback)
}
