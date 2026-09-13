//! Multi-layer safety and validation subsystem.

pub mod confirmation;
pub mod device_identity;
pub mod protection;
pub mod validator;

pub use confirmation::{ConfirmationManager, REQUIRED_CONFIRMATION_STRING};
pub use device_identity::DeviceIdentityMatcher;
pub use protection::SystemDiskProtection;
pub use validator::SafetyValidator;
