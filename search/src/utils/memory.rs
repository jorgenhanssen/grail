// Prefetch a memory address into CPU cache
#[inline(always)]
pub unsafe fn prefetch(ptr: *const u8) {
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64: Use SSE prefetch instruction (T0 = prefetch to all cache levels)
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(ptr as *const i8);
    }

    #[cfg(target_arch = "aarch64")]
    {
        // ARM: PRFM (prefetch memory) instruction - prefetch for read into L1 cache
        // https://developer.arm.com/documentation/ddi0596/2021-06/Base-Instructions/PRFM--immediate---Prefetch-Memory--immediate--
        std::arch::asm!(
            "prfm pldl1keep, [{ptr}]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly)
        );
    }

    // Noop on other architectures
}
