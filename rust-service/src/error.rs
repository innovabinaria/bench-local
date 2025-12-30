use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    MissingEnv(&'static str),
    InvalidConfig(&'static str),
    NotFound(String),
    Db(sqlx::Error),
    Io(std::io::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl AppError {
    pub fn missing_env(var: &'static str) -> Self {
        Self::MissingEnv(var)
    }

    pub fn invalid_config(msg: &'static str) -> Self {
        Self::InvalidConfig(msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::MissingEnv(var) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Missing env var: {var}"),
            ),
            AppError::InvalidConfig(msg) => (StatusCode::BAD_REQUEST, msg.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")),
            AppError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("IO error: {e}")),
        };

        (status, Json(ErrorBody { error: msg })).into_response()
    }
}
