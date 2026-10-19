//! Trusted device store and authorization policy.
//!
//! Maintains the set of paired and approved devices.
//! Implements strict revocation: when a device is revoked, all granted
//! permissions are immediately revoked and any ongoing or future session
//! must be terminated.

use crate::identity::DevicePlatform;
use crate::{DeviceId, SessionPermission};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Representation of a paired, trusted device in the host's trust store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: DeviceId,
    pub name: String,
    pub platform: DevicePlatform,
    pub fingerprint: String,
    pub paired_at_epoch_ms: u64,
    pub last_seen_epoch_ms: u64,
    pub granted_permissions: BTreeSet<SessionPermission>,
    pub is_revoked: bool,
    #[serde(default)]
    pub token_hash: String,
}

impl TrustedDevice {
    pub fn new(
        id: DeviceId,
        name: impl Into<String>,
        platform: DevicePlatform,
        fingerprint: impl Into<String>,
        paired_at_epoch_ms: u64,
        granted_permissions: impl IntoIterator<Item = SessionPermission>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            platform,
            fingerprint: fingerprint.into(),
            paired_at_epoch_ms,
            last_seen_epoch_ms: paired_at_epoch_ms,
            granted_permissions: granted_permissions.into_iter().collect(),
            is_revoked: false,
            token_hash: String::new(),
        }
    }

    /// Attaches the cryptographic token hash to this trusted device.
    pub fn with_token_hash(mut self, token_hash: impl Into<String>) -> Self {
        self.token_hash = token_hash.into();
        self
    }

    /// Check if device is active (not revoked) and holds the specified permission.
    pub fn has_permission(&self, permission: SessionPermission) -> bool {
        !self.is_revoked && self.granted_permissions.contains(&permission)
    }

    /// Update the timestamp of the last seen interaction.
    pub fn touch(&mut self, now_epoch_ms: u64) {
        if now_epoch_ms > self.last_seen_epoch_ms {
            self.last_seen_epoch_ms = now_epoch_ms;
        }
    }
}

/// Errors occurring within the trust store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustStoreError {
    DeviceNotFound,
    DeviceAlreadyExists,
    DeviceAlreadyRevoked,
}

impl fmt::Display for TrustStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => f.write_str("device not found in trust store"),
            Self::DeviceAlreadyExists => f.write_str("device already registered in trust store"),
            Self::DeviceAlreadyRevoked => f.write_str("device is already revoked"),
        }
    }
}

impl std::error::Error for TrustStoreError {}

/// In-memory host trust store.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustStore {
    devices: BTreeMap<DeviceId, TrustedDevice>,
}

impl TrustStore {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    /// Register a newly paired and approved device.
    pub fn add_device(&mut self, device: TrustedDevice) -> Result<(), TrustStoreError> {
        if self.devices.contains_key(&device.id) {
            return Err(TrustStoreError::DeviceAlreadyExists);
        }
        self.devices.insert(device.id.clone(), device);
        Ok(())
    }

    /// Retrieve a reference to a device by its ID.
    pub fn get_device(&self, id: &DeviceId) -> Option<&TrustedDevice> {
        self.devices.get(id)
    }

    /// Retrieve a mutable reference to a device by its ID.
    pub fn get_device_mut(&mut self, id: &DeviceId) -> Option<&mut TrustedDevice> {
        self.devices.get_mut(id)
    }

    /// List all trusted devices (including revoked ones).
    pub fn list_all(&self) -> Vec<&TrustedDevice> {
        self.devices.values().collect()
    }

    /// List all active, non-revoked devices.
    pub fn list_active(&self) -> Vec<&TrustedDevice> {
        self.devices.values().filter(|d| !d.is_revoked).collect()
    }

    /// Count of active, non-revoked devices.
    pub fn active_count(&self) -> usize {
        self.devices.values().filter(|d| !d.is_revoked).count()
    }

    /// Revoke all authorization for a device.
    pub fn revoke_device(&mut self, id: &DeviceId) -> Result<(), TrustStoreError> {
        let device = self.devices.get_mut(id).ok_or(TrustStoreError::DeviceNotFound)?;
        if device.is_revoked {
            return Err(TrustStoreError::DeviceAlreadyRevoked);
        }
        device.is_revoked = true;
        device.granted_permissions.clear();
        Ok(())
    }

    /// Completely remove a device from the trust store.
    pub fn remove_device(&mut self, id: &DeviceId) -> Result<TrustedDevice, TrustStoreError> {
        self.devices.remove(id).ok_or(TrustStoreError::DeviceNotFound)
    }

    /// Check if a device is active and authorized for a specific permission.
    pub fn is_authorized(&self, id: &DeviceId, permission: SessionPermission) -> bool {
        self.devices
            .get(id)
            .map(|d| d.has_permission(permission))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> DeviceId {
        DeviceId::try_from(s).unwrap()
    }

    #[test]
    fn store_lifecycle() {
        let mut store = TrustStore::new();
        assert_eq!(store.active_count(), 0);

        let dev = TrustedDevice::new(
            id("tab-01"),
            "Pixel Tablet",
            DevicePlatform::Android,
            "FP-9988",
            100_000,
            [SessionPermission::ViewScreen, SessionPermission::ControlMouse],
        );

        assert!(store.add_device(dev).is_ok());
        assert_eq!(store.active_count(), 1);

        // Cannot add duplicate
        let dup = TrustedDevice::new(
            id("tab-01"),
            "Pixel Tablet",
            DevicePlatform::Android,
            "FP-9988",
            100_000,
            [SessionPermission::ViewScreen],
        );
        assert_eq!(store.add_device(dup), Err(TrustStoreError::DeviceAlreadyExists));

        // Authorization checks
        assert!(store.is_authorized(&id("tab-01"), SessionPermission::ViewScreen));
        assert!(store.is_authorized(&id("tab-01"), SessionPermission::ControlMouse));
        assert!(!store.is_authorized(&id("tab-01"), SessionPermission::ControlKeyboard));

        // Revocation
        assert!(store.revoke_device(&id("tab-01")).is_ok());
        assert_eq!(store.active_count(), 0);
        assert!(!store.is_authorized(&id("tab-01"), SessionPermission::ViewScreen));

        // Double revoke fails
        assert_eq!(store.revoke_device(&id("tab-01")), Err(TrustStoreError::DeviceAlreadyRevoked));

        // Removal
        assert!(store.remove_device(&id("tab-01")).is_ok());
        assert_eq!(store.remove_device(&id("tab-01")), Err(TrustStoreError::DeviceNotFound));
    }

    #[test]
    fn test_token_hash_and_serde_backward_compat() {
        let dev = TrustedDevice::new(
            id("phone-01"),
            "Pixel 8",
            DevicePlatform::Android,
            "FP-ABCDEF1234567890",
            200_000,
            [SessionPermission::ViewScreen],
        ).with_token_hash("a1b2c3d4e5f6");

        assert_eq!(dev.token_hash, "a1b2c3d4e5f6");

        let serialized = serde_json::to_string(&dev).unwrap();
        let deserialized: TrustedDevice = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.token_hash, "a1b2c3d4e5f6");

        // Old JSON without token_hash deserializes cleanly with empty token_hash
        let legacy_json = r#"{
            "id": "phone-02",
            "name": "Old Phone",
            "platform": "android",
            "fingerprint": "FP-1234",
            "paired_at_epoch_ms": 100,
            "last_seen_epoch_ms": 100,
            "granted_permissions": ["VIEW_SCREEN"],
            "is_revoked": false
        }"#;
        let legacy_dev: TrustedDevice = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(legacy_dev.token_hash, "");
    }
}
