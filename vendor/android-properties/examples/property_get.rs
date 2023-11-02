use android_properties::AndroidProperty;

const HELLO_WORLD_PROPERTY: &str = "hello.world";

fn main() {
    let mut hello_world = AndroidProperty::new(HELLO_WORLD_PROPERTY);
    match hello_world.value() {
        Some(_value) => println!("{}", hello_world),
        None => println!("Property {} not found", hello_world.name()),
    };
}
