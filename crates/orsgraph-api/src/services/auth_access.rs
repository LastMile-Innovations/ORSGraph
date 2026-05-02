use crate::auth::AuthContext;
use crate::error::{ApiError, ApiResult};
use crate::models::auth_access::{
    AuthMeResponse, BetaInvite, CreateAccessRequest, CreateAccessRequestResponse,
    CreateInviteRequest, CreateInviteResponse, InviteLookupResponse, InviteRequest, UserProfile,
};
use crate::services::neo4j::Neo4jService;
use neo4rs::{Row, query};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DEFAULT_INVITE_TTL_DAYS: u64 = 14;
const MAX_NOTE_CHARS: usize = 1200;

pub struct AuthAccessService {
    neo4j: Arc<Neo4jService>,
}

impl AuthAccessService {
    pub fn new(neo4j: Arc<Neo4jService>) -> Self {
        Self { neo4j }
    }

    pub async fn ensure_indexes(&self) -> ApiResult<()> {
        let statements = [
            "CREATE CONSTRAINT auth_user_subject IF NOT EXISTS FOR (n:UserProfile) REQUIRE n.subject IS UNIQUE",
            "CREATE INDEX auth_user_email IF NOT EXISTS FOR (n:UserProfile) ON (n.email)",
            "CREATE INDEX auth_user_status IF NOT EXISTS FOR (n:UserProfile) ON (n.status)",
            "CREATE CONSTRAINT auth_beta_invite_id IF NOT EXISTS FOR (n:BetaInvite) REQUIRE n.invite_id IS UNIQUE",
            "CREATE CONSTRAINT auth_beta_invite_hash IF NOT EXISTS FOR (n:BetaInvite) REQUIRE n.token_hash IS UNIQUE",
            "CREATE INDEX auth_beta_invite_status IF NOT EXISTS FOR (n:BetaInvite) ON (n.status)",
            "CREATE INDEX auth_beta_invite_email IF NOT EXISTS FOR (n:BetaInvite) ON (n.email)",
            "CREATE CONSTRAINT auth_invite_request_id IF NOT EXISTS FOR (n:InviteRequest) REQUIRE n.request_id IS UNIQUE",
            "CREATE INDEX auth_invite_request_email IF NOT EXISTS FOR (n:InviteRequest) ON (n.email)",
            "CREATE INDEX auth_invite_request_status IF NOT EXISTS FOR (n:InviteRequest) ON (n.status)",
        ];
        for statement in statements {
            self.neo4j.run_rows(query(statement)).await?;
        }
        Ok(())
    }

    pub async fn create_access_request(
        &self,
        request: CreateAccessRequest,
    ) -> ApiResult<CreateAccessRequestResponse> {
        let email = normalize_email(&request.email)
            .ok_or_else(|| ApiError::BadRequest("Valid email is required".to_string()))?;
        let now = now_string();
        let request_id = format!("access-request:{}", hex_prefix(email.as_bytes(), 24));
        let existing = self.get_access_request_by_email(&email).await?;
        let created_at = existing
            .as_ref()
            .map(|request| request.created_at.clone())
            .unwrap_or_else(|| now.clone());
        let next = InviteRequest {
            request_id,
            email: email.clone(),
            status: existing
                .as_ref()
                .map(|request| request.status.clone())
                .unwrap_or_else(|| "pending".to_string()),
            situation_type: clean_optional(request.situation_type, 80),
            deadline_urgency: clean_optional(request.deadline_urgency, 80),
            jurisdiction: clean_optional(request.jurisdiction, 120),
            note: clean_optional(request.note, MAX_NOTE_CHARS),
            fulfilled_invite_id: existing.and_then(|request| request.fulfilled_invite_id),
            created_at,
            updated_at: now,
        };
        self.merge_access_request(&next).await?;
        Ok(CreateAccessRequestResponse {
            ok: true,
            status: "received".to_string(),
            message: "Your beta access request was received.".to_string(),
        })
    }

    pub async fn lookup_invite(&self, token: &str) -> ApiResult<InviteLookupResponse> {
        let Some(invite) = self.invite_by_token(token).await? else {
            return Ok(InviteLookupResponse {
                found: false,
                status: "not_found".to_string(),
                email: None,
                situation_type: None,
                deadline_urgency: None,
                jurisdiction: None,
                expires_at: None,
                accepted_at: None,
            });
        };
        Ok(InviteLookupResponse {
            found: true,
            status: effective_invite_status(&invite),
            email: invite.email,
            situation_type: invite.situation_type,
            deadline_urgency: invite.deadline_urgency,
            jurisdiction: invite.jurisdiction,
            expires_at: Some(invite.expires_at),
            accepted_at: invite.accepted_at,
        })
    }

    pub async fn me(&self, auth: &AuthContext, admin_role: &str) -> ApiResult<AuthMeResponse> {
        if auth.is_service() {
            return Ok(AuthMeResponse {
                authenticated: true,
                access_status: "active".to_string(),
                subject: auth.subject.clone(),
                email: auth.email.clone(),
                name: auth.name.clone(),
                roles: auth.roles.iter().cloned().collect(),
                is_admin: true,
                profile: None,
            });
        }

        let subject = auth.subject()?.to_string();
        let mut profile = match self.get_user_profile(&subject).await? {
            Some(mut profile) => {
                profile.email = auth.email.clone().or(profile.email);
                profile.name = auth.name.clone().or(profile.name);
                profile.last_seen_at = Some(now_string());
                profile.updated_at = now_string();
                profile.roles = merge_roles(profile.roles, auth.roles.iter().cloned());
                self.merge_user_profile(&profile).await?
            }
            None => {
                let now = now_string();
                self.merge_user_profile(&UserProfile {
                    subject: subject.clone(),
                    email: auth.email.clone(),
                    name: auth.name.clone(),
                    status: "pending".to_string(),
                    roles: auth.roles.iter().cloned().collect(),
                    situation_type: None,
                    deadline_urgency: None,
                    jurisdiction: None,
                    first_matter_id: None,
                    onboarding_completed_at: None,
                    invite_id: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    last_seen_at: Some(now),
                })
                .await?
            }
        };
        profile.status = normalized_access_status(&profile.status);
        Ok(AuthMeResponse {
            authenticated: true,
            access_status: profile.status.clone(),
            subject: Some(subject),
            email: auth.email.clone(),
            name: auth.name.clone(),
            roles: profile.roles.clone(),
            is_admin: auth.is_admin(admin_role),
            profile: Some(profile),
        })
    }

    pub async fn accept_invite(
        &self,
        token: &str,
        auth: &AuthContext,
        admin_role: &str,
    ) -> ApiResult<AuthMeResponse> {
        let subject = auth.subject()?.to_string();
        let mut invite = self
            .invite_by_token(token)
            .await?
            .ok_or_else(|| ApiError::NotFound("Invite not found".to_string()))?;
        match effective_invite_status(&invite).as_str() {
            "active" => {}
            "accepted" => {
                if invite.accepted_by_subject.as_deref() == Some(subject.as_str()) {
                    return self.me(auth, admin_role).await;
                }
                return Err(ApiError::Conflict(
                    "Invite has already been accepted".to_string(),
                ));
            }
            "expired" => return Err(ApiError::BadRequest("Invite has expired".to_string())),
            "revoked" => return Err(ApiError::BadRequest("Invite has been revoked".to_string())),
            _ => return Err(ApiError::BadRequest("Invite is not active".to_string())),
        }

        if let Some(invite_email) = invite.email.as_deref() {
            let user_email = auth.email.as_deref().and_then(normalize_email);
            if user_email.as_deref() != normalize_email(invite_email).as_deref() {
                return Err(ApiError::Forbidden(
                    "Invite email does not match signed-in account".to_string(),
                ));
            }
        }

        let now = now_string();
        invite.status = "accepted".to_string();
        invite.accepted_by_subject = Some(subject.clone());
        invite.accepted_at = Some(now.clone());
        invite.updated_at = now.clone();
        self.merge_invite(&invite).await?;

        let existing = self.get_user_profile(&subject).await?;
        let profile = UserProfile {
            subject: subject.clone(),
            email: auth.email.clone().or_else(|| invite.email.clone()),
            name: auth
                .name
                .clone()
                .or_else(|| existing.as_ref().and_then(|profile| profile.name.clone())),
            status: "active".to_string(),
            roles: merge_roles(
                existing
                    .as_ref()
                    .map(|profile| profile.roles.clone())
                    .unwrap_or_default(),
                invite
                    .roles
                    .iter()
                    .cloned()
                    .chain(auth.roles.iter().cloned()),
            ),
            situation_type: invite.situation_type.clone().or_else(|| {
                existing
                    .as_ref()
                    .and_then(|profile| profile.situation_type.clone())
            }),
            deadline_urgency: invite.deadline_urgency.clone().or_else(|| {
                existing
                    .as_ref()
                    .and_then(|profile| profile.deadline_urgency.clone())
            }),
            jurisdiction: invite.jurisdiction.clone().or_else(|| {
                existing
                    .as_ref()
                    .and_then(|profile| profile.jurisdiction.clone())
            }),
            first_matter_id: existing
                .as_ref()
                .and_then(|profile| profile.first_matter_id.clone()),
            onboarding_completed_at: existing
                .as_ref()
                .and_then(|profile| profile.onboarding_completed_at.clone()),
            invite_id: Some(invite.invite_id.clone()),
            created_at: existing
                .as_ref()
                .map(|profile| profile.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now.clone(),
            last_seen_at: Some(now),
        };
        self.merge_user_profile(&profile).await?;

        if let Some(email) = profile.email.as_deref().and_then(normalize_email) {
            if let Some(mut request) = self.get_access_request_by_email(&email).await? {
                request.status = "invited".to_string();
                request.fulfilled_invite_id = Some(invite.invite_id.clone());
                request.updated_at = now_string();
                self.merge_access_request(&request).await?;
            }
        }

        self.me(auth, admin_role).await
    }

    pub async fn require_active_user(&self, auth: &AuthContext, admin_role: &str) -> ApiResult<()> {
        if auth.is_service() || auth.is_admin(admin_role) {
            return Ok(());
        }
        let subject = auth.subject()?;
        let Some(profile) = self.get_user_profile(subject).await? else {
            return Err(ApiError::Forbidden("Invite required".to_string()));
        };
        match normalized_access_status(&profile.status).as_str() {
            "active" => Ok(()),
            "blocked" => Err(ApiError::Forbidden("App access blocked".to_string())),
            _ => Err(ApiError::Forbidden("Beta access is pending".to_string())),
        }
    }

    pub async fn list_access_requests(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> ApiResult<Vec<InviteRequest>> {
        let limit = limit.clamp(1, 500) as i64;
        let status = status.unwrap_or("").trim().to_ascii_lowercase();
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (r:InviteRequest)
                     WHERE $status = '' OR r.status = $status
                     RETURN r.payload AS payload
                     ORDER BY r.updated_at DESC
                     LIMIT $limit",
                )
                .param("status", status)
                .param("limit", limit),
            )
            .await?;
        rows.into_iter().map(payload_from_row).collect()
    }

    pub async fn list_invites(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> ApiResult<Vec<BetaInvite>> {
        let limit = limit.clamp(1, 500) as i64;
        let status = status.unwrap_or("").trim().to_ascii_lowercase();
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (i:BetaInvite)
                     WHERE $status = '' OR i.status = $status
                     RETURN i.payload AS payload
                     ORDER BY i.updated_at DESC
                     LIMIT $limit",
                )
                .param("status", status)
                .param("limit", limit),
            )
            .await?;
        rows.into_iter().map(payload_from_row).collect()
    }

    pub async fn create_invite(
        &self,
        request: CreateInviteRequest,
        auth: &AuthContext,
    ) -> ApiResult<CreateInviteResponse> {
        let token = generate_invite_token();
        let token_hash = hash_invite_token(&token);
        let now = now_string();
        let ttl_days = request
            .expires_in_days
            .unwrap_or(DEFAULT_INVITE_TTL_DAYS)
            .clamp(1, 90);
        let invite = BetaInvite {
            invite_id: format!("invite:{}", hex_prefix(token_hash.as_bytes(), 24)),
            token_hash,
            email: request.email.and_then(|email| normalize_email(&email)),
            status: "active".to_string(),
            roles: normalize_roles(request.roles.unwrap_or_default()),
            situation_type: clean_optional(request.situation_type, 80),
            deadline_urgency: clean_optional(request.deadline_urgency, 80),
            jurisdiction: clean_optional(request.jurisdiction, 120),
            created_by_subject: auth.subject.clone(),
            accepted_by_subject: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            expires_at: (now_secs() + ttl_days * 86_400).to_string(),
            accepted_at: None,
            revoked_at: None,
        };
        self.merge_invite(&invite).await?;
        Ok(CreateInviteResponse {
            invite,
            token: token.clone(),
            invite_url_path: format!("/auth/invite/{token}"),
        })
    }

    pub async fn revoke_invite(&self, invite_id: &str) -> ApiResult<BetaInvite> {
        let mut invite = self
            .get_invite(invite_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Invite not found".to_string()))?;
        if invite.status == "accepted" {
            return Err(ApiError::Conflict(
                "Accepted invites cannot be revoked".to_string(),
            ));
        }
        invite.status = "revoked".to_string();
        invite.revoked_at = Some(now_string());
        invite.updated_at = now_string();
        self.merge_invite(&invite).await
    }

    pub async fn update_user_access(
        &self,
        subject: &str,
        status: Option<String>,
        roles: Option<Vec<String>>,
    ) -> ApiResult<UserProfile> {
        let mut profile = self
            .get_user_profile(subject)
            .await?
            .ok_or_else(|| ApiError::NotFound("User profile not found".to_string()))?;
        if let Some(status) = status {
            profile.status = normalized_access_status(&status);
        }
        if let Some(roles) = roles {
            profile.roles = normalize_roles(roles);
        }
        profile.updated_at = now_string();
        self.merge_user_profile(&profile).await
    }

    async fn invite_by_token(&self, token: &str) -> ApiResult<Option<BetaInvite>> {
        let token_hash = hash_invite_token(token);
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (i:BetaInvite {token_hash: $token_hash}) RETURN i.payload AS payload")
                    .param("token_hash", token_hash),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn get_invite(&self, invite_id: &str) -> ApiResult<Option<BetaInvite>> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (i:BetaInvite {invite_id: $invite_id}) RETURN i.payload AS payload")
                    .param("invite_id", invite_id),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn get_user_profile(&self, subject: &str) -> ApiResult<Option<UserProfile>> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (u:UserProfile {subject: $subject}) RETURN u.payload AS payload")
                    .param("subject", subject),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn get_access_request_by_email(&self, email: &str) -> ApiResult<Option<InviteRequest>> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (r:InviteRequest {email: $email}) RETURN r.payload AS payload")
                    .param("email", email),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn merge_user_profile(&self, profile: &UserProfile) -> ApiResult<UserProfile> {
        let payload = to_payload(profile)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (u:UserProfile {subject: $subject})
                     SET u.payload = $payload,
                         u.email = $email,
                         u.name = $name,
                         u.status = $status,
                         u.updated_at = $updated_at
                     RETURN u.payload AS payload",
                )
                .param("subject", profile.subject.clone())
                .param("payload", payload)
                .param("email", profile.email.clone())
                .param("name", profile.name.clone())
                .param("status", profile.status.clone())
                .param("updated_at", profile.updated_at.clone()),
            )
            .await?;
        Ok(profile.clone())
    }

    async fn merge_invite(&self, invite: &BetaInvite) -> ApiResult<BetaInvite> {
        let payload = to_payload(invite)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (i:BetaInvite {invite_id: $invite_id})
                     SET i.payload = $payload,
                         i.token_hash = $token_hash,
                         i.email = $email,
                         i.status = $status,
                         i.updated_at = $updated_at
                     RETURN i.payload AS payload",
                )
                .param("invite_id", invite.invite_id.clone())
                .param("payload", payload)
                .param("token_hash", invite.token_hash.clone())
                .param("email", invite.email.clone())
                .param("status", invite.status.clone())
                .param("updated_at", invite.updated_at.clone()),
            )
            .await?;
        Ok(invite.clone())
    }

    async fn merge_access_request(&self, request: &InviteRequest) -> ApiResult<InviteRequest> {
        let payload = to_payload(request)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (r:InviteRequest {request_id: $request_id})
                     SET r.payload = $payload,
                         r.email = $email,
                         r.status = $status,
                         r.updated_at = $updated_at
                     RETURN r.payload AS payload",
                )
                .param("request_id", request.request_id.clone())
                .param("payload", payload)
                .param("email", request.email.clone())
                .param("status", request.status.clone())
                .param("updated_at", request.updated_at.clone()),
            )
            .await?;
        Ok(request.clone())
    }
}

pub fn hash_invite_token(token: &str) -> String {
    format!("sha256:{}", hex_digest(token.trim().as_bytes()))
}

pub fn effective_invite_status(invite: &BetaInvite) -> String {
    let status = invite.status.trim().to_ascii_lowercase();
    if matches!(status.as_str(), "accepted" | "revoked") {
        return status;
    }
    let expires_at = invite.expires_at.parse::<u64>().unwrap_or(0);
    if expires_at <= now_secs() {
        return "expired".to_string();
    }
    if status.is_empty() {
        "active".to_string()
    } else {
        status
    }
}

fn generate_invite_token() -> String {
    format!(
        "orsg_inv_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

fn to_payload<T: Serialize>(value: &T) -> ApiResult<String> {
    serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn from_payload<T: DeserializeOwned>(payload: &str) -> ApiResult<T> {
    serde_json::from_str(payload).map_err(|error| ApiError::Internal(error.to_string()))
}

fn payload_from_row<T: DeserializeOwned>(row: Row) -> ApiResult<T> {
    let payload = row
        .get::<String>("payload")
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    from_payload(&payload)
}

fn normalize_email(email: &str) -> Option<String> {
    let email = email.trim().to_ascii_lowercase();
    if email.contains('@') && email.contains('.') && email.len() <= 254 {
        Some(email)
    } else {
        None
    }
}

fn normalized_access_status(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" => "active".to_string(),
        "blocked" => "blocked".to_string(),
        "revoked" => "blocked".to_string(),
        "pending" => "pending".to_string(),
        _ => "pending".to_string(),
    }
}

fn normalize_roles(roles: Vec<String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for role in roles {
        let role = role.trim();
        if !role.is_empty() {
            set.insert(role.to_string());
        }
    }
    set.into_iter().collect()
}

fn merge_roles(base: Vec<String>, extra: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut roles = BTreeSet::new();
    for role in base.into_iter().chain(extra) {
        let role = role.trim();
        if !role.is_empty() {
            roles.insert(role.to_string());
        }
    }
    roles.into_iter().collect()
}

fn clean_optional(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| value.trim().chars().take(max_chars).collect::<String>())
        .filter(|value| !value.is_empty())
}

fn now_string() -> String {
    now_secs().to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let digest = hex_digest(bytes);
    digest.chars().take(chars).collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invite(status: &str, expires_at: u64) -> BetaInvite {
        BetaInvite {
            invite_id: "invite:test".to_string(),
            token_hash: hash_invite_token("token"),
            email: Some("user@example.com".to_string()),
            status: status.to_string(),
            roles: vec![],
            situation_type: None,
            deadline_urgency: None,
            jurisdiction: None,
            created_by_subject: None,
            accepted_by_subject: None,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            expires_at: expires_at.to_string(),
            accepted_at: None,
            revoked_at: None,
        }
    }

    #[test]
    fn invite_token_hash_is_stable_and_does_not_store_raw_token() {
        let first = hash_invite_token("orsg_inv_secret");
        let second = hash_invite_token("orsg_inv_secret");
        assert_eq!(first, second);
        assert!(first.starts_with("sha256:"));
        assert!(!first.contains("secret"));
    }

    #[test]
    fn expired_invite_is_effective_status_even_when_marked_active() {
        let expired = invite("active", now_secs().saturating_sub(1));
        assert_eq!(effective_invite_status(&expired), "expired");
    }

    #[test]
    fn accepted_and_revoked_invites_keep_terminal_status() {
        let accepted = invite("accepted", now_secs().saturating_sub(1));
        let revoked = invite("revoked", now_secs() + 86_400);
        assert_eq!(effective_invite_status(&accepted), "accepted");
        assert_eq!(effective_invite_status(&revoked), "revoked");
    }

    #[test]
    fn email_normalization_rejects_invalid_shapes() {
        assert_eq!(
            normalize_email("  USER@Example.COM "),
            Some("user@example.com".to_string())
        );
        assert_eq!(normalize_email("not-an-email"), None);
    }

    #[test]
    fn access_status_is_strictly_normalized() {
        assert_eq!(normalized_access_status("ACTIVE"), "active");
        assert_eq!(normalized_access_status("revoked"), "blocked");
        assert_eq!(normalized_access_status("anything"), "pending");
    }
}
