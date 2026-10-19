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

    for chunk in values.chunks_exact_mut(SIMD_WIDTH_F32) {
        let activated = SimdF32::new(chunk.try_into().unwrap()).max(zeros);
        chunk.copy_from_slice(activated.as_array());
    }
}

pub fn simd_add(dest: &mut [f32], src: &[f32]) {
    for (dest, src) in dest
        .chunks_exact_mut(SIMD_WIDTH_F32)
        .zip(src.chunks_exact(SIMD_WIDTH_F32))
    {
        let d = SimdF32::new(dest.try_into().unwrap());
        let s = SimdF32::new(src.try_into().unwrap());
        dest.copy_from_slice((d + s).as_array());
    }
}

pub fn simd_dot_product(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_vec = SimdF32::splat(0.0);

    for (a, b) in a
        .chunks_exact(SIMD_WIDTH_F32)
        .zip(b.chunks_exact(SIMD_WIDTH_F32))
    {
        let a_vec = SimdF32::new(a.try_into().unwrap());
        let b_vec = SimdF32::new(b.try_into().unwrap());
        sum_vec += a_vec * b_vec;
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
            *sum += x * SimdF32::new(weights[i]);
        }
    }

    sums.map(|sum| sum.reduce_add())
}
