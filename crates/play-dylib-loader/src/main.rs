use libloading::{Library, Symbol};
use play_shared::models::http_abi::*;
use std::error::Error;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // 获取 handle_request 函数的符号
    unsafe {


        let t : JoinHandle<anyhow::Result<()>>= tokio::spawn(async move{
            // 加载动态库
            let lib = Library::new("/Users/zhouzhipeng/RustroverProjects/play/target/release/libplay_dylib_example.dylib")?;


            let handle_request: Symbol<HandleRequestFn> =
                lib.get(HANDLE_REQUEST_FN_NAME.as_ref())?;

            // 创建一个请求
            let request = Request {
                headers: Default::default(),
                query: Default::default(),
                body: "sdfd".to_string(),
                url: "sdf".to_string(),
            };



            let response = handle_request(request);
            //
            // // 调用异步函数
            // let response = handle.await.unwrap();
            println!("response >> {:?}", response);

            Ok(())
        });


        t.await;

    }

    Ok(())
}