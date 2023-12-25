use tool::build_python_artifacts;

fn main() {
    std::env::set_var("RUST_BACKTRACE","1");


    if let Err(e)=build_python_artifacts(){
        println!("build_python_artifacts error >> {:?}", e);
    }
}
