/// Prefetch a memory address into CPU cache.
///
/// This is a hint to the CPU that we'll be accessing this memory soon,
/// allowing it to speculatively load the data into cache. This can
/// significantly reduce latency when accessing transposition table entries.
///
/// # Safety
/// The pointer must be valid for reading (though it doesn't need to point
/// to initialized memory, since we're only prefetching, not reading).
pub unsafe fn prefetch(ptr: *const u8) {
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64: Use SSE prefetch instruction (T0 = prefetch to all cache levels)
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(ptr as *const i8);
    }

    #[cfg(target_arch = "aarch64")]
    {
        // ARM: PRFM (prefetch memory) instruction - prefetch for read into L1 cache
        std::arch::asm!(
            "prfm pldl1keep, [{ptr}]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly)
        );
    }

    // No-op on other architectures
}
