use std::env::set_var;
use std::path::Path;
use std::process::Command;
use tool::{build_dev, build_python_artifacts};

fn main() {
    std::env::set_var("RUST_BACKTRACE","1");

    if let Err(e) = run(){
        println!("dev_mail error >> {:?}", e);
    };

}
fn run()->anyhow::Result<()>{
    build_dev("use_embed_python,mail_server")?;
    Ok(())
}