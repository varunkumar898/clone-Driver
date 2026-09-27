//! Block-level verification engine.

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::storage::buffer::AlignedBuffer;
use crate::storage::checksum::ChecksumState;
use crate::storage::io_ops::{BlockDevice, SequentialReader};

pub struct VerificationEngine;

impl VerificationEngine {
    pub fn verify_devices(
        source_path: &str,
        dest_path: &str,
        total_bytes: u64,
    ) -> Result<VerificationReport, StorageError> {
        let source = BlockDevice::open_read(source_path)?;
        let dest = BlockDevice::open_read(dest_path)?;

        let mut source_reader = SequentialReader::new(source, 0);
        let mut dest_reader = SequentialReader::new(dest, 0);

        let mut source_hash = ChecksumState::new(false);
        let mut dest_hash = ChecksumState::new(false);

        let mut matched = 0u64;
        let mut mismatched = 0u64;

        let buffer_size = 4 * 1024 * 1024;

        while matched + mismatched < total_bytes {
            let remaining = (total_bytes - (matched + mismatched)) as usize;
            let chunk_size = std::cmp::min(buffer_size, remaining);

            let mut src_buf = AlignedBuffer::new(chunk_size)?;
            let mut dst_buf = AlignedBuffer::new(chunk_size)?;

            let src_bytes = source_reader.read_into(&mut src_buf)?;
            let dst_bytes = dest_reader.read_into(&mut dst_buf)?;

            if src_bytes != dst_bytes {
                mismatched += src_bytes as u64;
                break;
            }

            if src_buf.as_slice() != dst_buf.as_slice() {
                mismatched += src_bytes as u64;
            } else {
                matched += src_bytes as u64;
            }

            source_hash.update(src_buf.as_slice());
            dest_hash.update(dst_buf.as_slice());

            if src_bytes == 0 {
                break;
            }
        }

        let (src_xxhash, _) = source_hash.finalize();
        let (dst_xxhash, _) = dest_hash.finalize();

        Ok(VerificationReport {
            matched_bytes: matched,
            mismatched_bytes: mismatched,
            total_bytes,
            source_hash: src_xxhash,
            dest_hash: dst_xxhash,
            verified: matched == total_bytes && src_xxhash == dst_xxhash,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub matched_bytes: u64,
    pub mismatched_bytes: u64,
    pub total_bytes: u64,
    pub source_hash: u64,
    pub dest_hash: u64,
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_verification_pass() {
        let mut src = tempfile::NamedTempFile::new().unwrap();
        src.write_all(b"test data").unwrap();
        src.flush().unwrap();

        let mut dst = tempfile::NamedTempFile::new().unwrap();
        dst.write_all(b"test data").unwrap();
        dst.flush().unwrap();

        let report = VerificationEngine::verify_devices(
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
            9,
        )
        .unwrap();

        assert!(report.verified);
    }
}
