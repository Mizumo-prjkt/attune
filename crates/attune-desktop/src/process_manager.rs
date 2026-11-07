//! Dynamic Process & Hardware Resource Manager Bridge for Attune Desktop.
//!
//! Relies strictly on the external `libattune_procmgr.so` / `attune_procmgr.dll`
//! dynamic shared library component loaded via `crate::dynamic_components`.
//!
//! Mandate 4: Zero hardcoded fallback values or silent static fallbacks. Any
//! failure in the underlying dynamic component is propagated as `Err(String)`
//! so the UI can report it transparently.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessMetrics {
    // Memory
    pub rss_bytes: u64,
    pub rss_peak_bytes: u64,
    pub vmem_bytes: u64,

    // CPU & Threads
    pub cpu_usage_pct: f32,
    pub thread_count: u32,
    pub voluntary_ctxt_switches: u64,
    pub nonvoluntary_ctxt_switches: u64,

    // GPU & Display
    pub gpu_device_name: String,
    pub gpu_driver: String,
    pub gpu_cur_freq_mhz: Option<u32>,
    pub gpu_max_freq_mhz: Option<u32>,
    pub gpu_hardware_accel: String,

    // Audio IRQ & DAC
    pub audio_irq_number: Option<u32>,
    pub audio_irq_device: String,
    pub audio_irq_count: u64,
    pub audio_irq_rate_per_sec: f32,
    pub alsa_stream_state: String,
    pub alsa_hw_params: String,

    // Timestamp
    pub timestamp_ms: u64,
}

/// Collects live process telemetry strictly from the external dynamic shared library component.
pub fn sample_metrics() -> Result<ProcessMetrics, String> {
    crate::dynamic_components::procmgr::sample_metrics()
}

/// Cleans process memory strictly via the external dynamic shared library component.
/// Returns `Ok((before_bytes, after_bytes))` or `Err(diagnostic)`.
pub fn clean_process_memory() -> Result<(u64, u64), String> {
    crate::dynamic_components::procmgr::clean_process_memory()
}

/// Alias for memory cleanup across stages.
pub fn trim_memory() -> Result<(u64, u64), String> {
    clean_process_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_procmgr_metrics_via_dynamic_component() {
        let _ = crate::dynamic_components::preflight_check_and_load();
        let m = sample_metrics().expect("sample_metrics must succeed via loaded dylib");
        assert!(m.timestamp_ms > 0);
        assert!(m.thread_count >= 1);
    }
}
