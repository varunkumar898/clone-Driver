//! C FFI bindings and ioctl codes for Linux block devices.

use crate::error::{DeviceError, Result};
use std::os::unix::io::AsRawFd;

// Linux block ioctl command codes
pub const BLKGETSIZE64: u64 = 0x80081272;
pub const BLKSSZGET: u64 = 0x1268;
pub const BLKFLSBUF: u64 = 0x1261;
pub const BLKRRPART: u64 = 0x125f;

/// Queries the total size of a block device in bytes using BLKGETSIZE64 ioctl.
pub fn get_block_device_size<T: AsRawFd>(fd: &T) -> Result<u64> {
    let mut size: u64 = 0;
    // SAFETY: fd is a valid open file descriptor; size is a local stack variable whose
    // address we pass to the kernel.  BLKGETSIZE64 writes exactly 8 bytes into it.
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), BLKGETSIZE64, &mut size as *mut u64) };
    if ret == -1 {
        Err(DeviceError::PropertyQueryFailed(format!(
            "BLKGETSIZE64 ioctl failed: {}",
            std::io::Error::last_os_error()
        ))
        .into())
    } else {
        Ok(size)
    }
}

/// Queries the logical sector size of a block device using BLKSSZGET ioctl.
pub fn get_sector_size<T: AsRawFd>(fd: &T) -> Result<u32> {
    let mut size: libc::c_int = 0;
    // SAFETY: same reasoning as above; BLKSSZGET writes sizeof(int) bytes.
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), BLKSSZGET, &mut size as *mut libc::c_int) };
    if ret == -1 {
        Err(DeviceError::PropertyQueryFailed(format!(
            "BLKSSZGET ioctl failed: {}",
            std::io::Error::last_os_error()
        ))
        .into())
    } else {
        Ok(size as u32)
    }
}

/// Flushes internal device buffers using BLKFLSBUF ioctl.
pub fn flush_buffers<T: AsRawFd>(fd: &T) -> Result<()> {
    // SAFETY: BLKFLSBUF takes no extra argument; the kernel ignores the third parameter.
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), BLKFLSBUF, 0) };
    if ret == -1 {
        Err(DeviceError::PropertyQueryFailed(format!(
            "BLKFLSBUF ioctl failed: {}",
            std::io::Error::last_os_error()
        ))
        .into())
    } else {
        Ok(())
    }
}

/// Requests the kernel to re-read partition tables using BLKRRPART ioctl.
pub fn reread_partition_table<T: AsRawFd>(fd: &T) -> Result<()> {
    // SAFETY: BLKRRPART takes no extra argument.
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), BLKRRPART, 0) };
    if ret == -1 {
        Err(DeviceError::PropertyQueryFailed(format!(
            "BLKRRPART ioctl failed: {}",
            std::io::Error::last_os_error()
        ))
        .into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure-logic smoke test — just verify the constants have expected values.
    #[test]
    fn test_ioctl_constants() {
        assert_eq!(BLKGETSIZE64, 0x80081272);
        assert_eq!(BLKSSZGET, 0x1268);
        assert_eq!(BLKFLSBUF, 0x1261);
        assert_eq!(BLKRRPART, 0x125f);
    }

    /// Tier-3: ioctl on a real block device or loop device.
    #[test]
    fn test_get_block_device_size_real_or_loop() {
        if unsafe { libc::geteuid() } == 0 {
            // Root path: try first real block device reported by sysfs
            if let Ok(entries) = std::fs::read_dir("/sys/block") {
                for entry in entries.flatten() {
                    let dev_path = std::path::PathBuf::from("/dev").join(entry.file_name());
                    if let Ok(f) = std::fs::File::open(&dev_path) {
                        let size = get_block_device_size(&f);
                        assert!(size.is_ok(), "BLKGETSIZE64 failed on {:?}", dev_path);
                        assert!(size.unwrap() > 0);
                        return;
                    }
                }
            }
        } else {
            // Non-root path: try opening /dev/loop0 or /dev/loop1 if setup by CI/user
            for loop_dev in &["/dev/loop0", "/dev/loop1", "/dev/loop-control"] {
                if let Ok(f) = std::fs::File::open(loop_dev) {
                    let res = get_block_device_size(&f);
                    // Don't panic if unconfigured, but if it succeeded assert > 0
                    if let Ok(sz) = res {
                        assert!(sz > 0);
                    }
                    return;
                }
            }
        }
    }
}
