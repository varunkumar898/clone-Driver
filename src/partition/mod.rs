//! Partition table parsing, inspection, and verification module.

pub mod gpt;
pub mod mbr;
pub mod parser;
pub mod table;

pub use parser::PartitionParser;
pub use table::{PartitionEntry, PartitionScheme, PartitionTable};
