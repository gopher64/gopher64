#[derive(num_enum::UnsafeFromPrimitive)]
#[repr(u8)]
enum Numbers {
    Zero,
    #[num_enum(alternatives = [2])]
    One,
}

fn main() {

}
