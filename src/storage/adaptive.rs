//! Adaptive block sizing controller based on latency and throughput.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    HDD,  // 4 MB initial
    SSD,  // 8 MB initial
    NVMe, // 16 MB initial
    USB,  // 2 MB initial
}

pub struct AdaptiveBlockSize {
    min_size: usize,
    max_size: usize,
    current_size: usize,
}

impl AdaptiveBlockSize {
    /// Create adaptive block sizer for device type
    pub fn new(device_type: DeviceType) -> Self {
        let (initial, min, max) = match device_type {
            DeviceType::HDD => (4 * 1024 * 1024, 1024 * 1024, 16 * 1024 * 1024),
            DeviceType::SSD => (8 * 1024 * 1024, 1024 * 1024, 16 * 1024 * 1024),
            DeviceType::NVMe => (16 * 1024 * 1024, 1024 * 1024, 16 * 1024 * 1024),
            DeviceType::USB => (2 * 1024 * 1024, 512 * 1024, 8 * 1024 * 1024),
        };

        AdaptiveBlockSize {
            min_size: min,
            max_size: max,
            current_size: initial,
        }
    }

    /// Adjust block size based on latency and throughput
    pub fn adjust(&mut self, latency_ms: f64, throughput_mbps: f64) {
        const MAX_LATENCY: f64 = 50.0;
        const MIN_LATENCY: f64 = 10.0;
        const ADJUSTMENT: usize = 1024 * 1024; // 1 MB

        if latency_ms > MAX_LATENCY {
            // High latency - decrease block size
            self.current_size = self
                .current_size
                .saturating_sub(ADJUSTMENT)
                .max(self.min_size);
        } else if latency_ms < MIN_LATENCY {
            // Low latency - check throughput
            let max_throughput = 1000.0; // Assume 1000 MB/s is max
            if throughput_mbps < max_throughput * 0.8 {
                // Under 80% max - increase block size
                self.current_size = (self.current_size + ADJUSTMENT).min(self.max_size);
            }
        }
    }

    /// Get current block size
    pub fn get_size(&self) -> usize {
        self.current_size
    }

    /// Minimum allowable block size
    pub fn min_size(&self) -> usize {
        self.min_size
    }

    /// Maximum allowable block size
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

/// Backward compatibility abstraction for chunk sizing.
pub struct AdaptiveChunker {
    current_chunk_size: usize,
    min_chunk_size: usize,
    max_chunk_size: usize,
}

impl AdaptiveChunker {
    pub fn new(initial: usize, min: usize, max: usize) -> Self {
        Self {
            current_chunk_size: initial,
            min_chunk_size: min,
            max_chunk_size: max,
        }
    }

    pub fn current_chunk_size(&self) -> usize {
        self.current_chunk_size
    }

    /// Evaluates I/O duration and throughput to adjust chunk size.
    pub fn observe(&mut self, duration_millis: u64, bytes: usize) {
        let latency_ms = duration_millis as f64;
        let throughput_mbps = if duration_millis > 0 {
            (bytes as f64 / (1024.0 * 1024.0)) / (duration_millis as f64 / 1000.0)
        } else {
            0.0
        };

        const MAX_LATENCY: f64 = 50.0;
        const MIN_LATENCY: f64 = 10.0;
        const ADJUSTMENT: usize = 1024 * 1024;

        if latency_ms > MAX_LATENCY {
            self.current_chunk_size = self
                .current_chunk_size
                .saturating_sub(ADJUSTMENT)
                .max(self.min_chunk_size);
        } else if latency_ms < MIN_LATENCY && throughput_mbps < 800.0 {
            self.current_chunk_size =
                (self.current_chunk_size + ADJUSTMENT).min(self.max_chunk_size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_hdd_defaults() {
        let adaptive = AdaptiveBlockSize::new(DeviceType::HDD);
        assert_eq!(adaptive.get_size(), 4 * 1024 * 1024);
    }

    #[test]
    fn test_adaptive_ssd_defaults() {
        let adaptive = AdaptiveBlockSize::new(DeviceType::SSD);
        assert_eq!(adaptive.get_size(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_adaptive_decrease_on_high_latency() {
        let mut adaptive = AdaptiveBlockSize::new(DeviceType::SSD);
        let initial = adaptive.get_size();

        adaptive.adjust(100.0, 100.0); // High latency
        assert!(adaptive.get_size() < initial);
    }

    #[test]
    fn test_device_type_initial_sizes() {
        let hdd = AdaptiveBlockSize::new(DeviceType::HDD);
        assert_eq!(hdd.get_size(), 4 * 1024 * 1024);
        assert_eq!(hdd.min_size(), 1024 * 1024);
        assert_eq!(hdd.max_size(), 16 * 1024 * 1024);

        let ssd = AdaptiveBlockSize::new(DeviceType::SSD);
        assert_eq!(ssd.get_size(), 8 * 1024 * 1024);

        let nvme = AdaptiveBlockSize::new(DeviceType::NVMe);
        assert_eq!(nvme.get_size(), 16 * 1024 * 1024);

        let usb = AdaptiveBlockSize::new(DeviceType::USB);
        assert_eq!(usb.get_size(), 2 * 1024 * 1024);
        assert_eq!(usb.min_size(), 512 * 1024);
        assert_eq!(usb.max_size(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_adjust_decrease_on_high_latency() {
        let mut sizer = AdaptiveBlockSize::new(DeviceType::HDD); // 4MB
        sizer.adjust(60.0, 500.0); // latency > 50ms
        assert_eq!(sizer.get_size(), 3 * 1024 * 1024);

        // Keep decreasing down to min
        sizer.adjust(60.0, 500.0);
        sizer.adjust(60.0, 500.0);
        sizer.adjust(60.0, 500.0); // should hit min_size = 1MB
        assert_eq!(sizer.get_size(), 1024 * 1024);
    }

    #[test]
    fn test_adjust_increase_on_low_latency() {
        let mut sizer = AdaptiveBlockSize::new(DeviceType::HDD); // 4MB
        sizer.adjust(5.0, 500.0); // latency < 10ms, throughput < 800 MB/s
        assert_eq!(sizer.get_size(), 5 * 1024 * 1024);
    }

    #[test]
    fn test_adaptive_chunker_backward_compat() {
        let mut chunker = AdaptiveChunker::new(4 * 1024 * 1024, 1024 * 1024, 16 * 1024 * 1024);
        assert_eq!(chunker.current_chunk_size(), 4 * 1024 * 1024);
        chunker.observe(60, 4 * 1024 * 1024);
        assert_eq!(chunker.current_chunk_size(), 3 * 1024 * 1024);
    }
}
