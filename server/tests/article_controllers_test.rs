use axum_test::TestServer;

use play::routers;
use play::init_app_state;
use play::tables::article::Article;
use shared::models::article::{AddArticle, QueryArticle};

mod common;


#[tokio::test]
#[ignore]
async fn test_fragment_controller() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(&play::config::init_config(true), true).await))?;


    let response = server.post("/fragment/article/add").form(&AddArticle {
        title: "123".to_string(),
        content: "456".to_string(),
    }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.text(), "Added result : ok");

    let response = server.get("/api/article/list").add_query_params(QueryArticle {
        title: "123".to_string(),
    }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.json::<Vec<Article>>().len(), 1);


    Ok(())
}


#[tokio::test]
#[ignore]
async fn test_page_controller() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(&play::config::init_config(true), true).await))?;


    let response = server.get("/page/article/add").await;
    assert_eq!(response.status_code(), 200);
    // assert_eq!(response.text(), "Added result : ok");


    Ok(())
}