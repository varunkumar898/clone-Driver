# Performance Tuning Guide

## I/O Engine Optimization

DiskClone is designed to maximize disk I/O bus saturation on SATA SSDs (550 MB/s) and NVMe drives (3,000 - 7,000+ MB/s).

### 1. Chunk Size Configuration
- **Default Chunk Size**: 4MB (`4 * 1024 * 1024` bytes).
- **Max Chunk Size**: 16MB.
- **Adaptive Chunking**:
  - The adaptive controller monitors throughput moving average and I/O latency.
  - If latency is below 15ms and throughput is increasing, chunk size scales up to 16MB.
  - If latency spikes or queue depth increases, chunk size drops to 1MB or 2MB to maintain responsiveness and permit clean cancellation.

### 2. Aligned Buffers
- Buffers are page-aligned (4096-byte alignment) to permit direct I/O (`O_DIRECT` capability) and bypass kernel page cache pollution.

### 3. Checksum Overhead
- xxHash64 consumes under 0.2 CPU cycles per byte, ensuring zero CPU bottleneck even at multi-gigabyte NVMe speeds.
