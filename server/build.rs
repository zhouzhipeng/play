use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

use shared::current_timestamp;

const HOOKS_PATH: &str = "../.git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo build
";

fn main() {


    println!("cargo:rustc-env=BUILT_TIME={}",current_timestamp!().to_string() );

    //generate git pre-commit file.
    #[cfg(feature = "debug")]
    gen_pre_commit();

    //check if you forgot to add your new rust file into mod.rs
    // #[cfg(feature = "debug")]
    // check_mod_files();


    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=doc");

}


fn check_mod_files() {
    let walker = WalkDir::new("src").into_iter();

    for entry in walker {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".rs") && name != "mod.rs" && name != "lib.rs" && name != "main.rs" {
            let file_stem = Path::new(&name).file_stem().unwrap().to_str().unwrap();
            let parent_mod_file = entry.path().parent().unwrap().join("mod.rs");
            if parent_mod_file.exists() {
                let mut f = File::open(&parent_mod_file).unwrap();
                let mut mod_contents = String::new();
                f.read_to_string(&mut mod_contents).unwrap();
                let expected = format!("mod {};", file_stem);
                if !mod_contents.contains(&expected) {
                    panic!("Error >> File {}.rs is not included in mod.rs", file_stem);
                }

                mod_contents = mod_contents.replace(&expected, "");

                if file_stem.ends_with("_controller") {
                    if mod_contents.rfind(&file_stem).is_none() {
                        panic!("Error >> you need to register controller: {}  in controller/mod.rs", file_stem);
                    }
                }
            }
        }
    }
}

fn gen_pre_commit() {
    fs::create_dir_all(HOOKS_PATH).unwrap();

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        // .mode(0o755)
        .open(format!("{}/pre-commit", HOOKS_PATH))
        .unwrap();

    file.write_all(PRE_COMMIT_HOOK.as_bytes()).unwrap();
}

fn test() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("hello.rs");

    fs::write(
        &dest_path,
        "pub fn message() -> &'static str {
            \"Hello, World!\"
        }
        ",
    ).unwrap();
}



fn increase_app_version()->anyhow::Result<()>{
    let cargo_toml_path = "Cargo.toml";
    let cargo_toml_contents = fs::read_to_string(cargo_toml_path)?;

    let version_regex = Regex::new(r#"^version = "(\d+)\.(\d+)\.(\d+)"$"#).unwrap();
    let new_contents = version_regex.replace(&cargo_toml_contents, |caps: &regex::Captures| {
        let major: i32 = caps[1].parse().unwrap();
        let minor: i32 = caps[2].parse().unwrap();
        let mut patch: i32 = caps[3].parse().unwrap();
        patch += 1; // Increment the patch version
        format!("version = \"{}.{}.{}\"", major, minor, patch)
    });

    let mut file = fs::File::create(cargo_toml_path)?;
    file.write_all(new_contents.as_bytes())?;
    Ok(())

}