use std::env;
use axum_test::TestServer;

use play::controller::routers;
use play::init_app_state;
use play::tables::user::{AddUser, QueryUser, UpdateUser, User};

#[tokio::test]
async fn test_index_controller() -> anyhow::Result<()> {

    let server = TestServer::new(routers(init_app_state(true).await))?;
    let response = server.get("/").await;
    assert_eq!(response.status_code(),200);

    let response = server.get("/test").await;
    assert_eq!(response.status_code(),200);

    Ok(())
}

#[tokio::test]
async fn test_user_controller() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(true).await))?;


    let response=server.get("/add-user").add_query_params(AddUser{
        name: "abc".to_string(),
    }).await;
    assert_eq!(response.status_code(),200);


    let response = server.get("/users").add_query_params(QueryUser{ name: "abc".to_string() }).await;
    assert_eq!(response.status_code(),200);
    assert_eq!(response.json::<Vec<User>>().len(),1);


    let response=server.get("/update-user/1").add_query_params(UpdateUser{
        name: "abc new".to_string(),
    }).await;
    assert_eq!(response.status_code(),200);


    let response = server.get("/users").add_query_params(QueryUser{ name: "abc new".to_string() }).await;
    assert_eq!(response.status_code(),200);
    assert_eq!(response.json::<Vec<User>>().len(),1);


    let response=server.get("/delete-user/1").await;
    assert_eq!(response.status_code(),200);


    let response = server.get("/users").add_query_params(QueryUser{ name: "abc new".to_string() }).await;
    assert_eq!(response.status_code(),200);
    assert_eq!(response.json::<Vec<User>>().len(),0);

    Ok(())
}