//! Device identity representation and validation.
//!
//! Every Smart Migrate device has a stable [`DeviceId`], a user-facing name,
//! an identified operating platform, and a cryptographic fingerprint.

use crate::{DeviceId, DeviceIdError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Operating platform category for a connected device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Windows,
    Android,
    Linux,
    MacOS,
    Unknown,
}

impl DevicePlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Android => "android",
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for DevicePlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for DevicePlatform {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "windows" | "win" | "win32" => Self::Windows,
            "android" => Self::Android,
            "linux" => Self::Linux,
            "macos" | "darwin" | "ios" => Self::MacOS,
            _ => Self::Unknown,
        }
    }
}

/// Device operational role in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRole {
    Host,
    Client,
}

/// A validated, persistent device identity descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub id: DeviceId,
    pub name: String,
    pub platform: DevicePlatform,
    pub fingerprint: String,
    pub role: DeviceRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    InvalidDeviceId(DeviceIdError),
    EmptyName,
    NameTooLong,
    InvalidFingerprint,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeviceId(err) => write!(f, "invalid device ID: {err}"),
            Self::EmptyName => f.write_str("device name cannot be empty"),
            Self::NameTooLong => f.write_str("device name exceeds maximum length of 64 characters"),
            Self::InvalidFingerprint => f.write_str("fingerprint must be a non-empty alphanumeric string"),
        }
    }
}

impl std::error::Error for IdentityError {}

impl DeviceIdentity {
    pub const MAX_NAME_LEN: usize = 64;

    pub fn new(
        id: DeviceId,
        name: impl Into<String>,
        platform: DevicePlatform,
        fingerprint: impl Into<String>,
        role: DeviceRole,
    ) -> Result<Self, IdentityError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(IdentityError::EmptyName);
        }
        if name.chars().count() > Self::MAX_NAME_LEN {
            return Err(IdentityError::NameTooLong);
        }

        let fingerprint = fingerprint.into().trim().to_string();
        if fingerprint.is_empty()
            || !fingerprint
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':')
        {
            return Err(IdentityError::InvalidFingerprint);
        }

        Ok(Self {
            id,
            name,
            platform,
            fingerprint,
            role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_id() -> DeviceId {
        DeviceId::try_from("dev-pc-01").unwrap()
    }

    #[test]
    fn valid_identity_creation() {
        let identity = DeviceIdentity::new(
            sample_id(),
            "Gokul's Workstation",
            DevicePlatform::Windows,
            "SM-A1B2-C3D4-E5F6",
            DeviceRole::Host,
        )
        .unwrap();

        assert_eq!(identity.name, "Gokul's Workstation");
        assert_eq!(identity.platform, DevicePlatform::Windows);
        assert_eq!(identity.role, DeviceRole::Host);
    }

    #[test]
    fn reject_empty_name() {
        let err = DeviceIdentity::new(
            sample_id(),
            "   ",
            DevicePlatform::Windows,
            "SM-A1B2",
            DeviceRole::Host,
        )
        .unwrap_err();

        assert_eq!(err, IdentityError::EmptyName);
    }

    #[test]
    fn reject_overlong_name() {
        let long_name = "x".repeat(65);
        let err = DeviceIdentity::new(
            sample_id(),
            long_name,
            DevicePlatform::Windows,
            "SM-A1B2",
            DeviceRole::Host,
        )
        .unwrap_err();

        assert_eq!(err, IdentityError::NameTooLong);
    }

    #[test]
    fn reject_invalid_fingerprint() {
        let err = DeviceIdentity::new(
            sample_id(),
            "Valid Name",
            DevicePlatform::Windows,
            "invalid fingerprint with spaces!",
            DeviceRole::Host,
        )
        .unwrap_err();

        assert_eq!(err, IdentityError::InvalidFingerprint);
    }

    #[test]
    fn platform_parsing() {
        assert_eq!(DevicePlatform::from("windows"), DevicePlatform::Windows);
        assert_eq!(DevicePlatform::from("WIN32"), DevicePlatform::Windows);
        assert_eq!(DevicePlatform::from("android"), DevicePlatform::Android);
        assert_eq!(DevicePlatform::from("Linux"), DevicePlatform::Linux);
        assert_eq!(DevicePlatform::from("darwin"), DevicePlatform::MacOS);
        assert_eq!(DevicePlatform::from("something-else"), DevicePlatform::Unknown);
    }
}
