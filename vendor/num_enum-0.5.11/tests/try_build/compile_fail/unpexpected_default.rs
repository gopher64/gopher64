#[derive(num_enum::UnsafeFromPrimitive)]
#[repr(u8)]
enum Numbers {
    Zero,
    #[num_enum(default)]
    NoneZero,
}

fn main() {

}
