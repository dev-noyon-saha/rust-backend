use serde::Serialize;

// Standard API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub error: Option<ApiErrorInfo>,
}

// Error information
#[derive(Debug, Serialize)]
pub struct ApiErrorInfo {
    pub code: &'static str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
}

// Field-level validation error
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

// Success response constructor
impl<T> ApiResponse<T> {
    pub fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            error: None,
        }
    }
}

// Error response constructor
impl ApiResponse<()> {
    pub fn failure(
        message: impl Into<String>,
        code: &'static str,
        details: Option<Vec<FieldError>>,
    ) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            error: Some(ApiErrorInfo { code, details }),
        }
    }
}
