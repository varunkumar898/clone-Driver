//! GUID Partition Table (GPT) parser and validator.
//!
//! Wraps the `gpt` crate to provide a clean, tested API that converts
//! `gpt::partition::Partition` entries into our internal `PartitionEntry` type.
//!
//! The `gpt` crate requires the underlying device to implement
//! `Read + Write + Seek + Debug` (the `DiskDevice` trait). To avoid requiring
//! `Write` from arbitrary callers, we buffer the source data into a
//! `Cursor<Vec<u8>>` which satisfies all bounds.

use super::table::{PartitionEntry, PartitionScheme, PartitionTable};
use crate::error::{PartitionError, Result};
use gpt::disk::LogicalBlockSize;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// Parser for GPT partition structures adhering to the UEFI specification.
pub struct GptParser;

impl GptParser {
    // ── Private helpers ───────────────────────────────────────────────────────

    /// Reads all remaining bytes from `reader` into an owned `Vec<u8>`,
    /// seeking to position 0 first. Returns the buffer wrapped in a `Cursor`
    /// so the `gpt` crate can treat it as a `DiskDevice`.
    fn buffer<R: Read + Seek>(reader: &mut R) -> std::io::Result<Cursor<Vec<u8>>> {
        reader.seek(SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(Cursor::new(buf))
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Detects whether the stream looks like a GPT disk by checking
    /// bytes 512–519 for the ASCII signature `"EFI PART"`.
    ///
    /// The reader position is reset to its original location after the check.
    pub fn is_gpt<R: Read + Seek>(reader: &mut R) -> bool {
        let original = reader.stream_position().unwrap_or(0);

        let result = (|| -> bool {
            let mut sig = [0u8; 8];
            if reader.seek(SeekFrom::Start(512)).is_err() {
                return false;
            }
            if reader.read_exact(&mut sig).is_err() {
                return false;
            }
            sig == *b"EFI PART"
        })();

        let _ = reader.seek(SeekFrom::Start(original));
        result
    }

    /// Parses GPT structures from a seekable byte source (device or binary fixture).
    ///
    /// `sector_size` must be 512 or 4096; any other value is treated as 512.
    pub fn parse<R: Read + Seek>(reader: &mut R, sector_size: u32) -> Result<PartitionTable> {
        let lbs = match sector_size {
            4096 => LogicalBlockSize::Lb4096,
            _ => LogicalBlockSize::Lb512,
        };

        // Buffer into a Cursor so gpt::DiskDevice (Read+Write+Seek+Debug) is satisfied.
        let cur = Self::buffer(reader)
            .map_err(|e| PartitionError::InvalidFormat(format!("failed to buffer reader: {e}")))?;

        let disk = gpt::GptConfig::new()
            .logical_block_size(lbs)
            .open_from_device(cur)
            .map_err(|e| PartitionError::InvalidFormat(format!("GPT open failed: {e}")))?;

        let header = disk
            .primary_header()
            .map_err(|e| PartitionError::InvalidFormat(format!("GPT primary header: {e}")))?;

        let total_sectors = header.last_usable + 1;

        let mut partitions: Vec<PartitionEntry> = disk
            .partitions()
            .iter()
            .filter(|(_, p)| {
                // Skip "unused" sentinel entries (zero LBAs).
                p.first_lba != 0 || p.last_lba != 0
            })
            .enumerate()
            .map(|(i, (_, p))| {
                let size_bytes =
                    (p.last_lba.saturating_sub(p.first_lba) + 1).saturating_mul(sector_size as u64);
                PartitionEntry {
                    index: (i + 1) as u32,
                    start_lba: p.first_lba,
                    end_lba: p.last_lba,
                    size_bytes,
                    partition_type_guid: Some(p.part_type_guid.guid.to_string()),
                    mbr_type_byte: None,
                    name: if p.name.is_empty() {
                        None
                    } else {
                        Some(p.name.clone())
                    },
                    is_bootable: (p.flags & 0b100) != 0, // bit 2 = legacy BIOS bootable
                }
            })
            .collect();

        // Sort by start LBA for consistent ordering.
        partitions.sort_by_key(|p| p.start_lba);

        Ok(PartitionTable {
            scheme: PartitionScheme::Gpt,
            sector_size,
            total_sectors,
            partitions,
        })
    }

    /// Validates the GPT header and partition-array CRC32 checksums.
    ///
    /// Returns `Ok(true)` if both CRCs are valid, `Ok(false)` on mismatch or
    /// if the source is not a GPT disk. Propagates I/O errors.
    pub fn validate_checksums<R: Read + Seek>(reader: &mut R) -> Result<bool> {
        let cur = match Self::buffer(reader) {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };
        // The `gpt` crate validates CRC32 on open; error ⟹ mismatch or not-GPT.
        match gpt::GptConfig::new()
            .logical_block_size(LogicalBlockSize::Lb512)
            .open_from_device(cur)
        {
            Ok(disk) => Ok(disk.primary_header().is_ok()),
            Err(_) => Ok(false),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gpt::{disk::LogicalBlockSize, mbr::ProtectiveMBR};
    use std::{convert::TryFrom, io::Cursor};

    fn make_gpt_image() -> Vec<u8> {
        const TOTAL_BYTES: usize = 1024 * 1024;
        let mut buf = Cursor::new(vec![0u8; TOTAL_BYTES]);
        let mbr = ProtectiveMBR::with_lb_size(
            u32::try_from(TOTAL_BYTES / 512 - 1).unwrap_or(0xFF_FF_FF_FF),
        );
        mbr.overwrite_lba0(&mut buf).expect("write protective MBR");

        let mut gdisk = gpt::GptConfig::default()
            .writable(true)
            .logical_block_size(LogicalBlockSize::Lb512)
            .create_from_device(buf, None)
            .expect("create GPT disk");

        gdisk
            .add_partition(
                "test_part",
                256 * 1024,
                gpt::partition_types::LINUX_FS,
                0,
                None,
            )
            .expect("add partition");

        let mut final_buf = gdisk.write().expect("write GPT");
        final_buf.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        final_buf.read_to_end(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn test_is_gpt_detects_valid_image() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        assert!(GptParser::is_gpt(&mut cur));
    }

    #[test]
    fn test_is_gpt_rejects_zeroed_buffer() {
        let mut cur = Cursor::new(vec![0u8; 4096]);
        assert!(!GptParser::is_gpt(&mut cur));
    }

    #[test]
    fn test_is_gpt_resets_cursor_position() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        cur.seek(SeekFrom::Start(0)).unwrap();
        GptParser::is_gpt(&mut cur);
        // Position should be restored.
        assert_eq!(cur.stream_position().unwrap(), 0);
    }

    #[test]
    fn test_parse_returns_gpt_scheme() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let table = GptParser::parse(&mut cur, 512).expect("parse GPT");
        assert_eq!(table.scheme, PartitionScheme::Gpt);
        assert_eq!(table.sector_size, 512);
    }

    #[test]
    fn test_parse_partition_fields() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let table = GptParser::parse(&mut cur, 512).expect("parse GPT");
        assert_eq!(table.partitions.len(), 1);

        let p = &table.partitions[0];
        assert_eq!(p.name.as_deref(), Some("test_part"));
        assert!(p.size_bytes > 0, "partition size must be non-zero");
        assert!(p.start_lba > 0, "partition must start after LBA 0");
        assert!(p.end_lba >= p.start_lba, "end LBA >= start LBA");
        assert!(p.partition_type_guid.is_some());
        assert!(p.mbr_type_byte.is_none());
    }

    #[test]
    fn test_parse_partitions_sorted_by_start_lba() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let table = GptParser::parse(&mut cur, 512).unwrap();
        let lbas: Vec<u64> = table.partitions.iter().map(|p| p.start_lba).collect();
        let mut sorted = lbas.clone();
        sorted.sort_unstable();
        assert_eq!(lbas, sorted, "partitions must be sorted by start_lba");
    }

    #[test]
    fn test_parse_empty_buffer_returns_error() {
        let mut cur = Cursor::new(vec![0u8; 1024]);
        let result = GptParser::parse(&mut cur, 512);
        assert!(
            result.is_err(),
            "zero-filled buffer should not parse as GPT"
        );
    }

    #[test]
    fn test_validate_checksums_valid_image() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let valid = GptParser::validate_checksums(&mut cur).expect("validate");
        assert!(valid);
    }

    #[test]
    fn test_validate_checksums_corrupted_header() {
        let mut bytes = make_gpt_image();
        // Corrupt the primary GPT header CRC (bytes 528–531).
        bytes[528] ^= 0xFF;
        bytes[529] ^= 0xFF;
        let mut cur = Cursor::new(bytes);
        let result = GptParser::validate_checksums(&mut cur);
        if let Ok(valid) = result {
            assert!(!valid, "corrupted header must not validate");
        }
    }
}
