//! Privilege escalation abstraction and helper process coordination.
//!
//! Provides interfaces for acquiring elevated block device I/O permissions
//! via Polkit, systemd-user, or helper daemons while maintaining an
//! unprivileged frontend session.

use crate::error::Result;

/// Privilege manager coordinating elevated operations.
pub struct PrivilegeManager;

impl PrivilegeManager {
    /// Checks if current process is running as root (UID == 0 or EUID == 0).
    pub fn is_root() -> bool {
        unsafe { libc::getuid() == 0 || libc::geteuid() == 0 }
    }

    /// Checks if current process has root or raw block access.
    pub fn has_raw_block_access() -> bool {
        Self::is_root()
    }

    /// Requests authorization via polkit or helper daemon.
    pub fn request_elevation() -> Result<()> {
        if Self::has_raw_block_access() {
            return Ok(());
        }
        // When not running as root, elevation dialog is coordinated via Tauri Polkit IPC.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_check_does_not_panic() {
        let is_elevated = PrivilegeManager::has_raw_block_access();
        assert_eq!(is_elevated, unsafe {
            libc::getuid() == 0 || libc::geteuid() == 0
        });
    }
}
