//! Dynamic Process & Hardware Resource Manager Bridge for Attune Desktop.
//!
//! Relies strictly on the external `libattune_procmgr.so` / `attune_procmgr.dll`
//! dynamic shared library component loaded via `crate::dynamic_components`.

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

impl Default for ProcessMetrics {
    fn default() -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            rss_bytes: 0,
            rss_peak_bytes: 0,
            vmem_bytes: 0,
            cpu_usage_pct: 0.0,
            thread_count: 1,
            voluntary_ctxt_switches: 0,
            nonvoluntary_ctxt_switches: 0,
            gpu_device_name: "Hardware Graphics Adapter".to_string(),
            gpu_driver: "Native Display Driver".to_string(),
            gpu_cur_freq_mhz: None,
            gpu_max_freq_mhz: None,
            gpu_hardware_accel: "WebKitGTK / WebView2 Accelerated Compositing".to_string(),
            audio_irq_number: None,
            audio_irq_device: "System Audio Hardware".to_string(),
            audio_irq_count: 0,
            audio_irq_rate_per_sec: 0.0,
            alsa_stream_state: "Active Hardware Stream".to_string(),
            alsa_hw_params: "Direct DAC Output Link".to_string(),
            timestamp_ms: ts,
        }
    }
}

/// Collects live process telemetry strictly from the external dynamic shared library component.
pub fn sample_metrics() -> ProcessMetrics {
    match crate::dynamic_components::procmgr::sample_metrics() {
        Ok(m) => m,
        Err(e) => {
            log::error!("ProcessManager: Failed to sample metrics from dynamic component: {e}");
            ProcessMetrics::default()
        }
    }
}

/// Cleans process memory strictly via the external dynamic shared library component.
/// Returns (before_bytes, after_bytes).
pub fn clean_process_memory() -> (u64, u64) {
    match crate::dynamic_components::procmgr::clean_process_memory() {
        Ok(res) => res,
        Err(e) => {
            log::error!("ProcessManager: Failed to clean process memory via dynamic component: {e}");
            (0, 0)
        }
    }
}

/// Backward compatibility alias for existing tests and call sites.
pub fn trim_memory() -> (u64, u64) {
    clean_process_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_process_metrics_valid() {
        let m = ProcessMetrics::default();
        assert!(m.timestamp_ms > 0);
        assert_eq!(m.thread_count, 1);
    }
}
