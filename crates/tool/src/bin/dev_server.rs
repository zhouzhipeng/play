use tool::build_dev;

fn main() {
    std::env::set_var("RUST_BACKTRACE","1");

    if let Err(e) = run(){
        println!("dev_server error >> {:?}", e);
    };

}
fn run()->anyhow::Result<()>{
    // must use use_embed_python(not tpl , because the debian server may miss some python libs)
    // below error when startup:
    /// ModuleNotFoundError: No module named 'encodings'
    build_dev("play-https,play-dylib-loader")?;
    Ok(())
}