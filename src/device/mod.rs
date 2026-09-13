//! Device discovery, inspection, and abstraction module.

#[allow(clippy::module_inception)]
pub mod device;
pub mod manager;
pub mod properties;
pub mod system_check;

pub use device::BlockDevice;
pub use manager::DeviceManager;
pub use properties::DeviceProperties;
pub use system_check::SystemDiskChecker;
