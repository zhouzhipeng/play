use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;
use fs_extra::dir::CopyOptions;

use walkdir::{DirEntry, WalkDir};
use wasm_pack::command::build::{BuildOptions, Target};
use wasm_pack::command::run_wasm_pack;

const HOOKS_PATH: &str = "../.git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo test
";

fn is_mod_file(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy() == "mod.rs"
}

fn main() {

    //test code.
    test();

    //generate git pre-commit file.
    gen_pre_commit();


    //check if you forgot to add your new rust file into mod.rs
    check_mod_files();

    if Ok("release".to_owned()) != env::var("PROFILE") {
        //will trigger deadlock when build --release
        //copy wasm files from `client` crate
        copy_wasm_files();
    }



    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");

    let env = if cfg!(feature = "dev"){"dev"}else if cfg!(feature = "prod"){"prod"}else{"dev"};

    // let env = option_env!("ENV").unwrap_or("dev");
    println!("cargo:rustc-cfg=ENV=\"{}\"",env );
    println!("cargo:rustc-env=ENV={}",env );


}



fn copy_wasm_files(){
    let client_path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("client");
    // fs::remove_dir_all(client_path.join("pkg"));
     run_wasm_pack(  wasm_pack::command::Command::Build(BuildOptions{
         path: Some(client_path),
         out_dir: "pkg".to_string(),
         release: true,
         target: Target::Web,
         // extra_options: vec!{"--target-dir".to_string(), "target2".to_string()},
         ..BuildOptions::default()
     })).expect("wasm-pack failed") ;

    let from_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(Path::new("client/pkg"));
    fs_extra::dir::copy(from_dir, Path::new(env!("CARGO_MANIFEST_DIR")).join("static/wasm"), &CopyOptions::new().overwrite(true)).expect("copy wasm files failed!");
}

fn check_mod_files() {
    let walker = WalkDir::new("src").into_iter();

    for entry in walker {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".rs") && name!="mod.rs" && name!="lib.rs" && name!="main.rs"{
            let file_stem = Path::new(&name).file_stem().unwrap().to_str().unwrap();
            let parent_mod_file = entry.path().parent().unwrap().join("mod.rs");
            if parent_mod_file.exists() {
                let mut f = File::open(&parent_mod_file).unwrap();
                let mut mod_contents = String::new();
                f.read_to_string(&mut mod_contents).unwrap();
                let expected = format!("mod {};", file_stem);
                if !mod_contents.contains(&expected) {
                    panic!("File {}.rs is not included in mod.rs", file_stem);
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
        .mode(0o755)
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
