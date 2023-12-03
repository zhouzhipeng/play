use axum_test::{TestServer, TestServerConfig, Transport};


use play::controller::routers;
use play::init_app_state;
use play::tables::article::Article;

use shared::models::{RequestClient};
use shared::models::article::{AddArticle, QueryArticle};

#[tokio::test]
async fn test_api_controller() -> anyhow::Result<()> {

    let server = TestServer::new_with_config(routers(init_app_state(play::config::init_config(), true).await), TestServerConfig{
        transport:  Some(Transport::HttpRandomPort),
       ..TestServerConfig::default()
    })?;
    let host = server.server_address().unwrap().to_string();
    println!("host >> {}", host);
    //
    let client = RequestClient{
        host,
        ..RequestClient::default()
    };
    let r = client.api_article_add( &AddArticle{
        title: "123".to_string(),
        content: "456".to_string(),
    }).await?;
    assert_eq!(r, "ok");


    let response = server.get("/api/article/list").add_query_params(QueryArticle {
        title: "123".to_string(),
    }).await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.json::<Vec<Article>>().len(), 1);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fragment_controller() -> anyhow::Result<()> {
    let server = TestServer::new(routers(init_app_state(play::config::init_config(), true).await))?;


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
    let server = TestServer::new(routers(init_app_state(play::config::init_config(), true).await))?;


    let response = server.get("/page/article/add").await;
    assert_eq!(response.status_code(), 200);
    // assert_eq!(response.text(), "Added result : ok");



    Ok(())
}