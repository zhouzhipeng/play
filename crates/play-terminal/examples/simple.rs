use axum::{
    routing::get,
    Router,
};
use play_terminal::WebTerminal;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "axum_web_terminal=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create a web terminal with default configuration
    let web_terminal = WebTerminal::default();

    // Create the main application router
    let app = Router::new()
        // Add a route to the home page
        .route("/", get(|| async { "Hello, World!" }))
        // Nest the web terminal router under the /terminal path
        .nest("/terminal", web_terminal.router());

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}