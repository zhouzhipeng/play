use std::env::set_var;
use std::path::Path;
use std::process::Command;
use tool::build_python_artifacts;

fn main()->anyhow::Result<()>{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_str().unwrap();
    println!("workspace root : {}", root);
    build_python_artifacts();
    set_var("PYO3_CONFIG_FILE", format!("{}/server/python/build/pyo3-build-config-file.txt", root));

    Command::new("cargo")
        .args(["build","--package", "play","--release", "--features=use_embed_python"])
        .spawn()?.wait()?;


    Ok(())
}