use std::env;

fn main() {
    //generate rustc args.
    let env = if cfg!(feature = "dev"){"dev"}else if cfg!(feature = "prod"){"prod"}else{"dev"};
    println!("cargo:rustc-cfg=ENV=\"{}\"", env);
}



