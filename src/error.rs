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

    #[error("Confirmation error: {0}")]
    Confirmation(#[from] ConfirmationError),

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

    #[error("BufferPool is full")]
    BufferPoolFull,

    #[error("Buffer does not belong to this BufferPool")]
    BufferNotFromPool,

    #[error("Verification failed: block at offset {offset} checksum mismatch")]
    VerificationMismatch { offset: u64 },

    #[error("Flush failure: {0}")]
    FlushFailed(String),

    #[error("Premature end of stream (expected {expected} bytes, got {actual})")]
    UnexpectedEof { expected: usize, actual: usize },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("I/O control error: {0}")]
    IoctlError(String),

    #[error("Could not locate system cache directory")]
    NoCacheDirectory,

    #[error("System time error occurred")]
    TimeError,

    #[error("Journal serialization error: {0}")]
    SerializationError(String),

    #[error("Clone operation cancelled")]
    CloneCancelled,

    #[error("Short write occurred during block cloning")]
    ShortWrite,

    // ── Phase 6: Safety Layer ─────────────────────────────────────────────
    #[error("System disk protection triggered: cannot clone to running OS disk")]
    SystemDiskProtected,

    #[error(
        "Insufficient capacity: source requires {source_bytes} bytes, destination only has {dest_bytes} bytes"
    )]
    InsufficientCapacity { source_bytes: u64, dest_bytes: u64 },

    #[error("Text confirmation mismatch: expected 'CLONE TO THIS DISK'")]
    TextConfirmationFailed,

    #[error("Confirmation state error: {0}")]
    ConfirmationStateError(String),

    #[error("Cannot query block device size")]
    CannotQueryDeviceSize,

    #[error("Invalid device path supplied")]
    InvalidDevicePath,
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
#[derive(Debug, Error, PartialEq)]
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
#[derive(Debug, Error, PartialEq)]
pub enum ResumeError {
    #[error("Resume journal corrupted or invalid: {0}")]
    CorruptedJournal(String),

    #[error("Resume journal not found at path: {0}")]
    JournalNotFound(String),

    #[error("Device state has changed since checkpoint")]
    DeviceStateChanged,

    #[error("Checkpoint expired: {0}")]
    CheckpointExpired(String),

    #[error("Device serial mismatch during resume: {0}")]
    SerialMismatch(String),

    #[error("Device size mismatch during resume: expected {expected} bytes, found {found} bytes")]
    SizeMismatch { expected: u64, found: u64 },

    #[error(
        "Verification hash mismatch on initial block: expected {expected:#x}, found {found:#x}"
    )]
    InitialHashMismatch { expected: u64, found: u64 },

    #[error("Invalid resume offset: {0}")]
    InvalidOffset(String),

    #[error("Device not found during resume: {0}")]
    DeviceNotFound(String),

    #[error(
        "Destination device too small: required {required} bytes, available {available} bytes"
    )]
    DeviceTooSmall { required: u64, available: u64 },
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

/// Errors occurring during confirmation and safety gate validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfirmationError {
    #[error("Destination device is the host system disk")]
    SystemDiskProtected,

    #[error("Insufficient capacity: source requires {source_bytes} bytes, destination only has {dest} bytes")]
    InsufficientCapacity { source_bytes: u64, dest: u64 },

    #[error("Text mismatch: expected '{expected}', got '{got}'")]
    TextMismatch { expected: String, got: String },

    #[error("Invalid state transition from {current:?}")]
    StateInvalid { current: crate::state::CloneState },
}
