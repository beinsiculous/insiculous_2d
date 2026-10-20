//! How much memory the process holds, for the loaded-memory figure of the budget.

/// The wasm module's linear memory (it only grows, so this is the high-water mark).
#[cfg(target_arch = "wasm32")]
pub fn resident_bytes() -> Option<u64> {
    const PAGE_BYTES: u64 = 65_536;
    Some(core::arch::wasm32::memory_size(0) as u64 * PAGE_BYTES)
}

/// The resident set size, from `/proc/self/statm` (Linux; `None` elsewhere).
#[cfg(not(target_arch = "wasm32"))]
pub fn resident_bytes() -> Option<u64> {
    const PAGE_BYTES: u64 = 4096;
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * PAGE_BYTES)
}

/// `resident_bytes` in MiB, or -1 when it cannot be read.
pub fn resident_mebibytes() -> f64 {
    resident_bytes().map_or(-1.0, |bytes| bytes as f64 / 1_048_576.0)
}
