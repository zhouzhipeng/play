use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::{inform, STRING, WasmDescribe};
use crate::models::article::AddArticle;


impl WasmDescribe for AddArticle {
    fn describe() {
        inform(STRING);
    }
}


impl FromWasmAbi for AddArticle {
    type Abi = <Vec<u8> as FromWasmAbi>::Abi;

    #[inline]
    unsafe fn from_abi(js: Self::Abi) -> Self {
        serde_json::from_str(String::from_utf8_unchecked(<Vec<u8>>::from_abi(js)).as_str()).unwrap()
    }
}
