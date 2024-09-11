use axum_test::TestServer;

use play::routers;
use play::init_app_state;
use play::tables::user::User;
use shared::models::user::{AddUser, QueryUser, UpdateUser};

#[ignore]
#[tokio::test]
async fn test_all() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(&play::config::init_config(true), true).await))?;


    let response = server.get("/add-user").add_query_params(AddUser {
        name: "abc".to_string(),
    }).await;
    assert_eq!(response.status_code(), 200);


    let response = server.get("/users").add_query_params(QueryUser { name: "abc".to_string() }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.json::<Vec<User>>().len(), 1);


    let response = server.get("/update-user/1").add_query_params(UpdateUser {
        name: "abc new".to_string(),
    }).await;
    assert_eq!(response.status_code(), 200);


    let response = server.get("/users").add_query_params(QueryUser { name: "abc new".to_string() }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.json::<Vec<User>>().len(), 1);


    let response = server.get("/delete-user/1").await;
    assert_eq!(response.status_code(), 200);


    let response = server.get("/users").add_query_params(QueryUser { name: "abc new".to_string() }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.json::<Vec<User>>().len(), 0);

    Ok(())
}