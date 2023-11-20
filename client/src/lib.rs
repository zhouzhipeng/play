use anyhow::anyhow;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;

use shared::models::{RequestClient, user};
use shared::models::article::AddArticle;
use shared::models::user::{AddUser, QueryUser};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

async fn test_http() -> anyhow::Result<()> {
    let body = user::add_user(AddUser {
        name: "zzp".to_string(),
    }).await?;

    console_log!("body = {:?}", body);

    let body = user::query_users(QueryUser {
        name: "zzp".to_string(),
    }).await?;

    console_log!("body = {:?}", body);
    Ok(())
}

#[wasm_bindgen(start)]
async fn main() -> Result<(), JsValue> {
    set_panic_hook();
    Ok(init().await.unwrap())
}

#[wasm_bindgen]
pub async fn save_btn_click() -> Result<(), JsValue> {
    //test
    let client = RequestClient::default();


    //dom related
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    let title_input = document.get_element_by_id("title").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
    let content_input = document.get_element_by_id("content").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
    // let save_button = document.get_element_by_id("save").unwrap().dyn_into::<web_sys::HtmlButtonElement>().unwrap();
    let result_pre = document.get_element_by_id("result").unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();

    let r = client.api_article_add(&AddArticle{
        title: title_input.value(),
        content: content_input.value(),
    }).await.unwrap();

    result_pre.set_inner_text(r.as_str());

    console_log!("clicked");
    Ok(())
}


// #[wasm_bindgen(start)]
async fn init() -> anyhow::Result<()> {
    panic!("test panic!");




    Ok(())
}


pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}