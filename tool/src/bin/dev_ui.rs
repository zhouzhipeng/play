use std::env::set_var;
use std::fs;
use std::path::Path;
use std::process::Command;
use tool::build_python_artifacts;




fn main()->anyhow::Result<()>{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_str().unwrap();
    println!("workspace root : {}", root);
    build_python_artifacts();
    set_var("PYO3_CONFIG_FILE", format!("{}/server/python/build/pyo3-build-config-file.txt", root));

    Command::new("cargo")
        .args(["build","--package", "play","--release", "--features=use_embed_python,ui"])
        .spawn()?.wait()?;

    let app_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("target/release").join("play.app");

    fs::remove_dir_all(&app_dir)?;

    let Contents_dir = app_dir.join("Contents");
    let Resources_dir = Contents_dir.join("Resources");
    let MacOS_dir = Contents_dir.join("MacOS");

    fs::create_dir_all(&Resources_dir)?;
    fs::create_dir_all(&MacOS_dir)?;

    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("libs/ui/res/Info.plist"), &Contents_dir.join("Info.plist"))?;
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
        .join("target/release/play"), &MacOS_dir.join("play"))?;
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
                 .join("libs/ui/icon.icns"), &Resources_dir.join("icon.icns"))?;




    Ok(())
}