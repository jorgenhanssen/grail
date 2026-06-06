// Custom kernel for Candle, adapted from the custom-ops example:
// https://github.com/huggingface/candle/blob/main/candle-examples/examples/custom-ops/kernels/layernorm_kernels.cu
// https://github.com/official-stockfish/nnue-pytorch/blob/master/docs/nnue.md

extern "C" __global__ void crelu_fwd_f32(const float *x, float *y, const int n) {
    const int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        y[i] = fminf(fmaxf(x[i], 0.0f), 1.0f);
    }
}

extern "C" __global__ void crelu_bwd_f32(const float *x, const float *gy, float *gx, const int n) {
    const int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        const float xi = x[i];
        gx[i] = (xi > 0.0f && xi < 1.0f) ? gy[i] : 0.0f;
    }
}
