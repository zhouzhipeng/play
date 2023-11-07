use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

use axum::{http::StatusCode, Json, response::IntoResponse, Router, routing::{get, post}};
use axum::extract::{Query, State};
use axum::handler::HandlerWithoutStateExt;
use crossbeam_channel::{bounded, Receiver, Sender, unbounded};
use rustpython_vm as vm;
use serde::{Deserialize, Serialize};
use tokio::spawn;
use crate::py_tool::Value;

mod py_tool;


// Define a struct to hold the shared state
struct AppState {
    py_sender: Sender<String>,
    api_receiver: Receiver<String>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    //

// Create a channel of unbounded capacity.
    let (py_sender, py_receiver) = bounded(0);
    let (api_sender, api_receiver) =  bounded(1);

    spawn(async move {
        let interpreter = py_tool::init_py_interpreter();

        interpreter.enter(|vm| {
            // let module = vm::py_compile!(file = "python/test.py");
            // let code = vm.ctx.new_code(module);

            loop{
                // Receive the message from the channel.
                let result = py_receiver.recv().unwrap();
                println!("recv : {}", result);

                let html = fs::read_to_string("/Users/zhouzhipeng/RustroverProjects/play/templates/index.html").unwrap();
                let pycode = fs::read_to_string("/Users/zhouzhipeng/RustroverProjects/play/python/test.py").unwrap();

                let args = HashMap::from([
                    ("name", Value::Str(result)),
                    ("age", Value::Int(20)),
                    ("male", Value::Bool(true))
                ]);


                let start = Instant::now();
                let r = match py_tool::run_py_template( vm, pycode.as_str(), html.as_str(), "hello", args) {
                    Ok(s) => s,
                    Err(s) => s,
                };



                api_sender.send(r).expect("send error");
                println!("send spent:{}", start.elapsed().as_millis());

            }
        });


    });


    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        py_sender,
        api_receiver
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

#[derive(Deserialize)]
struct Param{
    name : String
}

// basic handler that responds with a static string
async fn root(name: Query<Param> , State(state): State<Arc<AppState>>,) -> String {
    // py_tool::test();
    state.py_sender.send(name.0.name).expect("send error");
    state.api_receiver.recv().unwrap()
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