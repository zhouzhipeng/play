#[derive(Debug)]
struct Car{
  components : Vec<String>
}


fn main() {

    let owner = Car{components: vec!["motor".to_string()]};

    let friend2= &owner;
    let friend1= &owner;
    println!("friend1 {:?}", friend1);
    println!("friend2 {:?}", friend2);

}