#include "src/compat/sse2neon/v1.8.0/sse2neon.hpp"

// Static wrappers

__m128i _mm_srli_epi16__extern(__m128i __a, int __imm8) { return _mm_srli_epi16(__a, __imm8); }

__m128i _mm_insert_epi8__extern(__m128i __a, int __i, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_insert_epi8(__a, __i, 0);
    case 1:
        return _mm_insert_epi8(__a, __i, 1);
    case 2:
        return _mm_insert_epi8(__a, __i, 2);
    case 3:
        return _mm_insert_epi8(__a, __i, 3);
    case 4:
        return _mm_insert_epi8(__a, __i, 4);
    case 5:
        return _mm_insert_epi8(__a, __i, 5);
    case 6:
        return _mm_insert_epi8(__a, __i, 6);
    case 7:
        return _mm_insert_epi8(__a, __i, 7);
    case 8:
        return _mm_insert_epi8(__a, __i, 8);
    case 9:
        return _mm_insert_epi8(__a, __i, 9);
    case 10:
        return _mm_insert_epi8(__a, __i, 10);
    case 11:
        return _mm_insert_epi8(__a, __i, 11);
    case 12:
        return _mm_insert_epi8(__a, __i, 12);
    case 13:
        return _mm_insert_epi8(__a, __i, 13);
    case 14:
        return _mm_insert_epi8(__a, __i, 14);
    case 15:
        return _mm_insert_epi8(__a, __i, 15);
    default:
        abort();
    }
}

int _mm_extract_epi8__extern(__m128i __a, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_extract_epi8(__a, 0);
    case 1:
        return _mm_extract_epi8(__a, 1);
    case 2:
        return _mm_extract_epi8(__a, 2);
    case 3:
        return _mm_extract_epi8(__a, 3);
    case 4:
        return _mm_extract_epi8(__a, 4);
    case 5:
        return _mm_extract_epi8(__a, 5);
    case 6:
        return _mm_extract_epi8(__a, 6);
    case 7:
        return _mm_extract_epi8(__a, 7);
    case 8:
        return _mm_extract_epi8(__a, 8);
    case 9:
        return _mm_extract_epi8(__a, 9);
    case 10:
        return _mm_extract_epi8(__a, 10);
    case 11:
        return _mm_extract_epi8(__a, 11);
    case 12:
        return _mm_extract_epi8(__a, 12);
    case 13:
        return _mm_extract_epi8(__a, 13);
    case 14:
        return _mm_extract_epi8(__a, 14);
    case 15:
        return _mm_extract_epi8(__a, 15);
    default:
        abort();
    }
}

__m128i _mm_insert_epi16__extern(__m128i __a, int __i, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_insert_epi16(__a, __i, 0);
    case 1:
        return _mm_insert_epi16(__a, __i, 1);
    case 2:
        return _mm_insert_epi16(__a, __i, 2);
    case 3:
        return _mm_insert_epi16(__a, __i, 3);
    case 4:
        return _mm_insert_epi16(__a, __i, 4);
    case 5:
        return _mm_insert_epi16(__a, __i, 5);
    case 6:
        return _mm_insert_epi16(__a, __i, 6);
    case 7:
        return _mm_insert_epi16(__a, __i, 7);
    default:
        abort();
    }
}

int _mm_extract_epi16__extern(__m128i __a, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_extract_epi16(__a, 0);
    case 1:
        return _mm_extract_epi16(__a, 1);
    case 2:
        return _mm_extract_epi16(__a, 2);
    case 3:
        return _mm_extract_epi16(__a, 3);
    case 4:
        return _mm_extract_epi16(__a, 4);
    case 5:
        return _mm_extract_epi16(__a, 5);
    case 6:
        return _mm_extract_epi16(__a, 6);
    case 7:
        return _mm_extract_epi16(__a, 7);
    default:
        abort();
    }
}

__m128i _mm_insert_epi32__extern(__m128i __a, int __i, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_insert_epi32(__a, __i, 0);
    case 1:
        return _mm_insert_epi32(__a, __i, 1);
    case 2:
        return _mm_insert_epi32(__a, __i, 2);
    case 3:
        return _mm_insert_epi32(__a, __i, 3);
    default:
        abort();
    }
}

int _mm_extract_epi32__extern(__m128i __a, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_extract_epi32(__a, 0);
    case 1:
        return _mm_extract_epi32(__a, 1);
    case 2:
        return _mm_extract_epi32(__a, 2);
    case 3:
        return _mm_extract_epi32(__a, 3);
    default:
        abort();
    }
}

__m128i _mm_insert_epi64__extern(__m128i __a, int64_t __i, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_insert_epi64(__a, __i, 0);
    case 1:
        return _mm_insert_epi64(__a, __i, 1);
    default:
        abort();
    }
}

int64_t _mm_extract_epi64__extern(__m128i __a, const int __imm8)
{
    switch (__imm8)
    {
    case 0:
        return _mm_extract_epi64(__a, 0);
    case 1:
        return _mm_extract_epi64(__a, 1);
    default:
        abort();
    }
}
