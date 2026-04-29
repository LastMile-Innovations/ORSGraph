use crate::error::ApiResult;
use crate::models::home::{FeaturedStatute, HomePageData};
use crate::state::AppState;
use axum::{extract::State, Json};

pub async fn get_home(State(state): State<AppState>) -> ApiResult<Json<HomePageData>> {
    let data = state.home_service.get_home_page_data().await?;
    Ok(Json(data))
}

pub async fn featured_statutes(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<FeaturedStatute>>> {
    let data = state.home_service.get_featured_statutes().await?;
    Ok(Json(data))
}
