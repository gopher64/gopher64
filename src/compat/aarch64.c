#include "src/compat/sse2neon.h"

// Static wrappers

__m128i _mm_srli_epi16__extern(__m128i __a, int __imm8) { return _mm_srli_epi16(__a, __imm8); }

__m128i _mm_insert_epi8__extern(__m128i __a, int __i, const int __imm8) { return _mm_insert_epi8(__a, __i, __imm8); }

int _mm_extract_epi8__extern(__m128i __a, const int __imm8) { return _mm_extract_epi8(__a, __imm8); }

__m128i _mm_insert_epi16__extern(__m128i __a, int __i, const int __imm8) { return _mm_insert_epi16(__a, __i, __imm8); }

int _mm_extract_epi16__extern(__m128i __a, const int __imm8) { return _mm_extract_epi16(__a, __imm8); }
