use crate::error::ApiResult;
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::{Json, extract::State, response::IntoResponse};

pub async fn get_home(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let data = state.home_service.get_home_page_data().await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(data)))
}

pub async fn featured_statutes(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let data = state.home_service.get_featured_statutes().await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(data)))
}
