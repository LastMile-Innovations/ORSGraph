use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub roles: Vec<String>,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub first_matter_id: Option<String>,
    pub onboarding_completed_at: Option<String>,
    pub invite_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BetaInvite {
    pub invite_id: String,
    pub token_hash: String,
    pub email: Option<String>,
    pub status: String,
    #[serde(default)]
    pub roles: Vec<String>,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub created_by_subject: Option<String>,
    pub accepted_by_subject: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InviteRequest {
    pub request_id: String,
    pub email: String,
    pub status: String,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub note: Option<String>,
    pub fulfilled_invite_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccessRequest {
    pub email: String,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateAccessRequestResponse {
    pub ok: bool,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct InviteLookupResponse {
    pub found: bool,
    pub status: String,
    pub email: Option<String>,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub expires_at: Option<String>,
    pub accepted_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthMeResponse {
    pub authenticated: bool,
    pub access_status: String,
    pub subject: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub is_admin: bool,
    pub profile: Option<UserProfile>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub email: Option<String>,
    pub roles: Option<Vec<String>>,
    pub situation_type: Option<String>,
    pub deadline_urgency: Option<String>,
    pub jurisdiction: Option<String>,
    pub expires_in_days: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateInviteResponse {
    pub invite: BetaInvite,
    pub token: String,
    pub invite_url_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ListAuthQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PatchUserAccessRequest {
    pub status: Option<String>,
    pub roles: Option<Vec<String>>,
}
