fn main() {
    //generate rustc args.
    println!("cargo:rustc-cfg=ENV=\"{}\"", option_env!("ENV").unwrap_or("dev"));
}



