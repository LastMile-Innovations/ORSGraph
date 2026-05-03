use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use jsonwebtoken::{
    Algorithm, EncodingKey, Header, encode,
    jwk::{Jwk, JwkSet},
};
use orsgraph_mcp::{
    OrsGraphApiClient,
    streamable_http::{
        JwtAuthRuntimeConfig, RateLimitRuntimeConfig, StreamableHttpRuntimeConfig,
        streamable_http_router,
    },
};
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn streamable_http_requires_bearer_when_configured() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(api_base_url, Some("secret".to_string()), None).await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_rejects_disallowed_origin() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(
        api_base_url,
        Some("secret".to_string()),
        Some(vec!["http://allowed.example".to_string()]),
    )
    .await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("authorization", "Bearer secret")
        .header("origin", "http://evil.example")
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_initializes_session() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(api_base_url, Some("secret".to_string()), None).await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("authorization", "Bearer secret")
        .header("origin", server.base_url.clone())
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("mcp-session-id").is_some());
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("text/event-stream"))
    );

    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_rate_limits_mcp_but_not_healthz() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http_with_config(api_base_url, |config| {
        config.rate_limit = Some(RateLimitRuntimeConfig {
            requests: 1,
            per: Duration::from_secs(60),
        });
    })
    .await?;
    let client = reqwest::Client::new();

    let first = post_initialize(&client, &server).await?;
    assert_eq!(first.status(), StatusCode::OK);

    let limited = post_initialize(&client, &server).await?;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(limited.headers().get("retry-after").is_some());

    let healthz = client
        .get(format!("{}/healthz", server.base_url))
        .send()
        .await?;
    assert_eq!(healthz.status(), StatusCode::OK);
    let healthz = healthz.json::<Value>().await?;
    assert_eq!(healthz["rate_limit_enabled"], true);
    assert_eq!(healthz["rate_limit_requests"], 1);

    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_jwt_auth_validates_audience_and_exposes_metadata() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let issuer = spawn_test_issuer().await?;
    let mut jwt = JwtAuthRuntimeConfig {
        issuer: issuer.base_url.clone(),
        audience: "https://mcp.example.com/mcp".to_string(),
        jwks_uri: Some(format!("{}/jwks", issuer.base_url)),
        required_scopes: vec!["orsgraph:mcp".to_string()],
    };
    let server = spawn_mcp_http_with_config(api_base_url, |config| {
        jwt.issuer = issuer.base_url.clone();
        config.jwt_auth = Some(jwt);
        config.oauth_resource = Some("https://mcp.example.com/mcp".to_string());
        config.oauth_authorization_servers = vec![issuer.base_url.clone()];
        config.oauth_scopes = vec!["orsgraph:mcp".to_string()];
    })
    .await?;

    let metadata = reqwest::Client::new()
        .get(format!(
            "{}/.well-known/oauth-protected-resource",
            server.base_url
        ))
        .send()
        .await?;
    assert_eq!(metadata.status(), StatusCode::OK);
    let metadata = metadata.json::<Value>().await?;
    assert_eq!(metadata["resource"], "https://mcp.example.com/mcp");
    assert_eq!(metadata["authorization_servers"][0], issuer.base_url);
    assert_eq!(metadata["scopes_supported"][0], "orsgraph:mcp");

    let missing = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("origin", server.base_url.clone())
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    assert!(
        missing
            .headers()
            .get("www-authenticate")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("resource_metadata="))
    );

    let wrong_audience_token = issuer.token("https://wrong.example/mcp", "orsgraph:mcp")?;
    let wrong_audience = post_initialize_with_token(&server, &wrong_audience_token).await?;
    assert_eq!(wrong_audience.status(), StatusCode::UNAUTHORIZED);

    let missing_scope_token = issuer.token("https://mcp.example.com/mcp", "other:scope")?;
    let missing_scope = post_initialize_with_token(&server, &missing_scope_token).await?;
    assert_eq!(missing_scope.status(), StatusCode::FORBIDDEN);

    let token = issuer.token("https://mcp.example.com/mcp", "orsgraph:mcp")?;
    let authorized = post_initialize_with_token(&server, &token).await?;
    assert_eq!(authorized.status(), StatusCode::OK);
    assert!(authorized.headers().get("mcp-session-id").is_some());

    server.shutdown();
    issuer.shutdown();
    Ok(())
}

fn initialize_payload() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "orsgraph-mcp-http-test",
                "version": "0.1.0"
            }
        }
    })
}

struct TestHttpServer {
    base_url: String,
    cancellation_token: CancellationToken,
    handle: JoinHandle<()>,
}

impl TestHttpServer {
    fn shutdown(self) {
        self.cancellation_token.cancel();
        self.handle.abort();
    }
}

async fn spawn_mcp_http(
    api_base_url: String,
    bearer_token: Option<String>,
    allowed_origins: Option<Vec<String>>,
) -> anyhow::Result<TestHttpServer> {
    spawn_mcp_http_with_config(api_base_url, |config| {
        config.bearer_token = bearer_token;
        if let Some(allowed_origins) = allowed_origins {
            config.allowed_origins = allowed_origins;
        }
    })
    .await
}

async fn spawn_mcp_http_with_config(
    api_base_url: String,
    configure: impl FnOnce(&mut StreamableHttpRuntimeConfig),
) -> anyhow::Result<TestHttpServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let api = OrsGraphApiClient::new(api_base_url, Duration::from_secs(3))
        .map_err(|error| anyhow::anyhow!(error))?;
    let mut config = StreamableHttpRuntimeConfig::local_default().with_bind(addr);
    configure(&mut config);

    let cancellation_token = CancellationToken::new();
    let router = streamable_http_router(api, config, cancellation_token.child_token())
        .map_err(|error| anyhow::anyhow!(error))?;
    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mcp http server");
    });

    Ok(TestHttpServer {
        base_url: format!("http://{addr}"),
        cancellation_token,
        handle,
    })
}

async fn post_initialize_with_token(
    server: &TestHttpServer,
    token: &str,
) -> anyhow::Result<reqwest::Response> {
    Ok(reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .header("origin", server.base_url.clone())
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?)
}

async fn post_initialize(
    client: &reqwest::Client,
    server: &TestHttpServer,
) -> anyhow::Result<reqwest::Response> {
    Ok(client
        .post(format!("{}/mcp", server.base_url))
        .header("origin", server.base_url.clone())
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?)
}

#[derive(Clone)]
struct TestIssuerState {
    base_url: String,
    jwks: JwkSet,
}

struct TestIssuer {
    base_url: String,
    encoding_key: EncodingKey,
    cancellation_token: CancellationToken,
    handle: JoinHandle<()>,
}

impl TestIssuer {
    fn token(&self, audience: &str, scope: &str) -> anyhow::Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;
        let claims = TestClaims {
            iss: self.base_url.clone(),
            sub: "user:test".to_string(),
            aud: audience.to_string(),
            iat: now,
            exp: now + 300,
            scope: scope.to_string(),
        };
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key".to_string());
        Ok(encode(&header, &claims, &self.encoding_key)?)
    }

    fn shutdown(self) {
        self.cancellation_token.cancel();
        self.handle.abort();
    }
}

#[derive(Debug, Serialize)]
struct TestClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: usize,
    exp: usize,
    scope: String,
}

async fn spawn_test_issuer() -> anyhow::Result<TestIssuer> {
    async fn openid(State(state): State<TestIssuerState>) -> Json<Value> {
        Json(json!({
            "issuer": state.base_url,
            "jwks_uri": format!("{}/jwks", state.base_url),
            "authorization_endpoint": format!("{}/authorize", state.base_url),
            "token_endpoint": format!("{}/token", state.base_url),
            "code_challenge_methods_supported": ["S256"]
        }))
    }

    async fn jwks(State(state): State<TestIssuerState>) -> Json<JwkSet> {
        Json(state.jwks)
    }

    let encoding_key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY.as_bytes())?;
    let mut jwk = Jwk::from_encoding_key(&encoding_key, Algorithm::RS256)?;
    jwk.common.key_id = Some("test-key".to_string());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");
    let state = TestIssuerState {
        base_url: base_url.clone(),
        jwks: JwkSet { keys: vec![jwk] },
    };
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(openid))
        .route("/jwks", get(jwks))
        .with_state(state);
    let cancellation_token = CancellationToken::new();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test issuer server");
    });

    Ok(TestIssuer {
        base_url,
        encoding_key,
        cancellation_token,
        handle,
    })
}

const TEST_RSA_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDDnH6yP8ClXU7E
ad4b9z3BQdnnbFgHE9KBc+U3nl8D3doTFI0EjPsBIb3deDnb1y83RjtFlMWoRq3V
WW8HJ3S06dwNDiqtr3EnqIpSqBrdHOCu1L8MxwhSqmzrYvUBiUhHC2IfZR2nQokt
8Ao6N4kkRpcuoMGb3Yq2AN6T53jvh0Oenfqaut+YpooEjSVuXTmKzWhCMouZwvlQ
4vGcsSmJxumi85rIRskjsAYgGluDS82qfcOiFd9dU0dSVbS7OEumvrcpZQG0z6NN
UA1N8aPeslbj9XiBKs9AmTEduy1l/oI4fvlqFiXPlYAr0rRYJWeVt2Rh6dCMu7HE
kAwKczR9AgMBAAECggEAJaOew2i+BnPKXMPsP6BW6zFjHwM7ev0m887bq9SW/tT5
CdPaAKaqA8E82fkw1Or2hGHItO5YDDWxbEydrxg5/jfDpmVz8+C+2r01BIuhQ6uz
ViDMtEY2BUYS2EY907JZTIZVtqnLx2vnnoXCSgp2oprMq2W2a2n5L2VCbt5K6Hfd
5VlBm0MiUh8BDIP0PYMzzuAuDIaq4VNvjGdqEdNy0XyBasRZnGbAVrDKspJcfy1d
UQsf20vvWbduu02D9iJU23J7aznPa6u1v2hw5SSsoLecPd8hDVVml4B06xs90bb0
bP9TxG9yNysJIIxPG6qPOEyRg3ZisvWUAI6RSVifZwKBgQDqoW1Nc4US5Hr0xdPy
817HBt0J0rTIe7FzzkJ4l/RgTYPfHTvjvpLLhQH/leoIMQNCoDSoIyEChRrqqCdh
bReLzcEpXZVIoREAemoZhhDnLY/Eo89RsODudJeInqiRQIBtNXn9Id0AqEE12AtW
NZ8IWzgJseAx0q3TAEAogM+sawKBgQDVbU6UejwHu4r7YGwZxn4pZCifi5pl4yj2
B36h27rf+SwSsBG9f58ghIUXflebCt5urAqf1jZjzK8HpC1YepHs0/WMTMF9QxAF
xsPM9EtHavtPlX7BQDI/8pD95kRrORjmvNhza7GvyzsSMc6fOqN3CKTFnsDzca/w
uua+zjnctwKBgQDo85U+DK/W7hpV5ARndtJm8J2NHzJ2yriIrgS5DsWGx+9iGfhY
SeIdRtWRGRrfPyppf/5H4Xjos5bh3EodJN734zUhCNUq4x+qReAJr14g6M+RAMLZ
7K+mkQPSlRPwAwZ/Z1TSykhOWr9D4lh/I4XeGhMtLPnW/cGveNQ6YonOOQKBgQCw
4a2NepzD39e4rFoLJqmqzjqot7+Xj6OD/AQkSwJe25h+SHP7dIjCH3JaXThn46Mj
X+xSOevL3Hh3QcbfHH5SI/zOcVKu6OSflPLzqse1AeIUPBbYPMconnUyKCQuJR0R
JhPR3MBfrHRhiOvwpV2SqpQ8wGyzllY1kgDow+vKkwKBgQCuy+JPGMgB6UwtgCqJ
AFGuw6TsnLz6qgMW738lNWo7L2J0mPx3+WkMAczEnkTE8TDuLr1RNdqpgX5zVch0
FgKefkuGyWEWjn6JWJ+2jpB/8Aorc1j34h7LH6QeKgBbIhSrxKltf/APL72UN3mS
PRh9h/x6ZFmlHMa71o2Vw5K5wQ==
-----END PRIVATE KEY-----"#;

async fn spawn_test_api() -> anyhow::Result<String> {
    async fn health() -> Json<Value> {
        Json(json!({
            "ok": true,
            "service": "orsgraph-api",
            "neo4j": "connected",
            "version": "test"
        }))
    }

    async fn search(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
        Json(json!({
            "query": params.get("q").cloned().unwrap_or_default(),
            "mode": "auto",
            "total": 1,
            "limit": params.get("limit").and_then(|value| value.parse::<u32>().ok()).unwrap_or(10),
            "results": [{
                "id": "or:ors:90.100",
                "citation": "ORS 90.100",
                "title": "Definitions",
                "snippet": "Test fixture"
            }]
        }))
    }

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/search", get(search));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test API server");
    });

    Ok(format!("http://{addr}/api/v1"))
}
