use std::net::SocketAddr;

use axum::{BoxError, Router};
use axum::extract::Host;
use axum::handler::HandlerWithoutStateExt;
use axum::http::{StatusCode, Uri};
use axum::response::Redirect;
use log::{info, warn};
use rustls_acme::AcmeConfig;
use rustls_acme::caches::DirCache;
use tokio_stream::StreamExt;

pub struct HttpsConfig{
    pub domains: Vec<String>,
    pub email: Vec<String>,
    pub cache_dir : String,
    /// (see https://letsencrypt.org/docs/staging-environment/)
    pub prod: bool,
    pub http_port: u16,
    pub https_port: u16,
    pub auto_redirect : bool,
}

pub async fn start_https_server(config : &HttpsConfig, app: Router){

    let mut state = AcmeConfig::new(&config.domains)
        .contact(config.email.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(Some(DirCache::new(config.cache_dir.clone())))
        .directory_lets_encrypt(config.prod)
        .state();
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => log::info!("event: {:?}", ok),
                Err(err) => log::error!("error: {:?}", err),
            }
        }
    });

    if config.auto_redirect{
        //spawn a second server to redirect http requests to this server
        tokio::spawn(redirect_http_to_https(Ports{ http: config.http_port, https: config.https_port }));
    }else{
        //listen 80 port too.
        let app_clone = app.clone();
        let http_port = config.http_port;
        tokio::spawn(async move{
            let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
            axum_server::bind(addr).serve(app_clone.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
        });
    }



    let addr = SocketAddr::from(([0, 0, 0, 0], config.https_port));
    info!("start a https server at : {:?}", addr);
    axum_server::bind(addr).acceptor(acceptor).serve(app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}
async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                warn!("failed to convert URI to HTTPS, err : {:?}", error);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };


    let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
    info!("start a http server at : {:?}", addr);
    axum_server::bind(addr).serve(redirect.into_make_service())
        .await
        .unwrap();
}