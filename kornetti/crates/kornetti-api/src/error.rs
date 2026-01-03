//! API error handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use kornetti_core::Error;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            error: ErrorBody {
                message: self.message,
                code: self.code,
            },
        });
        (self.status, body).into_response()
    }
}

impl From<Error> for ApiError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotFound(msg) => ApiError {
                status: StatusCode::NOT_FOUND,
                message: msg,
                code: Some("NOT_FOUND".to_string()),
            },
            Error::Unauthorized(msg) => ApiError {
                status: StatusCode::UNAUTHORIZED,
                message: msg,
                code: Some("UNAUTHORIZED".to_string()),
            },
            Error::Validation(msg) => ApiError {
                status: StatusCode::BAD_REQUEST,
                message: msg,
                code: Some("VALIDATION_ERROR".to_string()),
            },
            Error::Config(msg) => ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: msg,
                code: Some("CONFIG_ERROR".to_string()),
            },
            _ => ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
                code: Some("INTERNAL_ERROR".to_string()),
            },
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Database error".to_string(),
            code: Some("DATABASE_ERROR".to_string()),
        }
    }
}
