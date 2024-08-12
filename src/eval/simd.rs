#[cfg(target_feature = "avx2")]
pub(crate) mod avx2 {
    use std::arch::x86_64::*;

    use crate::eval::network::{RELU_MAX, RELU_MIN};
    use crate::eval::{Block, HIDDEN_SIZE};

    const CHUNK_SIZE: usize = 16;
    /// Number of SIMD vectors contained within one hidden layer
    const REQUIRED_ITERS: usize = HIDDEN_SIZE / CHUNK_SIZE;

    #[inline]
    pub unsafe fn flatten(acc: &Block, weights: &Block) -> i32 {
        {
            let mut sum = _mm256_setzero_si256();
            for i in 0..REQUIRED_ITERS {
                let us_vector = _mm256_load_si256(acc.as_ptr().add(i * CHUNK_SIZE).cast());
                let weights = _mm256_load_si256(weights.as_ptr().add(i * CHUNK_SIZE).cast());
                let crelu_result = clipped_relu(us_vector);
                let v = _mm256_mullo_epi16(crelu_result, weights);
                let mul = _mm256_madd_epi16(v, crelu_result);
                sum = _mm256_add_epi32(sum, mul);
            }
            hadd_i32(sum)
        }
    }

    #[inline]
    unsafe fn hadd_i32(sum: __m256i) -> i32 {
        let upper_128 = _mm256_extracti128_si256::<1>(sum);
        let lower_128 = _mm256_castsi256_si128(sum);
        let sum_128 = _mm_add_epi32(upper_128, lower_128);

        let upper_64 = _mm_unpackhi_epi64(sum_128, sum_128);
        let sum_64 = _mm_add_epi32(upper_64, sum_128);

        let upper_32 = _mm_shuffle_epi32::<0b00_00_00_01>(sum_64);
        let sum_32 = _mm_add_epi32(upper_32, sum_64);

        _mm_cvtsi128_si32(sum_32)
    }

    #[inline]
    unsafe fn clipped_relu(i: __m256i) -> __m256i {
        let min = _mm256_set1_epi16(RELU_MIN);
        let max = _mm256_set1_epi16(RELU_MAX);

        _mm256_min_epi16(_mm256_max_epi16(i, min), max)
    }
}
