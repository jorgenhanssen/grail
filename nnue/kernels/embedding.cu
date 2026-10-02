// Beware! The following code was assisted with an LLM due to skill issue.
// If you are allergic to such code, close this file immediately.


// Adapted from Stockfish's sparse feature transformer:
// https://github.com/official-stockfish/nnue-pytorch/blob/d56672e4a157d8f47df6819e353af5a421bcceb5/model/modules/feature_transformer/sparse_linear_kernel.py

#include <stddef.h>
#include <stdint.h>

// Must match BLOCK_SIZE and OUTPUTS_PER_THREAD in embedding.rs.
constexpr unsigned kBlockSize = 256;
constexpr unsigned kOutputsPerThread = 3;
constexpr unsigned kEmbeddingSize = kBlockSize * kOutputsPerThread;
static_assert(kEmbeddingSize == 768, "must match EMBEDDING_SIZE");

extern "C" __global__ void embedding_fwd_f32(
    const uint32_t* __restrict__ stm_indices,
    const uint32_t* __restrict__ nstm_indices,
    const float* __restrict__ weights,
    float* __restrict__ output,
    uint32_t batch_len,
    uint32_t indices_per_sample,
    uint32_t num_features
) {
    const uint32_t sample = blockIdx.x;
    const uint32_t perspective = blockIdx.y;
    if (sample >= batch_len || perspective >= 2 || threadIdx.x >= kBlockSize) {
        return;
    }

    const uint32_t slice_offset = threadIdx.x;
    const uint32_t* indices = perspective == 0 ? stm_indices : nstm_indices;
    const uint32_t* input_index_row =
        indices + static_cast<size_t>(sample) * indices_per_sample;

    // The rest of the network expects stm first, followed by nstm.
    float* output_slice =
        output + (static_cast<size_t>(sample) * 2 + perspective) * kEmbeddingSize + slice_offset;

    // Spacing each thread's outputs one block apart keeps neighboring threads
    // reading neighboring weights.
    float acc[kOutputsPerThread] = {};

    for (uint32_t k = 0; k < indices_per_sample; ++k) {
        const uint32_t input_index = input_index_row[k];

        // Padding follows the active indices, so the rest of the row can be skipped.
        if (input_index >= num_features) {
            break;
        }

        const float* weight_slice =
            weights + static_cast<size_t>(input_index) * kEmbeddingSize + slice_offset;

        #pragma unroll
        for (uint32_t s = 0; s < kOutputsPerThread; ++s) {
            acc[s] += weight_slice[s * kBlockSize];
        }
    }

    #pragma unroll
    for (uint32_t s = 0; s < kOutputsPerThread; ++s) {
        output_slice[s * kBlockSize] = acc[s];
    }
}

// The caller must zero weight_grad because samples and perspectives add to
// the same weights.
extern "C" __global__ void embedding_bwd_f32(
    const uint32_t* __restrict__ stm_indices,
    const uint32_t* __restrict__ nstm_indices,
    const float* __restrict__ grad_output,
    float* __restrict__ weight_grad,
    uint32_t batch_len,
    uint32_t indices_per_sample,
    uint32_t num_features
) {
    const uint32_t sample = blockIdx.x;
    const uint32_t perspective = blockIdx.y;
    if (sample >= batch_len || perspective >= 2 || threadIdx.x >= kBlockSize) {
        return;
    }

    const uint32_t slice_offset = threadIdx.x;
    const uint32_t* indices = perspective == 0 ? stm_indices : nstm_indices;
    const uint32_t* input_index_row =
        indices + static_cast<size_t>(sample) * indices_per_sample;
    const float* output_grad_slice =
        grad_output + (static_cast<size_t>(sample) * 2 + perspective) * kEmbeddingSize + slice_offset;

    float grad[kOutputsPerThread];

    #pragma unroll
    for (uint32_t s = 0; s < kOutputsPerThread; ++s) {
        grad[s] = output_grad_slice[s * kBlockSize];
    }

    for (uint32_t k = 0; k < indices_per_sample; ++k) {
        const uint32_t input_index = input_index_row[k];
        if (input_index >= num_features) {
            break;
        }

        float* weight_grad_slice =
            weight_grad + static_cast<size_t>(input_index) * kEmbeddingSize + slice_offset;

        #pragma unroll
        for (uint32_t s = 0; s < kOutputsPerThread; ++s) {
            if (grad[s] != 0.0f) {
                atomicAdd(&weight_grad_slice[s * kBlockSize], grad[s]);
            }
        }
    }
}