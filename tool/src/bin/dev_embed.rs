use std::env::set_var;
use std::path::Path;
use std::process::Command;
use tool::{build_dev, build_python_artifacts};

fn main() {
    if let Err(e) = run(){
        println!("dev_embed error >> {:?}", e);
    };

}
fn run()->anyhow::Result<()>{
    build_dev("use_embed_python")?;
    Ok(())
}