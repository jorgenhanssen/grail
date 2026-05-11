/// Hint the CPU to pull ptr into L1, used to hide TT probe latency.
///
/// # Safety
/// The pointer must be valid for reading. The target memory doesn't need to be
/// initialized (never actually read).
pub unsafe fn prefetch(ptr: *const u8) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(ptr as *const i8);
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        std::arch::asm!(
            "prfm pldl1keep, [{ptr}]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly)
        );
    }
}
