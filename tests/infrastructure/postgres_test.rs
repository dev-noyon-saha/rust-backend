use backend_project::infrastructure::database::postgres;

fn database_url() -> String {
    dotenvy::dotenv().ok();

    std::env::var("DATABASE_URL").expect("DATABASE_URL must be configured")
}

#[tokio::test]
async fn postgres_connection_successful() {
    let url = database_url();

    let pool = postgres::connect(&url)
        .await
        .expect("Failed to connect to PostgreSQL");

    assert!(!pool.is_closed());

    pool.close().await;
}

#[tokio::test]
async fn postgres_query_execution_successful() {
    let url = database_url();

    let pool = postgres::connect(&url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Execute test query
    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("Failed to execute PostgreSQL query");

    assert_eq!(result, 1);

    pool.close().await;
}
