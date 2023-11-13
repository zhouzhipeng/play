use axum_test::TestServer;
use play::controller::routers;
use play::init_app_state;
use play::tables::user::{AddUser, QueryUser, UpdateUser, User};

#[tokio::test]
async fn test_all() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(true).await))?;
    let response = server.get("/").await;
    assert_eq!(response.status_code(), 200);

    let response = server.get("/test").await;
    assert_eq!(response.status_code(), 200);

    Ok(())
}
