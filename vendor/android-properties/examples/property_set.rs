fn main() {
    android_properties::setprop("hello.world", "hello").expect("Cannot set android property");
}
