use std::env::set_var;
use std::fs;
use std::path::Path;
use std::process::Command;
use tool::{build_dev, build_python_artifacts};

fn main() {
    std::env::set_var("RUST_BACKTRACE","1");

    if let Err(e)=run(){
        println!("dev_ui build error >> {:?}", e);
    }
}


fn run()->anyhow::Result<()>{
    build_dev("use_embed_python,ui,job")?;

    let app_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("target/release").join("play.app");

    fs::remove_dir_all(&app_dir);

    let Contents_dir = app_dir.join("Contents");
    let Resources_dir = Contents_dir.join("Resources");
    let MacOS_dir = Contents_dir.join("MacOS");

    fs::create_dir_all(&Resources_dir)?;
    fs::create_dir_all(&MacOS_dir)?;

    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("crates/play_ui/res/Info.plist"), &Contents_dir.join("Info.plist"))?;
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("target/release/play"), &MacOS_dir.join("play"))?;
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
                 .join("crates/play_ui/icon.icns"), &Resources_dir.join("icon.icns"))?;




    Ok(())
}