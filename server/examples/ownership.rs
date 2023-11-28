#[derive(Debug)]
struct Car{
  components : Vec<String>
}

trait Vehicles{
    fn drive(&self);
}

impl Vehicles for Car{
    fn drive(&self) {
        println!("drive1 ")
    }
}


fn main() {


    let owner = Car{components: vec!["motor".to_string()]};

    owner.drive();
}