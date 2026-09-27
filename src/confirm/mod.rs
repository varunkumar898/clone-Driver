//! Confirmation flow module.

pub mod confirmation;

pub use confirmation::{
    ConfirmationManager, ConfirmationRequest, ConfirmationResponse, REQUIRED_CONFIRMATION_STRING,
};
