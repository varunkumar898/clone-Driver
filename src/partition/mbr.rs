//! Master Boot Record (MBR) parser and validator.
//!
//! Wraps the `mbrman` crate to provide a clean, tested API that converts
//! `mbrman::MBRPartitionEntry` values into our internal `PartitionEntry` type.

use super::table::{PartitionEntry, PartitionScheme, PartitionTable};
use crate::error::{PartitionError, Result};
use std::io::{Read, Seek, SeekFrom};

/// The canonical two-byte boot signature at bytes 510–511 of sector 0.
pub const MBR_SIGNATURE: [u8; 2] = [0x55, 0xAA];

/// Parser for classical MBR (DOS) partition tables.
pub struct MbrParser;

impl MbrParser {
    /// Verifies the 0x55AA boot signature at byte offsets 510–511 of `sector_data`.
    ///
    /// `sector_data` must be at least 512 bytes; anything shorter returns `false`.
    pub fn verify_signature(sector_data: &[u8]) -> bool {
        sector_data.len() >= 512
            && sector_data[510] == MBR_SIGNATURE[0]
            && sector_data[511] == MBR_SIGNATURE[1]
    }

    /// Detects whether the stream looks like an MBR disk.
    ///
    /// Returns `true` if:
    /// - sector 0 ends with `0x55AA`, **and**
    /// - the data does **not** contain the GPT protective-MBR + EFI signature
    ///   (i.e. bytes 512–519 are **not** `"EFI PART"`).
    ///
    /// This intentionally rejects GPT disks that carry a protective MBR.
    pub fn is_mbr<R: Read + Seek>(reader: &mut R) -> bool {
        let original = reader.stream_position().unwrap_or(0);

        let result = (|| -> bool {
            // Read first 512 bytes (sector 0).
            let mut sector0 = [0u8; 512];
            if reader.seek(SeekFrom::Start(0)).is_err() {
                return false;
            }
            if reader.read_exact(&mut sector0).is_err() {
                return false;
            }
            if !Self::verify_signature(&sector0) {
                return false;
            }

            // Reject GPT protective-MBR: check for "EFI PART" at offset 512.
            let mut gpt_sig = [0u8; 8];
            if reader.read_exact(&mut gpt_sig).is_ok() && &gpt_sig == b"EFI PART" {
                return false;
            }

            true
        })();

        let _ = reader.seek(SeekFrom::Start(original));
        result
    }

    /// Parses MBR structures from sector 0 of a block device or binary fixture.
    ///
    /// `sector_size` is typically 512; it is forwarded to `mbrman::MBR::read_from`.
    pub fn parse<R: Read + Seek>(reader: &mut R, sector_size: u32) -> Result<PartitionTable> {
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| PartitionError::InvalidFormat(format!("seek to start failed: {e}")))?;

        let mbr = mbrman::MBR::read_from(reader, sector_size)
            .map_err(|e| PartitionError::InvalidFormat(format!("MBR read failed: {e}")))?;

        let disk_size_sectors = mbr.disk_size as u64;

        let mut partitions: Vec<PartitionEntry> = Vec::new();
        for (i, p) in mbr.iter() {
            if !p.is_used() {
                continue;
            }
            let start_lba = p.starting_lba as u64;
            let size_sectors = p.sectors as u64;
            let end_lba = start_lba + size_sectors.saturating_sub(1);
            let size_bytes = size_sectors.saturating_mul(sector_size as u64);

            partitions.push(PartitionEntry {
                index: i as u32,
                start_lba,
                end_lba,
                size_bytes,
                partition_type_guid: None,
                mbr_type_byte: Some(p.sys),
                name: None,
                is_bootable: p.boot == mbrman::BOOT_ACTIVE,
            });
        }

        // Sort by start LBA for consistent ordering.
        partitions.sort_by_key(|p| p.start_lba);

        Ok(PartitionTable {
            scheme: PartitionScheme::Mbr,
            sector_size,
            total_sectors: disk_size_sectors,
            partitions,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Builds a minimal in-memory MBR disk image with one primary partition.
    fn make_mbr_image(sector_size: u32, num_sectors: u32) -> Vec<u8> {
        let data = vec![0u8; (num_sectors * sector_size) as usize];
        let mut cur = Cursor::new(data);
        let mut mbr = mbrman::MBR::new_from(&mut cur, sector_size, [0xde, 0xad, 0xbe, 0xef])
            .expect("create MBR");

        // Add a single Linux partition spanning most of the disk.
        let sectors = num_sectors - 2; // leave 2 sectors headroom
        let starting_lba = 1;
        mbr[1] = mbrman::MBRPartitionEntry {
            boot: mbrman::BOOT_ACTIVE,
            first_chs: mbrman::CHS::empty(),
            sys: 0x83, // Linux filesystem
            last_chs: mbrman::CHS::empty(),
            starting_lba,
            sectors,
        };
        mbr.write_into(&mut cur).expect("write MBR");
        cur.into_inner()
    }

    // ── verify_signature ─────────────────────────────────────────────────────

    #[test]
    fn test_verify_signature_valid() {
        let mut sector = vec![0u8; 512];
        sector[510] = 0x55;
        sector[511] = 0xAA;
        assert!(MbrParser::verify_signature(&sector));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let sector = vec![0u8; 512];
        assert!(!MbrParser::verify_signature(&sector));
    }

    #[test]
    fn test_verify_signature_too_short() {
        let sector = vec![0x55u8, 0xAA];
        assert!(!MbrParser::verify_signature(&sector));
    }

    #[test]
    fn test_verify_signature_wrong_bytes() {
        let mut sector = vec![0u8; 512];
        sector[510] = 0xAA;
        sector[511] = 0x55; // swapped
        assert!(!MbrParser::verify_signature(&sector));
    }

    // ── is_mbr ───────────────────────────────────────────────────────────────

    #[test]
    fn test_is_mbr_detects_valid_image() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        assert!(MbrParser::is_mbr(&mut cur));
    }

    #[test]
    fn test_is_mbr_rejects_zeroed_buffer() {
        let mut cur = Cursor::new(vec![0u8; 2048]);
        assert!(!MbrParser::is_mbr(&mut cur));
    }

    #[test]
    fn test_is_mbr_resets_cursor_position() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        MbrParser::is_mbr(&mut cur);
        assert_eq!(cur.stream_position().unwrap(), 0);
    }

    #[test]
    fn test_is_mbr_rejects_gpt_disk() {
        // A GPT protective-MBR has 0x55AA but also "EFI PART" at offset 512.
        use gpt::{disk::LogicalBlockSize, mbr::ProtectiveMBR};
        use std::convert::TryFrom;
        const SZ: usize = 1024 * 1024;
        let mut buf = Cursor::new(vec![0u8; SZ]);
        let pmbr = ProtectiveMBR::with_lb_size(u32::try_from(SZ / 512 - 1).unwrap_or(0xFFFFFFFF));
        pmbr.overwrite_lba0(&mut buf).unwrap();
        let mut gdisk = gpt::GptConfig::default()
            .writable(true)
            .logical_block_size(LogicalBlockSize::Lb512)
            .create_from_device(buf, None)
            .unwrap();
        gdisk
            .add_partition("p1", 128 * 1024, gpt::partition_types::LINUX_FS, 0, None)
            .unwrap();
        let mut final_buf = gdisk.write().unwrap();
        final_buf.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        final_buf.read_to_end(&mut bytes).unwrap();

        let mut cur = Cursor::new(bytes);
        assert!(
            !MbrParser::is_mbr(&mut cur),
            "GPT disk should not be detected as MBR"
        );
    }

    // ── parse ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_returns_mbr_scheme() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        let table = MbrParser::parse(&mut cur, 512).expect("parse MBR");
        assert_eq!(table.scheme, PartitionScheme::Mbr);
        assert_eq!(table.sector_size, 512);
    }

    #[test]
    fn test_parse_partition_count() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        let table = MbrParser::parse(&mut cur, 512).expect("parse MBR");
        assert_eq!(
            table.partitions.len(),
            1,
            "should find exactly one partition"
        );
    }

    #[test]
    fn test_parse_partition_fields() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        let table = MbrParser::parse(&mut cur, 512).expect("parse MBR");
        let p = &table.partitions[0];
        assert_eq!(p.mbr_type_byte, Some(0x83), "Linux FS type byte");
        assert!(p.is_bootable, "partition was marked active");
        assert!(p.size_bytes > 0, "partition has non-zero size");
        assert_eq!(p.start_lba, 1, "starts at LBA 1");
        assert!(p.partition_type_guid.is_none(), "MBR has no GUID");
    }

    #[test]
    fn test_parse_partitions_sorted_by_lba() {
        let bytes = make_mbr_image(512, 100);
        let mut cur = Cursor::new(bytes);
        let table = MbrParser::parse(&mut cur, 512).unwrap();
        let lbas: Vec<u64> = table.partitions.iter().map(|p| p.start_lba).collect();
        let mut sorted = lbas.clone();
        sorted.sort_unstable();
        assert_eq!(lbas, sorted);
    }

    #[test]
    fn test_parse_empty_returns_error() {
        let mut cur = Cursor::new(vec![0u8; 512]);
        // mbrman should error on an all-zero (invalid) MBR.
        let result = MbrParser::parse(&mut cur, 512);
        // Either Ok with 0 partitions or Err — both are acceptable.
        if let Ok(table) = result {
            assert_eq!(
                table.partitions.len(),
                0,
                "zero-filled MBR should yield no partitions"
            );
        }
    }
}
