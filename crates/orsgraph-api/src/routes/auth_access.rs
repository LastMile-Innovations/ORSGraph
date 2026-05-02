use crate::auth::AuthContext;
use crate::error::ApiResult;
use crate::models::auth_access::{
    AuthMeResponse, BetaInvite, CreateAccessRequest, CreateAccessRequestResponse,
    CreateInviteRequest, CreateInviteResponse, InviteLookupResponse, ListAuthQuery,
    PatchUserAccessRequest, UserProfile,
};
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/access-request", post(create_access_request))
        .route("/auth/invites/{token}", get(lookup_invite))
        .route("/auth/invites/{token}/accept", post(accept_invite))
        .route("/auth/me", get(me))
        .route("/admin/auth/access-requests", get(list_access_requests))
        .route("/admin/auth/invites", get(list_invites).post(create_invite))
        .route(
            "/admin/auth/invites/{invite_id}/revoke",
            post(revoke_invite),
        )
        .route("/admin/auth/users/{subject}", patch(update_user_access))
}

async fn create_access_request(
    State(state): State<AppState>,
    Json(request): Json<CreateAccessRequest>,
) -> ApiResult<Json<CreateAccessRequestResponse>> {
    Ok(Json(
        state
            .auth_access_service
            .create_access_request(request)
            .await?,
    ))
}

async fn lookup_invite(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<InviteLookupResponse>> {
    Ok(Json(state.auth_access_service.lookup_invite(&token).await?))
}

async fn accept_invite(
    State(state): State<AppState>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path(token): Path<String>,
) -> ApiResult<Json<AuthMeResponse>> {
    Ok(Json(
        state
            .auth_access_service
            .accept_invite(&token, &auth, &state.config.auth_admin_role)
            .await?,
    ))
}

async fn me(
    State(state): State<AppState>,
    axum::Extension(auth): axum::Extension<AuthContext>,
) -> ApiResult<Json<AuthMeResponse>> {
    Ok(Json(
        state
            .auth_access_service
            .me(&auth, &state.config.auth_admin_role)
            .await?,
    ))
}

async fn list_access_requests(
    State(state): State<AppState>,
    Query(query): Query<ListAuthQuery>,
) -> ApiResult<Json<Vec<crate::models::auth_access::InviteRequest>>> {
    Ok(Json(
        state
            .auth_access_service
            .list_access_requests(query.status.as_deref(), query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn list_invites(
    State(state): State<AppState>,
    Query(query): Query<ListAuthQuery>,
) -> ApiResult<Json<Vec<BetaInvite>>> {
    Ok(Json(
        state
            .auth_access_service
            .list_invites(query.status.as_deref(), query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn create_invite(
    State(state): State<AppState>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(request): Json<CreateInviteRequest>,
) -> ApiResult<Json<CreateInviteResponse>> {
    Ok(Json(
        state
            .auth_access_service
            .create_invite(request, &auth)
            .await?,
    ))
}

async fn revoke_invite(
    State(state): State<AppState>,
    Path(invite_id): Path<String>,
) -> ApiResult<Json<BetaInvite>> {
    Ok(Json(
        state.auth_access_service.revoke_invite(&invite_id).await?,
    ))
}

async fn update_user_access(
    State(state): State<AppState>,
    Path(subject): Path<String>,
    Json(request): Json<PatchUserAccessRequest>,
) -> ApiResult<Json<UserProfile>> {
    Ok(Json(
        state
            .auth_access_service
            .update_user_access(&subject, request.status, request.roles)
            .await?,
    ))
}
