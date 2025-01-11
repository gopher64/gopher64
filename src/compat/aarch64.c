#include "src/compat/simd_aarch64.h"

// Static wrappers

__m128i _mm_srli_epi16__extern(__m128i __a, int __imm8) { return _mm_srli_epi16(__a, __imm8); }
