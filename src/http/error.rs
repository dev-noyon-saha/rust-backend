use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::response::{ApiResponse, FieldError};

// Centralized API error
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<Vec<FieldError>>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }
}

// Convert errors into standard HTTP JSON responses
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiResponse::<()>::failure(self.message, self.code, self.details);

        (self.status, Json(body)).into_response()
    }
}
