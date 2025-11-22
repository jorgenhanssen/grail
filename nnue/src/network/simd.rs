use std::simd::prelude::SimdFloat;
use std::simd::{f32x16, i16x32};

pub type SimdF32 = f32x16;
pub type SimdI16 = i16x32;

pub const SIMD_WIDTH_F32: usize = 16;
pub const SIMD_WIDTH_I16: usize = 32;

#[inline(always)]
pub fn simd_relu(values: &mut [f32]) {
    let len = values.len();
    let mut i = 0;
    let zeros = SimdF32::splat(0.0);

    while i + SIMD_WIDTH_F32 <= len {
        let chunk = SimdF32::from_slice(&values[i..i + SIMD_WIDTH_F32]);
        chunk
            .simd_max(zeros)
            .copy_to_slice(&mut values[i..i + SIMD_WIDTH_F32]);
        i += SIMD_WIDTH_F32;
    }

    for val in &mut values[i..len] {
        *val = val.max(0.0);
    }
}

#[inline(always)]
pub fn simd_add(dest: &mut [f32], src: &[f32]) {
    let len = dest.len();
    let mut i = 0;

    while i + SIMD_WIDTH_F32 <= len {
        let d = SimdF32::from_slice(&dest[i..i + SIMD_WIDTH_F32]);
        let s = SimdF32::from_slice(&src[i..i + SIMD_WIDTH_F32]);
        (d + s).copy_to_slice(&mut dest[i..i + SIMD_WIDTH_F32]);
        i += SIMD_WIDTH_F32;
    }

    for j in i..len {
        dest[j] += src[j];
    }
}

#[inline(always)]
pub fn dot_product(a: &[f32], b: &[f32], len: usize) -> f32 {
    let mut sum_vec = SimdF32::splat(0.0);
    let mut i = 0;

    while i + SIMD_WIDTH_F32 <= len {
        let a_vec = SimdF32::from_slice(&a[i..i + SIMD_WIDTH_F32]);
        let b_vec = SimdF32::from_slice(&b[i..i + SIMD_WIDTH_F32]);
        sum_vec += a_vec * b_vec;
        i += SIMD_WIDTH_F32;
    }

    let mut sum = sum_vec.reduce_sum();
    for j in i..len {
        sum += a[j] * b[j];
    }

    sum
}
