use backend_project::http::response::{ApiResponse, FieldError};

use serde_json::{Value, json};

// Test standard success response
#[test]
fn success_response_has_correct_structure() {
    let data = json!({
        "status": "healthy",
        "service": "backend-ai-api",
        "version": "0.1.0"
    });

    let response = ApiResponse::success("Service is healthy", data.clone());

    let result = serde_json::to_value(response).unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["message"], "Service is healthy");
    assert_eq!(result["data"], data);
    assert!(result["error"].is_null());
}

// Test success response with a list
#[test]
fn success_response_supports_generic_data() {
    let products = vec!["Laptop", "Mouse", "Keyboard"];

    let response = ApiResponse::success("Products retrieved successfully", products);

    let result = serde_json::to_value(response).unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["data"][0], "Laptop");
    assert_eq!(result["data"][1], "Mouse");
    assert_eq!(result["data"][2], "Keyboard");
    assert_eq!(result["data"].as_array().unwrap().len(), 3);
}

// Test error response without details
#[test]
fn error_response_has_correct_structure() {
    let response = ApiResponse::failure("Resource not found", "NOT_FOUND", None);

    let result = serde_json::to_value(response).unwrap();

    assert_eq!(result["success"], false);
    assert_eq!(result["message"], "Resource not found");
    assert!(result["data"].is_null());
    assert_eq!(result["error"]["code"], "NOT_FOUND");

    // Optional details should be omitted
    assert!(result["error"].get("details").is_none());
}

// Test validation errors with field details
#[test]
fn validation_error_contains_field_details() {
    let errors = vec![
        FieldError {
            field: "message".to_string(),
            message: "Message cannot be empty".to_string(),
        },
        FieldError {
            field: "business_id".to_string(),
            message: "Invalid business ID".to_string(),
        },
    ];

    let response = ApiResponse::failure("Validation failed", "VALIDATION_ERROR", Some(errors));

    let result = serde_json::to_value(response).unwrap();

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "VALIDATION_ERROR");

    let details = result["error"]["details"].as_array().unwrap();

    assert_eq!(details.len(), 2);
    assert_eq!(details[0]["field"], "message");
    assert_eq!(details[0]["message"], "Message cannot be empty");
    assert_eq!(details[1]["field"], "business_id");
}

// Test that all standard response fields exist
#[test]
fn response_contains_all_required_fields() {
    let response = ApiResponse::success("Operation successful", json!({ "id": 101 }));

    let result: Value = serde_json::to_value(response).unwrap();

    assert!(result.get("success").is_some());
    assert!(result.get("message").is_some());
    assert!(result.get("data").is_some());
    assert!(result.get("error").is_some());
}
