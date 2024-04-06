use tool::build_python_artifacts;

fn main() {


    if let Err(e)=build_python_artifacts(){
        println!("build_python_artifacts error >> {:?}", e);
    }
}
