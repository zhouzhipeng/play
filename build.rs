use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use walkdir::{DirEntry, WalkDir};

const HOOKS_PATH: &str = ".git/hooks";
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


    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rustc-cfg=ENV=\"{}\"", option_env!("ENV").unwrap_or("dev"));
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
