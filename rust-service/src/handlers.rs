use crate::{error::AppError, state::AppState};
use axum::{
    extract::{Path, State},
    response::Response,
    Json,
};
use serde::Serialize;
use sqlx::Row;

#[derive(Serialize)]
pub struct ItemDto {
    pub id: i32,
    pub name: String,
}

pub async fn health() -> &'static str {
    "ok"
}

pub async fn get_item(
    State(st): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ItemDto>, AppError> {
    if id <= 0 {
        return Err(AppError::invalid_config("id must be a positive integer"));
    }

    let row_opt = sqlx::query(r#"SELECT id, name FROM items WHERE id = $1"#)
        .bind(id)
        .fetch_optional(&st.pool)
        .await
        .map_err(AppError::Db)?;

    let row = match row_opt {
        Some(r) => r,
        None => return Err(AppError::NotFound(format!("Item {id} not found"))),
    };

    let id: i32 = row.try_get("id").map_err(AppError::Db)?;
    let name: String = row.try_get("name").map_err(AppError::Db)?;

    Ok(Json(ItemDto { id, name }))
}

pub async fn metrics_endpoint(State(st): State<AppState>) -> Response {
    st.metrics.response()
}
