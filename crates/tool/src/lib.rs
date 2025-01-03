use std::process::Command;


pub fn build_dev(features: &str)->anyhow::Result<()>{

    Command::new("cargo")
        .args(["build","--locked", "--package", "play","--release", &format!("--features={}",features)])
        .spawn()?.wait()?;


    Ok(())
}