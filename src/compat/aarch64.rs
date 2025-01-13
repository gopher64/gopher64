unsafe extern "C" {
    #[link_name = "_mm_srli_epi16__extern"]
    pub fn _mm_srli_epi16(__a: __m128i, __imm8: ::std::os::raw::c_int) -> __m128i;
}

unsafe extern "C" {
    #[link_name = "_mm_insert_epi8__extern"]
    pub fn _mm_insert_epi8(
        __a: __m128i,
        __i: ::std::os::raw::c_int,
        __imm8: ::std::os::raw::c_int,
    ) -> __m128i;
}

unsafe extern "C" {
    #[link_name = "_mm_extract_epi8__extern"]
    pub fn _mm_extract_epi8(__a: __m128i, __imm8: ::std::os::raw::c_int) -> i32;
}

unsafe extern "C" {
    #[link_name = "_mm_insert_epi16__extern"]
    pub fn _mm_insert_epi16(
        __a: __m128i,
        __i: ::std::os::raw::c_int,
        __imm8: ::std::os::raw::c_int,
    ) -> __m128i;
}

unsafe extern "C" {
    #[link_name = "_mm_extract_epi16__extern"]
    pub fn _mm_extract_epi16(__a: __m128i, __imm8: ::std::os::raw::c_int) -> i32;
}
