use std::fmt::format;
use wasm_bindgen::__rt::IntoJsResult;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::{inform, STRING, WasmDescribe};
use wasm_bindgen::JsValue;
use crate::models::article::AddArticle;


macro_rules! impl_wasm_abi_input {
    ($t:ty) => {
        impl WasmDescribe for $t {
            fn describe() {
                inform(STRING);
            }
        }


        impl FromWasmAbi for $t {
             type Abi = <Vec<u8> as FromWasmAbi>::Abi;

            #[inline]
            unsafe fn from_abi(js: Self::Abi) -> Self {
                serde_json::from_str(String::from_utf8_unchecked(<Vec<u8>>::from_abi(js)).as_str()).unwrap()
            }
        }

    };
}

macro_rules! impl_wasm_abi_output {
    ($t:ty) => {
        impl IntoJsResult for $t{
            fn into_js_result(self) -> Result<JsValue, JsValue>{
                match serde_json::to_string(&self){
                    Ok(s)=>Ok(JsValue::from_str(s.as_str())),
                    Err(e)=>Err(JsValue::from_str(format!("Error : serde json error :{:?}", e).as_str()))
                }
            }
        }

    };
}


// expose it to #[wasm_bindgen] input params
impl_wasm_abi_input!(AddArticle);

// expose it to #[wasm_bindgen] output params
impl_wasm_abi_output!(AddArticle);