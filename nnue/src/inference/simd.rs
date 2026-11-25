use wide::{f32x16, i16x32};

pub type SimdF32 = f32x16;
pub type SimdI16 = i16x32;

pub const SIMD_WIDTH_F32: usize = 16;
pub const SIMD_WIDTH_I16: usize = 32;

// Different architectures can keep a different number of SIMD vectors in registers at once.
// Needed because stuff like dot products can then load the large input vector once per batch instead of once per row (which is expensive).
// Tuned (ish).
// Future me: This was actually not the case, and it is CPU specific. So just hardcode 4 for now since it is faster...
// TODO: Look more into this.
pub const SIMD_BATCH_SIZE: usize = 4;

pub fn simd_relu(values: &mut [f32]) {
    let zeros = SimdF32::splat(0.0);

    let (chunks, _) = values.as_chunks_mut::<SIMD_WIDTH_F32>();

    for chunk in chunks {
        *chunk = *SimdF32::new(*chunk).max(zeros).as_array();
    }
}

pub fn simd_add(dest: &mut [f32], src: &[f32]) {
    let (dest, _) = dest.as_chunks_mut::<SIMD_WIDTH_F32>();
    let (src, _) = src.as_chunks::<SIMD_WIDTH_F32>();

    for (dest, src) in dest.iter_mut().zip(src) {
        let sum = SimdF32::new(*dest) + SimdF32::new(*src);
        *dest = *sum.as_array();
    }
}

pub fn simd_dot_product(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_vec = SimdF32::splat(0.0);

    let (a, _) = a.as_chunks::<SIMD_WIDTH_F32>();
    let (b, _) = b.as_chunks::<SIMD_WIDTH_F32>();

    for (a, b) in a.iter().zip(b) {
        sum_vec += SimdF32::new(*a) * SimdF32::new(*b);
    }

    sum_vec.reduce_add()
}

pub fn simd_dot_product_batched<const N: usize>(input: &[f32], rows: [&[f32]; N]) -> [f32; N] {
    let len = input.len();
    let (input, _) = input.as_chunks::<SIMD_WIDTH_F32>();
    let rows = rows.map(|row| row[..len].as_chunks::<SIMD_WIDTH_F32>().0);
    let mut sums = [SimdF32::splat(0.0); N];

    for (i, chunk) in input.iter().enumerate() {
        let x = SimdF32::new(*chunk);
        for (sum, weights) in sums.iter_mut().zip(&rows) {
            *sum = x.mul_add(SimdF32::new(weights[i]), *sum);
        }
    }

    sums.map(|sum| sum.reduce_add())
}
