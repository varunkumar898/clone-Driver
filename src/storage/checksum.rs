//! Checksumming implementation (xxHash64 and SHA256).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xxhash_rust::xxh64::xxh64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    XxHash64,
    Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockChecksum {
    XxHash64(u64),
    Sha256(String),
}

/// Computes xxHash64 checksum of a byte slice with seed 0.
pub fn compute_xxhash64(data: &[u8]) -> u64 {
    xxh64(data, 0)
}

/// Computes SHA256 hex digest of a byte slice.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Computes checksum according to the specified algorithm.
pub fn compute_checksum(data: &[u8], algorithm: ChecksumAlgorithm) -> BlockChecksum {
    match algorithm {
        ChecksumAlgorithm::XxHash64 => BlockChecksum::XxHash64(compute_xxhash64(data)),
        ChecksumAlgorithm::Sha256 => BlockChecksum::Sha256(compute_sha256(data)),
    }
}
