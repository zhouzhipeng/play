use play_shared::current_timestamp;

fn main() {
    println!(
        "cargo:rustc-env=BUILT_TIME={}",
        current_timestamp!().to_string()
    );
}
