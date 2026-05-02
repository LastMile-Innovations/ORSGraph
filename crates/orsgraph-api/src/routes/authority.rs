use crate::state::AppState;
use axum::http::{HeaderMap, HeaderValue};

const DEFAULT_SWR_SECONDS: u64 = 86_400;

pub fn authority_headers_for_state(state: &AppState, cache_status: &str) -> HeaderMap {
    build_authority_headers(
        state.corpus_release_service.release_id(),
        cache_status,
        state.config.authority_cache_ttl_seconds,
    )
}

pub fn build_authority_headers(
    release_id: &str,
    cache_status: &str,
    ttl_seconds: u64,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    insert_header(&mut headers, "x-ors-cache", cache_status);
    insert_header(&mut headers, "x-ors-corpus-release", release_id);
    insert_header(
        &mut headers,
        "cache-control",
        &format!(
            "public, max-age=0, s-maxage={}, stale-while-revalidate={}",
            ttl_seconds.max(1),
            DEFAULT_SWR_SECONDS
        ),
    );
    headers
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::build_authority_headers;

    #[test]
    fn authority_headers_include_release_cache_and_cdn_policy() {
        let headers = build_authority_headers("release:test", "origin", 120);

        assert_eq!(headers["x-ors-cache"], "origin");
        assert_eq!(headers["x-ors-corpus-release"], "release:test");
        assert_eq!(
            headers["cache-control"],
            "public, max-age=0, s-maxage=120, stale-while-revalidate=86400"
        );
    }
}
