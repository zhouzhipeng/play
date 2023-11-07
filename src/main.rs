use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use axum::{http::StatusCode, Json, response::IntoResponse, Router, routing::{get, post}};
use axum::extract::State;
use axum::handler::HandlerWithoutStateExt;
use crossbeam_channel::{Sender, unbounded};
use serde::{Deserialize, Serialize};
use tokio::spawn;
use crate::py_tool::Value;

mod py_tool;


// Define a struct to hold the shared state
struct AppState {
    py_sender: Sender<&'static str>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    //

// Create a channel of unbounded capacity.
    let (s, r) = unbounded();

// Send a message into the channel.
    s.send("Hello, world!").unwrap();
    spawn(async move {
        let interpreter = py_tool::init_py_interpreter();

        loop{
            // Receive the message from the channel.
            let result = r.recv().unwrap();
            println!("recv : {}", result);

            let html = fs::read_to_string("/Users/zhouzhipeng/RustroverProjects/play/templates/index.html").unwrap();

            let args = HashMap::from([
                ("name", Value::Str("周志鹏sss".into())),
                ("age", Value::Int(20)),
                ("male", Value::Bool(true))
            ]);

            match py_tool::run_py_template( &interpreter, html.as_str(), "hello", args) {
                Ok(s) => println!("{}", s),
                Err(s) => println!("{}", s),
            }
        }
    });


    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        py_sender: s,
    });

    // build our application with a route
    let app = Router::new()

        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .with_state(app_state);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root(State(state): State<Arc<AppState>>,) -> &'static str {
    // py_tool::test();
    state.py_sender.send("hello").expect("send error");
    "Hello, World!"
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}