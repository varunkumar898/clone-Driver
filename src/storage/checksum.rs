//! Checksumming implementation (xxHash64 and SHA256).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xxhash_rust::xxh64::{xxh64, Xxh64};

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

/// Running checksum state (xxHash64 + optional SHA256)
#[derive(Clone)]
pub struct ChecksumState {
    xxhash: Xxh64,
    sha256: Option<Sha256>,
}

impl ChecksumState {
    /// Create new checksum state
    pub fn new(use_sha256: bool) -> Self {
        ChecksumState {
            xxhash: Xxh64::new(0), // Seed 0
            sha256: if use_sha256 {
                Some(Sha256::new())
            } else {
                None
            },
        }
    }

    /// Update with new data
    pub fn update(&mut self, data: &[u8]) {
        self.xxhash.update(data);
        if let Some(ref mut sha) = self.sha256 {
            sha.update(data);
        }
    }

    /// Finalize and return checksums
    pub fn finalize(self) -> (u64, Option<String>) {
        let xxhash = self.xxhash.digest();
        let sha256 = self.sha256.map(|sha| format!("{:x}", sha.finalize()));
        (xxhash, sha256)
    }
}

/// Checksum of a block (for journal storage)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecksumBlock {
    pub offset: u64,
    pub size: usize,
    pub xxhash: u64,
    pub sha256: Option<String>,
}

impl ChecksumBlock {
    /// Compute checksum of data block
    pub fn compute(data: &[u8], offset: u64, use_sha256: bool) -> Self {
        let mut state = ChecksumState::new(use_sha256);
        state.update(data);
        let (xxhash, sha256) = state.finalize();

        ChecksumBlock {
            offset,
            size: data.len(),
            xxhash,
            sha256,
        }
    }

    /// Verify this checksum against data
    pub fn verify(&self, data: &[u8]) -> bool {
        if data.len() != self.size {
            return false;
        }

        let block = ChecksumBlock::compute(data, self.offset, self.sha256.is_some());
        block.xxhash == self.xxhash && block.sha256 == self.sha256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_state_xxhash() {
        let mut state = ChecksumState::new(false);
        state.update(b"test");
        let (hash, sha) = state.finalize();
        assert!(hash > 0);
        assert!(sha.is_none());
    }

    #[test]
    fn test_checksum_state_xxhash_only() {
        let mut state = ChecksumState::new(false);
        state.update(b"diskclone");
        state.update(b"-test-data");
        let (xxhash, sha256) = state.finalize();

        assert_eq!(xxhash, compute_xxhash64(b"diskclone-test-data"));
        assert_eq!(sha256, None);
    }

    #[test]
    fn test_checksum_state_with_sha256() {
        let mut state = ChecksumState::new(true);
        state.update(b"test");
        let (hash, sha) = state.finalize();
        assert!(hash > 0);
        assert!(sha.is_some());
        assert_eq!(sha.unwrap().len(), 64); // 64 hex chars = 32 bytes
    }

    #[test]
    fn test_checksum_block_compute() {
        let data = b"test data";
        let block = ChecksumBlock::compute(data, 0, false);
        assert_eq!(block.offset, 0);
        assert_eq!(block.size, 9);
        assert!(block.xxhash > 0);
        assert!(block.sha256.is_none());
    }

    #[test]
    fn test_checksum_block_verify() {
        let data = b"test data";
        let block = ChecksumBlock::compute(data, 0, false);
        assert!(block.verify(data));
        assert!(!block.verify(b"wrong data"));
    }

    #[test]
    fn test_xxhash64_consistency() {
        let data = b"diskclone-test-data-stream";
        let h1 = compute_xxhash64(data);
        let h2 = compute_xxhash64(data);
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn test_sha256_known_vector() {
        let empty = b"";
        let hash = compute_sha256(empty);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_checksum_block_compute_and_verify() {
        let data = b"hello world 4096 block data";
        let block = ChecksumBlock::compute(data, 1024, true);

        assert_eq!(block.offset, 1024);
        assert_eq!(block.size, data.len());
        assert_eq!(block.xxhash, compute_xxhash64(data));
        assert_eq!(block.sha256, Some(compute_sha256(data)));

        // Verification passes for matching data
        assert!(block.verify(data));

        // Verification fails for wrong size
        assert!(!block.verify(b"short"));

        // Verification fails for modified data
        let mut corrupted = data.to_vec();
        corrupted[0] ^= 0xFF;
        assert!(!block.verify(&corrupted));
    }
}
