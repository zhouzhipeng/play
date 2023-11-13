use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const HOOKS_PATH: &str = ".git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo test
";

fn main() {

    //test code.
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("hello.rs");

    fs::write(
        &dest_path,
        "pub fn message() -> &'static str {
            \"Hello, World!\"
        }
        "
    ).unwrap();

    //generate git pre-commit file.
    fs::create_dir_all(HOOKS_PATH).unwrap();

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o755)
        .open(format!("{}/pre-commit", HOOKS_PATH))
        .unwrap();

    file.write_all(PRE_COMMIT_HOOK.as_bytes()).unwrap();




    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rustc-cfg=ENV=\"{}\"", option_env!("ENV").unwrap_or("dev"));
}