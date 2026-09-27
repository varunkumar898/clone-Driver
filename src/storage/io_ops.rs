//! Low-level positional block I/O operations (pread/pwrite) and sequential streamers.

use super::buffer::AlignedBuffer;
use crate::error::StorageError;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::uio::{pread, pwrite};
use std::os::unix::io::AsRawFd;

/// Block device with positional I/O
pub struct BlockDevice {
    file: std::fs::File,
    path: String,
    sector_size: u32,
}

impl BlockDevice {
    /// Open device for reading
    pub fn open_read(path: &str) -> Result<Self, StorageError> {
        let file = std::fs::File::open(path).map_err(StorageError::IoError)?;

        let sector_size = query_sector_size_from_file(&file)
            .or_else(|| query_sector_size(path))
            .unwrap_or(512);

        Ok(BlockDevice {
            file,
            path: path.to_string(),
            sector_size,
        })
    }

    /// Open device for writing (with O_DIRECT for block devices, O_SYNC always)
    pub fn open_write(path: &str) -> Result<Self, StorageError> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(StorageError::IoError)?;

        let fd = file.as_raw_fd();

        // Always apply O_SYNC
        let current_flags = fcntl(fd, FcntlArg::F_GETFL).unwrap_or(0);
        let _ = fcntl(
            fd,
            FcntlArg::F_SETFL(OFlag::from_bits_truncate(current_flags) | OFlag::O_SYNC),
        );

        // Try O_DIRECT only for block devices (CLARIFICATION 1)
        if path.starts_with("/dev/") {
            let current_flags = fcntl(fd, FcntlArg::F_GETFL).unwrap_or(0);
            let _ = fcntl(
                fd,
                FcntlArg::F_SETFL(OFlag::from_bits_truncate(current_flags) | OFlag::O_DIRECT),
            );
            // Ignore failure - not all block devices support O_DIRECT
        }

        let sector_size = query_sector_size_from_file(&file)
            .or_else(|| query_sector_size(path))
            .unwrap_or(512);

        Ok(BlockDevice {
            file,
            path: path.to_string(),
            sector_size,
        })
    }

    /// Read bytes at offset (does not advance file position)
    pub fn pread_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, StorageError> {
        pread(&self.file, buf, offset as i64).map_err(|e| StorageError::IoctlError(e.to_string()))
    }

    /// Write bytes at offset (does not advance file position)
    pub fn pwrite_at(&self, offset: u64, buf: &[u8]) -> Result<usize, StorageError> {
        pwrite(&self.file, buf, offset as i64).map_err(|e| StorageError::IoctlError(e.to_string()))
    }

    /// Flush buffers to persistent storage
    pub fn sync_buffers(&self) -> Result<(), StorageError> {
        nix::unistd::fdatasync(self.file.as_raw_fd())
            .map_err(|e| StorageError::IoctlError(e.to_string()))
    }

    /// Get sector size
    pub fn sector_size(&self) -> u32 {
        self.sector_size
    }

    /// Access underlying file reference
    pub fn file(&self) -> &std::fs::File {
        &self.file
    }

    /// Get device node / file path
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Helper to query block device sector size via file descriptor
fn query_sector_size_from_file(file: &std::fs::File) -> Option<u32> {
    crate::linux::ffi::get_sector_size(file).ok()
}

/// Helper to query block device sector size via path
fn query_sector_size(path: &str) -> Option<u32> {
    if let Ok(file) = std::fs::File::open(path) {
        query_sector_size_from_file(&file)
    } else {
        None
    }
}

/// Sequential reader (maintains current offset)
pub struct SequentialReader {
    pub device: BlockDevice,
    pub current_offset: u64,
}

impl SequentialReader {
    pub fn new(device: BlockDevice, start_offset: u64) -> Self {
        SequentialReader {
            device,
            current_offset: start_offset,
        }
    }

    /// Read into aligned buffer, advance offset
    pub fn read_into(&mut self, buf: &mut AlignedBuffer) -> Result<usize, StorageError> {
        let bytes = self
            .device
            .pread_at(self.current_offset, buf.as_mut_slice())?;
        buf.size = bytes;
        buf.offset = self.current_offset;
        self.current_offset += bytes as u64;
        Ok(bytes)
    }

    /// Current reading byte offset
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Reference to underlying BlockDevice
    pub fn device(&self) -> &BlockDevice {
        &self.device
    }
}

/// Sequential writer (maintains current offset)
pub struct SequentialWriter {
    pub device: BlockDevice,
    pub current_offset: u64,
}

impl SequentialWriter {
    pub fn new(device: BlockDevice, start_offset: u64) -> Self {
        SequentialWriter {
            device,
            current_offset: start_offset,
        }
    }

    /// Write from aligned buffer, advance offset
    pub fn write_from(&mut self, buf: &AlignedBuffer) -> Result<usize, StorageError> {
        let bytes = self.device.pwrite_at(self.current_offset, buf.as_slice())?;
        self.current_offset += bytes as u64;
        Ok(bytes)
    }

    /// Flush buffers to persistent storage
    pub fn sync_buffers(&self) -> Result<(), StorageError> {
        self.device.sync_buffers()
    }

    /// Current writing byte offset
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Reference to underlying BlockDevice
    pub fn device(&self) -> &BlockDevice {
        &self.device
    }
}

/// Positional block I/O abstraction.
pub struct BlockIo;

impl BlockIo {
    /// Reads exact chunk from file/device at specified byte offset.
    pub fn read_at(
        file: &std::fs::File,
        offset: u64,
        buffer: &mut [u8],
    ) -> crate::error::Result<usize> {
        pread(file, buffer, offset as i64)
            .map_err(|e| StorageError::IoctlError(e.to_string()).into())
    }

    /// Writes exact chunk to file/device at specified byte offset.
    pub fn write_at(
        file: &std::fs::File,
        offset: u64,
        buffer: &[u8],
    ) -> crate::error::Result<usize> {
        pwrite(file, buffer, offset as i64)
            .map_err(|e| StorageError::IoctlError(e.to_string()).into())
    }

    /// Flushes kernel write caches and syncs to physical media.
    pub fn sync_device(file: &std::fs::File) -> crate::error::Result<()> {
        nix::unistd::fdatasync(file.as_raw_fd())
            .map_err(|e| StorageError::IoctlError(e.to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_sequential_reader_writer() {
        // Create temp file
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(b"0123456789").unwrap();
        temp.flush().unwrap();

        // Read sequentially
        let device = BlockDevice::open_read(temp.path().to_str().unwrap()).unwrap();
        let mut reader = SequentialReader::new(device, 0);

        let mut buf = AlignedBuffer::new(10).unwrap();
        let bytes = reader.read_into(&mut buf).unwrap();

        assert_eq!(bytes, 10);
        assert_eq!(buf.as_slice(), b"0123456789");
    }

    #[test]
    fn test_sequential_writer_and_sync() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();

        let device = BlockDevice::open_write(path).unwrap();
        let mut writer = SequentialWriter::new(device, 0);

        let mut buf = AlignedBuffer::new(10).unwrap();
        buf.as_mut_slice()[..5].copy_from_slice(b"hello");
        buf.size = 5;

        let bytes = writer.write_from(&buf).unwrap();
        assert_eq!(bytes, 5);
        assert_eq!(writer.current_offset(), 5);

        writer.sync_buffers().unwrap();

        // Read back
        let read_dev = BlockDevice::open_read(path).unwrap();
        let mut read_buf = AlignedBuffer::new(10).unwrap();
        let n = read_dev
            .pread_at(0, &mut read_buf.as_mut_slice()[..5])
            .unwrap();
        read_buf.size = n;
        assert_eq!(n, 5);
        assert_eq!(read_buf.as_slice(), b"hello");
    }

    #[test]
    fn test_block_io_functions() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(b"blockio-test-payload").unwrap();
        temp.flush().unwrap();

        let file = std::fs::File::open(temp.path()).unwrap();
        let mut data = [0u8; 7];
        let n = BlockIo::read_at(&file, 0, &mut data).unwrap();
        assert_eq!(n, 7);
        assert_eq!(&data, b"blockio");

        BlockIo::sync_device(&file).unwrap();
    }
}
