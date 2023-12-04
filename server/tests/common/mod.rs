use axum_test::{TestServer, TestServerConfig, Transport};
use shared::models::{RequestClient};
use play::controller::routers;
use play::init_app_state;
use play::tables::article::Article;
pub async fn setup() -> (TestServer, RequestClient) {
    let server = TestServer::new_with_config(routers(init_app_state(&play::config::init_config(), true).await), TestServerConfig{
        transport:  Some(Transport::HttpRandomPort),
        ..TestServerConfig::default()
    }).unwrap();
    let host = server.server_address().unwrap().to_string();
    println!("host >> {}", host);
    //
    let client = RequestClient{
        host,
        ..RequestClient::default()
    };
    (server, client)
}