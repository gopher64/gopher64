use android_properties::{setprop, AndroidProperty};

const HELLO_WORLD_PROPERTY: &str = "hello.world";

fn main() {
    setprop(HELLO_WORLD_PROPERTY, "initial value").expect("Cannot set android property");
    let hello_world = AndroidProperty::new(HELLO_WORLD_PROPERTY);
    println!("Initial property: {}", hello_world);

    setprop(HELLO_WORLD_PROPERTY, "refreshed value").expect("Cannot set android property");
    println!("Refreshed property: {}", hello_world);
}
