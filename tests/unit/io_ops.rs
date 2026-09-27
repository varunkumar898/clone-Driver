use diskclone::storage::buffer::AlignedBuffer;
use diskclone::storage::io_ops::{BlockDevice, BlockIo, SequentialReader, SequentialWriter};
use std::io::Write;

#[test]
fn test_block_io_scaffold() {
    let _ = BlockIo;
}

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
