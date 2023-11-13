use std::net::{SocketAddr, TcpListener};
use axum_test::TestServer;
use hyper::{Body, Request};
use tokio::runtime::Runtime;

use tracing::info;

use play::controller::routers;
use play::init_app_state;
use play::tables::user::{QueryUser, User};

#[tokio::test]
async fn test_index_controller() -> anyhow::Result<()> {

    let app_state = init_app_state().await;
    let server = TestServer::new(routers(app_state))?;
    let response = server.get("/").await;
    assert_eq!(response.status_code(),200);

    let response = server.get("/test").await;
    assert_eq!(response.status_code(),200);

    Ok(())
}

#[tokio::test]
async fn test_user_controller() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state().await))?;
    let response = server.get("/users").add_query_params(QueryUser{ name: "zzp".to_string() }).await;
    assert_eq!(response.status_code(),200);
    assert_eq!(response.json::<Vec<User>>().len(),2);
    Ok(())
}