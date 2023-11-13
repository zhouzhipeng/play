use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("hello.rs");

    fs::write(
        &dest_path,
        "pub fn message() -> &'static str {
            \"Hello, World!\"
        }
        "
    ).unwrap();


    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rustc-cfg=ENV=\"{}\"", option_env!("ENV").unwrap_or("dev"));
}