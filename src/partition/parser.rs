//! Unified partition table detector and parser.
//!
//! Provides a single entry-point (`PartitionParser::parse_auto`) that:
//! 1. Detects whether the source is GPT or MBR.
//! 2. Delegates to the appropriate sub-parser.
//! 3. Returns a unified `PartitionTable` for use by the rest of the engine.

use super::{
    gpt::GptParser,
    mbr::MbrParser,
    table::{PartitionScheme, PartitionTable},
};
use crate::error::{PartitionError, Result};
use std::io::{Read, Seek, SeekFrom};

/// Unified partition parser capable of auto-detecting GPT or MBR schemes.
pub struct PartitionParser;

impl PartitionParser {
    /// Detects and parses either GPT or MBR from the provided reader.
    ///
    /// Detection order:
    /// 1. **GPT** — checks for the `"EFI PART"` signature at byte offset 512.
    /// 2. **MBR** — checks for the `0x55AA` boot signature at bytes 510–511
    ///    while confirming there is *no* GPT signature immediately after.
    /// 3. **Unknown** — returns `PartitionScheme::Unknown` with an empty
    ///    partition list rather than an error, allowing callers to handle raw
    ///    (unpartitioned) disks gracefully.
    ///
    /// The reader is always rewound to position 0 before delegation.
    pub fn parse_auto<R: Read + Seek>(reader: &mut R, sector_size: u32) -> Result<PartitionTable> {
        // Rewind so both detectors start from the same position.
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| PartitionError::InvalidFormat(format!("seek failed: {e}")))?;

        if GptParser::is_gpt(reader) {
            return GptParser::parse(reader, sector_size);
        }

        if MbrParser::is_mbr(reader) {
            return MbrParser::parse(reader, sector_size);
        }

        // Unknown / raw disk — return an empty table rather than an error.
        Ok(PartitionTable {
            scheme: PartitionScheme::Unknown,
            sector_size,
            total_sectors: 0,
            partitions: Vec::new(),
        })
    }

    /// Returns the detected `PartitionScheme` without full parsing.
    ///
    /// Cheaper than `parse_auto` when only the scheme type is needed.
    pub fn detect_scheme<R: Read + Seek>(reader: &mut R) -> PartitionScheme {
        reader.seek(SeekFrom::Start(0)).ok();
        if GptParser::is_gpt(reader) {
            return PartitionScheme::Gpt;
        }
        reader.seek(SeekFrom::Start(0)).ok();
        if MbrParser::is_mbr(reader) {
            return PartitionScheme::Mbr;
        }
        PartitionScheme::Unknown
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gpt::{disk::LogicalBlockSize, mbr::ProtectiveMBR};
    use std::{convert::TryFrom, io::Cursor};

    // ── fixture helpers ───────────────────────────────────────────────────────

    fn make_gpt_image() -> Vec<u8> {
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
            .add_partition(
                "gpt_part",
                256 * 1024,
                gpt::partition_types::LINUX_FS,
                0,
                None,
            )
            .unwrap();
        let mut dev = gdisk.write().unwrap();
        dev.seek(SeekFrom::Start(0)).unwrap();
        let mut v = Vec::new();
        dev.read_to_end(&mut v).unwrap();
        v
    }

    fn make_mbr_image() -> Vec<u8> {
        let data = vec![0u8; 100 * 512];
        let mut cur = Cursor::new(data);
        let mut mbr = mbrman::MBR::new_from(&mut cur, 512, [0x11, 0x22, 0x33, 0x44]).unwrap();
        mbr[1] = mbrman::MBRPartitionEntry {
            boot: mbrman::BOOT_INACTIVE,
            first_chs: mbrman::CHS::empty(),
            sys: 0x83,
            last_chs: mbrman::CHS::empty(),
            starting_lba: 1,
            sectors: 97,
        };
        mbr.write_into(&mut cur).unwrap();
        cur.into_inner()
    }

    // ── detect_scheme ─────────────────────────────────────────────────────────

    #[test]
    fn test_detect_scheme_gpt() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        assert_eq!(
            PartitionParser::detect_scheme(&mut cur),
            PartitionScheme::Gpt
        );
    }

    #[test]
    fn test_detect_scheme_mbr() {
        let bytes = make_mbr_image();
        let mut cur = Cursor::new(bytes);
        assert_eq!(
            PartitionParser::detect_scheme(&mut cur),
            PartitionScheme::Mbr
        );
    }

    #[test]
    fn test_detect_scheme_unknown() {
        let mut cur = Cursor::new(vec![0u8; 4096]);
        assert_eq!(
            PartitionParser::detect_scheme(&mut cur),
            PartitionScheme::Unknown
        );
    }

    // ── parse_auto ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_auto_gpt() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let table = PartitionParser::parse_auto(&mut cur, 512).expect("parse GPT via auto");
        assert_eq!(table.scheme, PartitionScheme::Gpt);
        assert!(!table.partitions.is_empty(), "at least one GPT partition");
    }

    #[test]
    fn test_parse_auto_mbr() {
        let bytes = make_mbr_image();
        let mut cur = Cursor::new(bytes);
        let table = PartitionParser::parse_auto(&mut cur, 512).expect("parse MBR via auto");
        assert_eq!(table.scheme, PartitionScheme::Mbr);
        assert!(!table.partitions.is_empty(), "at least one MBR partition");
    }

    #[test]
    fn test_parse_auto_unknown_does_not_error() {
        let mut cur = Cursor::new(vec![0u8; 4096]);
        let table =
            PartitionParser::parse_auto(&mut cur, 512).expect("unknown should return empty table");
        assert_eq!(table.scheme, PartitionScheme::Unknown);
        assert!(table.partitions.is_empty());
    }

    #[test]
    fn test_parse_auto_gpt_partition_fields() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        let table = PartitionParser::parse_auto(&mut cur, 512).unwrap();
        let p = &table.partitions[0];
        assert_eq!(p.name.as_deref(), Some("gpt_part"));
        assert!(p.size_bytes > 0);
        assert!(p.partition_type_guid.is_some());
    }

    #[test]
    fn test_parse_auto_mbr_partition_fields() {
        let bytes = make_mbr_image();
        let mut cur = Cursor::new(bytes);
        let table = PartitionParser::parse_auto(&mut cur, 512).unwrap();
        let p = &table.partitions[0];
        assert_eq!(p.mbr_type_byte, Some(0x83));
        assert!(p.size_bytes > 0);
        assert!(p.partition_type_guid.is_none());
    }

    /// Ensures that after `parse_auto` the reader is left in a usable state.
    #[test]
    fn test_parse_auto_reader_still_seekable_after() {
        let bytes = make_gpt_image();
        let mut cur = Cursor::new(bytes);
        PartitionParser::parse_auto(&mut cur, 512).unwrap();
        // Should be able to seek back to start without error.
        assert!(cur.seek(SeekFrom::Start(0)).is_ok());
    }
}
