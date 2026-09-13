//! Comprehensive, strongly-typed error definitions for DiskClone.

use thiserror::Error;

/// Result alias for DiskClone operations.
pub type Result<T> = std::result::Result<T, CloneError>;

/// Primary application error enum encompassing all subsystem errors.
#[derive(Debug, Error)]
pub enum CloneError {
    #[error("Device error: {0}")]
    Device(#[from] DeviceError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Partition error: {0}")]
    Partition(#[from] PartitionError),

    #[error("Safety validation error: {0}")]
    Safety(#[from] SafetyError),

    #[error("State machine error: {0}")]
    StateMachine(#[from] StateMachineError),

    #[error("Resume error: {0}")]
    Resume(#[from] ResumeError),

    #[error("Privilege error: {0}")]
    Privilege(#[from] PrivilegeError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Errors originating from device discovery, sysfs inspection, and udev events.
#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device not found: {0}")]
    NotFound(String),

    #[error("Failed to query device properties: {0}")]
    PropertyQueryFailed(String),

    #[error("Invalid device node path: {0}")]
    InvalidPath(String),

    #[error("Udev discovery failed: {0}")]
    UdevError(String),

    #[error("Device disconnected unexpectedly: {0}")]
    DeviceDisconnected(String),
}

/// Errors occurring during storage I/O, buffer allocation, and checksumming.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Read failure at offset {offset}: {source}")]
    ReadFailed {
        offset: u64,
        #[source]
        source: std::io::Error,
    },

    #[error("Write failure at offset {offset}: {source}")]
    WriteFailed {
        offset: u64,
        #[source]
        source: std::io::Error,
    },

    #[error("Buffer allocation exhausted in BufferPool")]
    BufferPoolExhausted,

    #[error("Verification failed: block at offset {offset} checksum mismatch")]
    VerificationMismatch { offset: u64 },

    #[error("Flush failure: {0}")]
    FlushFailed(String),

    #[error("Premature end of stream (expected {expected} bytes, got {actual})")]
    UnexpectedEof { expected: usize, actual: usize },
}

/// Errors arising from partition table discovery, validation, and parsing.
#[derive(Debug, Error)]
pub enum PartitionError {
    #[error("Invalid partition table format: {0}")]
    InvalidFormat(String),

    #[error("GPT CRC32 checksum mismatch in header")]
    GptChecksumMismatch,

    #[error("Unsupported partition table type")]
    UnsupportedTableType,

    #[error("Partition overlaps with existing partition")]
    OverlappingPartition,
}

/// Safety rule violations preventing hazardous operations.
#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("Destination device is the host system disk ({0})")]
    DestinationIsSystemDisk(String),

    #[error("Destination device contains mounted partition ({0})")]
    DestinationIsMounted(String),

    #[error("Destination capacity ({dest_bytes} B) is smaller than source ({source_bytes} B)")]
    DestinationTooSmall { source_bytes: u64, dest_bytes: u64 },

    #[error("Source and destination devices are identical: {0}")]
    IdenticalSourceAndDestination(String),

    #[error(
        "Required explicit confirmation token mismatch. Expected: '{expected}', Got: '{provided}'"
    )]
    InvalidConfirmationToken { expected: String, provided: String },

    #[error("Device serial number verification failed during resume. Expected: '{expected}', Found: '{found}'")]
    SerialMismatch { expected: String, found: String },
}

/// Errors encountered in state transition validations.
#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("Illegal state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Required context missing for transition: {0}")]
    MissingContext(String),

    #[error("Operation cancelled by user")]
    Cancelled,
}

/// Errors occurring during resume journal processing and checkpointing.
#[derive(Debug, Error)]
pub enum ResumeError {
    #[error("Resume journal corrupted or invalid: {0}")]
    CorruptedJournal(String),

    #[error("Resume journal not found at path: {0}")]
    JournalNotFound(String),

    #[error("Device state has changed since checkpoint")]
    DeviceStateChanged,
}

/// Errors related to privilege escalation and polkit / daemon communication.
#[derive(Debug, Error)]
pub enum PrivilegeError {
    #[error("Insufficient privileges to execute raw block operation: {0}")]
    AccessDenied(String),

    #[error("Helper daemon not available: {0}")]
    HelperUnavailable(String),

    #[error("Polkit authorization failed: {0}")]
    AuthorizationFailed(String),
}
