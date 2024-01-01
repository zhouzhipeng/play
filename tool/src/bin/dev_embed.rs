use tool::build_dev;

fn main() {
    std::env::set_var("RUST_BACKTRACE","1");

    if let Err(e) = run(){
        println!("dev_embed error >> {:?}", e);
    };

}
fn run()->anyhow::Result<()>{
    build_dev("use_embed_python")?;
    Ok(())
}