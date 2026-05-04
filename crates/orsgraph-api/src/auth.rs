use crate::config::ApiConfig;
use crate::error::{ApiError, ApiResult};
use axum::http::{HeaderMap, Method, header};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const JWKS_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKind {
    Anonymous,
    Service,
    User,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub kind: AuthKind,
    pub subject: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub roles: BTreeSet<String>,
}

impl AuthContext {
    pub fn anonymous() -> Self {
        Self {
            kind: AuthKind::Anonymous,
            subject: None,
            email: None,
            name: None,
            roles: BTreeSet::new(),
        }
    }

    pub fn service() -> Self {
        Self {
            kind: AuthKind::Service,
            subject: Some("service:api-key".to_string()),
            email: None,
            name: Some("Service API key".to_string()),
            roles: BTreeSet::new(),
        }
    }

    fn user(claims: ZitadelClaims) -> Self {
        Self {
            kind: AuthKind::User,
            subject: Some(claims.sub),
            email: claims.email,
            name: claims.name.or(claims.preferred_username),
            roles: roles_from_claims(&claims.extra),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        !matches!(self.kind, AuthKind::Anonymous)
    }

    pub fn is_service(&self) -> bool {
        matches!(self.kind, AuthKind::Service)
    }

    pub fn is_admin(&self, admin_role: &str) -> bool {
        self.is_service()
            || (!admin_role.trim().is_empty() && self.roles.contains(admin_role.trim()))
    }

    pub fn subject(&self) -> ApiResult<&str> {
        self.subject
            .as_deref()
            .filter(|subject| !subject.trim().is_empty())
            .ok_or(ApiError::Unauthorized)
    }
}

#[derive(Debug)]
pub struct AuthVerifier {
    issuer: String,
    audience: Option<String>,
    client: reqwest::Client,
    discovery: RwLock<Option<CachedDiscovery>>,
    jwks: RwLock<Option<CachedJwks>>,
}

#[derive(Debug, Clone)]
struct CachedDiscovery {
    jwks_uri: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct CachedJwks {
    keys: Vec<Jwk>,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct OpenIdConfiguration {
    jwks_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    alg: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZitadelClaims {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl AuthVerifier {
    pub fn from_config(config: &ApiConfig) -> ApiResult<Option<Self>> {
        if !config.auth_enabled {
            return Ok(None);
        }

        let issuer = config
            .auth_issuer
            .as_deref()
            .map(str::trim)
            .filter(|issuer| !issuer.is_empty())
            .ok_or_else(|| {
                ApiError::Config(config::ConfigError::Message(
                    "ORS_AUTH_ISSUER is required when ORS_AUTH_ENABLED=true".to_string(),
                ))
            })?
            .trim_end_matches('/')
            .to_string();

        Ok(Some(Self {
            issuer,
            audience: config
                .auth_audience
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            client: reqwest::Client::new(),
            discovery: RwLock::new(None),
            jwks: RwLock::new(None),
        }))
    }

    pub async fn verify_bearer(&self, token: &str) -> ApiResult<AuthContext> {
        let header = decode_header(token).map_err(|error| {
            ApiError::Unauthorized.with_log(format!("JWT header error: {error}"))
        })?;
        let kid = header.kid.as_deref();
        let jwk = self
            .jwk_for_kid(kid)
            .await?
            .ok_or_else(|| ApiError::Unauthorized.with_log("No matching JWT key".to_string()))?;
        let key = decoding_key(&jwk)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        if let Some(audience) = self.audience.as_deref() {
            validation.set_audience(&[audience]);
        } else {
            validation.validate_aud = false;
        }

        let data = decode::<ZitadelClaims>(token, &key, &validation).map_err(|error| {
            ApiError::Unauthorized.with_log(format!("JWT validation error: {error}"))
        })?;
        Ok(AuthContext::user(data.claims))
    }

    async fn discovery(&self) -> ApiResult<String> {
        if let Some(cached) = self.discovery.read().await.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.jwks_uri.clone());
            }
        }

        let url = format!("{}/.well-known/openid-configuration", self.issuer);
        let discovery = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<OpenIdConfiguration>()
            .await?;
        let jwks_uri = discovery.jwks_uri;
        *self.discovery.write().await = Some(CachedDiscovery {
            jwks_uri: jwks_uri.clone(),
            expires_at: Instant::now() + JWKS_CACHE_TTL,
        });
        Ok(jwks_uri)
    }

    async fn jwk_for_kid(&self, kid: Option<&str>) -> ApiResult<Option<Jwk>> {
        let keys = self.jwks().await?;
        Ok(keys.into_iter().find(|key| {
            let key_id_matches = match kid {
                Some(kid) => key.kid.as_deref() == Some(kid),
                None => true,
            };
            key_id_matches
                && key.kty == "RSA"
                && key.alg.as_deref().is_none_or(|alg| alg == "RS256")
        }))
    }

    async fn jwks(&self) -> ApiResult<Vec<Jwk>> {
        if let Some(cached) = self.jwks.read().await.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.keys.clone());
            }
        }

        let jwks_uri = self.discovery().await?;
        let jwks = self
            .client
            .get(jwks_uri)
            .send()
            .await?
            .error_for_status()?
            .json::<Jwks>()
            .await?;
        *self.jwks.write().await = Some(CachedJwks {
            keys: jwks.keys.clone(),
            expires_at: Instant::now() + JWKS_CACHE_TTL,
        });
        Ok(jwks.keys)
    }
}

pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    header_value(headers, header::AUTHORIZATION.as_str())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

pub fn is_public_path(method: &Method, path: &str) -> bool {
    path == "/"
        || path == "/api/v1/health"
        || (path == "/api/v1/auth/access-request" && *method == Method::POST)
        || (path.starts_with("/api/v1/auth/invites/")
            && !path.ends_with("/accept")
            && *method == Method::GET)
        || is_public_authority_read(method, path)
}

fn is_public_authority_read(method: &Method, path: &str) -> bool {
    if !matches!(*method, Method::GET | Method::HEAD) {
        return false;
    }

    matches!(
        path,
        "/api/v1/home"
            | "/api/v1/featured-statutes"
            | "/api/v1/analytics/home"
            | "/api/v1/stats"
            | "/api/v1/search"
            | "/api/v1/search/open"
            | "/api/v1/search/suggest"
            | "/api/v1/sources"
            | "/api/v1/statutes"
            | "/api/v1/graph/neighborhood"
    ) || path.starts_with("/api/v1/sources/")
        || path.starts_with("/api/v1/statutes/")
        || path.starts_with("/api/v1/provisions/")
        || path.starts_with("/api/v1/rules/")
}

pub fn is_auth_access_bootstrap_path(method: &Method, path: &str) -> bool {
    (path == "/api/v1/auth/me" && *method == Method::GET)
        || (path.starts_with("/api/v1/auth/invites/")
            && path.ends_with("/accept")
            && *method == Method::POST)
}

pub fn is_admin_operation(_method: &Method, path: &str) -> bool {
    path.starts_with("/api/v1/admin") || path.starts_with("/api/v1/qc")
}

pub fn matter_id_from_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/api/v1/matters/")?;
    let raw = rest.split('/').next()?.trim();
    if raw.is_empty() {
        return None;
    }
    percent_decode_str(raw)
        .decode_utf8()
        .ok()
        .map(|value| value.to_string())
}

fn decoding_key(jwk: &Jwk) -> ApiResult<DecodingKey> {
    let n = jwk
        .n
        .as_deref()
        .ok_or_else(|| ApiError::Unauthorized.with_log("JWT key missing modulus".to_string()))?;
    let e = jwk
        .e
        .as_deref()
        .ok_or_else(|| ApiError::Unauthorized.with_log("JWT key missing exponent".to_string()))?;
    DecodingKey::from_rsa_components(n, e)
        .map_err(|error| ApiError::Unauthorized.with_log(format!("JWT key error: {error}")))
}

fn roles_from_claims(extra: &BTreeMap<String, Value>) -> BTreeSet<String> {
    let mut roles = BTreeSet::new();
    for (key, value) in extra {
        if is_role_claim_key(key) {
            collect_roles(value, &mut roles);
        }
    }
    roles
}

fn is_role_claim_key(key: &str) -> bool {
    key == "roles"
        || key == "role"
        || key == "urn:iam:org:project:roles"
        || key == "urn:zitadel:iam:org:project:roles"
        || (key.starts_with("urn:zitadel:iam:org:project:") && key.ends_with(":roles"))
}

fn collect_roles(value: &Value, roles: &mut BTreeSet<String>) {
    match value {
        Value::String(role) => {
            if !role.trim().is_empty() {
                roles.insert(role.trim().to_string());
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_roles(value, roles);
            }
        }
        Value::Object(values) => {
            for key in values.keys() {
                if !key.trim().is_empty() {
                    roles.insert(key.trim().to_string());
                }
            }
        }
        _ => {}
    }
}

trait UnauthorizedLog {
    fn with_log(self, message: String) -> Self;
}

impl UnauthorizedLog for ApiError {
    fn with_log(self, message: String) -> Self {
        tracing::warn!("{message}");
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{is_auth_access_bootstrap_path, is_public_path, roles_from_claims};
    use axum::http::Method;
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn auth_access_public_paths_are_method_scoped() {
        assert!(is_public_path(&Method::POST, "/api/v1/auth/access-request"));
        assert!(is_public_path(&Method::GET, "/api/v1/auth/invites/token"));
        assert!(!is_public_path(&Method::GET, "/api/v1/auth/access-request"));
        assert!(!is_public_path(
            &Method::POST,
            "/api/v1/auth/invites/token/accept"
        ));
    }

    #[test]
    fn public_authority_reads_are_method_scoped_and_private_routes_stay_private() {
        for path in [
            "/api/v1/home",
            "/api/v1/stats",
            "/api/v1/search",
            "/api/v1/search/open",
            "/api/v1/statutes",
            "/api/v1/statutes/or:ors:90.320/page",
            "/api/v1/provisions/or:ors:90.320:1",
            "/api/v1/graph/neighborhood",
            "/api/v1/rules/applicable",
        ] {
            assert!(
                is_public_path(&Method::GET, path),
                "{path} should be a public GET"
            );
            assert!(
                is_public_path(&Method::HEAD, path),
                "{path} should be a public HEAD"
            );
            assert!(
                !is_public_path(&Method::POST, path),
                "{path} should not be public for POST"
            );
        }

        for path in [
            "/api/v1/ask",
            "/api/v1/admin",
            "/api/v1/sidebar",
            "/api/v1/graph/full",
            "/api/v1/graph/path",
            "/api/v1/matters/demo",
            "/api/v1/casebuilder/webhooks/assemblyai",
        ] {
            assert!(
                !is_public_path(&Method::GET, path),
                "{path} should remain private"
            );
        }
    }

    #[test]
    fn auth_access_bootstrap_paths_allow_authenticated_pending_users() {
        assert!(is_auth_access_bootstrap_path(
            &Method::GET,
            "/api/v1/auth/me"
        ));
        assert!(is_auth_access_bootstrap_path(
            &Method::POST,
            "/api/v1/auth/invites/token/accept"
        ));
        assert!(!is_auth_access_bootstrap_path(
            &Method::GET,
            "/api/v1/matters"
        ));
    }

    #[test]
    fn zitadel_project_scoped_role_claims_return_only_role_keys() {
        let mut claims = BTreeMap::new();
        claims.insert(
            "urn:zitadel:iam:org:project:371183997394536814:roles".to_string(),
            json!({
                "orsgraph_admin": {
                    "371183997394471278": "lastmile.example"
                },
                "reviewer": {
                    "371183997394471278": "lastmile.example"
                }
            }),
        );

        assert_eq!(
            roles_from_claims(&claims),
            BTreeSet::from(["orsgraph_admin".to_string(), "reviewer".to_string()])
        );
    }

    #[test]
    fn auth_role_claims_keep_legacy_shapes() {
        let mut claims = BTreeMap::new();
        claims.insert("role".to_string(), json!("owner"));
        claims.insert("roles".to_string(), json!(["editor"]));
        claims.insert(
            "urn:iam:org:project:roles".to_string(),
            json!({
                "legacy_admin": {
                    "371183997394471278": "lastmile.example"
                }
            }),
        );
        claims.insert(
            "urn:zitadel:iam:org:project:roles".to_string(),
            json!({
                "project_admin": {
                    "371183997394471278": "lastmile.example"
                }
            }),
        );

        assert_eq!(
            roles_from_claims(&claims),
            BTreeSet::from([
                "editor".to_string(),
                "legacy_admin".to_string(),
                "owner".to_string(),
                "project_admin".to_string(),
            ])
        );
    }
}
