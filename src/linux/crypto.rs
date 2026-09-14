//! LUKS encryption header detection.

use crate::error::Result;
use std::io::Read;
use std::path::Path;

/// Magic bytes at the start of a LUKS1 or LUKS2 header: `LUKS` + `0xBA 0xBE`.
const LUKS_MAGIC: &[u8; 6] = b"LUKS\xBA\xBE";

pub struct LuksDetector;

impl LuksDetector {
    /// Detects whether a block device contains a LUKS1 or LUKS2 header by
    /// inspecting the first 6 bytes.
    ///
    /// Returns `Ok(false)` — rather than an error — when the device cannot be
    /// opened (permission denied, not a block device, etc.) so that callers
    /// can treat unreadable devices as "not LUKS" without aborting a scan.
    pub fn is_luks_encrypted(device_path: &Path) -> Result<bool> {
        let mut file = match std::fs::File::open(device_path) {
            Ok(f) => f,
            // Gracefully handle permission errors or missing nodes
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
                ) =>
            {
                return Ok(false)
            }
            Err(e) => return Err(e.into()),
        };

        let mut magic = [0u8; 6];
        match file.read_exact(&mut magic) {
            Ok(_) => Ok(&magic == LUKS_MAGIC),
            // File shorter than 6 bytes → not LUKS
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_luks_magic_detection_positive() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"LUKS\xBA\xBErest_of_header").unwrap();
        assert!(LuksDetector::is_luks_encrypted(f.path()).unwrap());
    }

    #[test]
    fn test_luks_magic_detection_negative() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"notLUKS_header_data").unwrap();
        assert!(!LuksDetector::is_luks_encrypted(f.path()).unwrap());
    }

    #[test]
    fn test_luks_too_short_returns_false() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"LU").unwrap(); // only 2 bytes
        assert!(!LuksDetector::is_luks_encrypted(f.path()).unwrap());
    }

    #[test]
    fn test_luks_missing_file_returns_false() {
        let result = LuksDetector::is_luks_encrypted(Path::new("/nonexistent/device/xyz"));
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
