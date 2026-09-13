//! C FFI bindings and ioctl codes for Linux block devices.

use crate::error::Result;
use std::os::unix::io::AsRawFd;

// Linux block ioctl command codes
pub const BLKGETSIZE64: u64 = 0x80081272;
pub const BLKSSZGET: u64 = 0x1268;
pub const BLKFLSBUF: u64 = 0x1261;
pub const BLKRRPART: u64 = 0x125f;

/// Queries the total size of a block device in bytes using BLKGETSIZE64 ioctl.
pub fn get_block_device_size<T: AsRawFd>(_fd: &T) -> Result<u64> {
    todo!("Phase 2: implement BLKGETSIZE64 ioctl wrapper")
}

/// Queries the logical sector size of a block device using BLKSSZGET ioctl.
pub fn get_sector_size<T: AsRawFd>(_fd: &T) -> Result<u32> {
    todo!("Phase 2: implement BLKSSZGET ioctl wrapper")
}

/// Flushes internal device buffers using BLKFLSBUF ioctl.
pub fn flush_buffers<T: AsRawFd>(_fd: &T) -> Result<()> {
    todo!("Phase 2: implement BLKFLSBUF ioctl wrapper")
}

/// Requests the kernel to re-read partition tables using BLKRRPART ioctl.
pub fn reread_partition_table<T: AsRawFd>(_fd: &T) -> Result<()> {
    todo!("Phase 2: implement BLKRRPART ioctl wrapper")
}
