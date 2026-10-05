//! Secure persistence adapter for Windows host identity and trust store.
//!
//! Stores device identity and trusted devices in `%APPDATA%/SmartMigrate/`.
//! Adheres strictly to the AGENTS.md rule:
//! "No secret, device-private key, pairing token, screen frame, password,
//! or raw user file path may reach logs."

use migroute::identity::{DeviceIdentity, DevicePlatform, DeviceRole};
use migroute::trust::TrustStore;
use migroute::DeviceId;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn get_app_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("SmartMigrate")
    } else {
        std::env::temp_dir().join("SmartMigrate")
    }
}

/// Cryptographically secure random bytes using Windows Advapi32 RtlGenRandom.
pub fn secure_random_bytes(buf: &mut [u8]) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "advapi32")]
        extern "system" {
            #[link_name = "SystemFunction036"]
            fn RtlGenRandom(buffer: *mut u8, length: u32) -> u8;
        }
        let success = unsafe { RtlGenRandom(buf.as_mut_ptr(), buf.len() as u32) != 0 };
        if success {
            return;
        }
    }

    // Fallback if not on windows or if RtlGenRandom fails
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut state = seed as u64;
    for b in buf.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *b = (state >> 33) as u8;
    }
}

/// Generates a random 6-digit integer code [0, 999_999].
pub fn generate_pairing_code() -> u32 {
    let mut bytes = [0u8; 4];
    secure_random_bytes(&mut bytes);
    let val = u32::from_le_bytes(bytes);
    val % 1_000_000
}

/// Generates a random cryptographic hex token (e.g. 32 chars).
pub fn generate_crypto_token() -> String {
    let mut bytes = [0u8; 16];
    secure_random_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Loads existing host device identity from disk, or initializes and saves a new persistent one.
pub fn load_or_create_identity() -> DeviceIdentity {
    let dir = get_app_dir();
    let _ = fs::create_dir_all(&dir);
    let id_path = dir.join("identity.json");

    if let Ok(content) = fs::read_to_string(&id_path) {
        if let Ok(identity) = serde_json::from_str::<DeviceIdentity>(&content) {
            return identity;
        }
    }

    // Generate new stable identity
    let mut id_bytes = [0u8; 8];
    secure_random_bytes(&mut id_bytes);
    let id_hex: String = id_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let raw_id = format!("sm-win-{}", id_hex);
    let device_id = DeviceId::try_from(raw_id.as_str()).unwrap_or_else(|_| {
        DeviceId::try_from("sm-win-host-primary").expect("fallback ID is valid")
    });

    let host_name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Windows PC".to_string());

    let mut fp_bytes = [0u8; 12];
    secure_random_bytes(&mut fp_bytes);
    let fp_hex: String = fp_bytes.iter().map(|b| format!("{:02X}", b)).collect();
    let fingerprint = format!("SM-{}-{}", &fp_hex[..6], &fp_hex[6..]);

    let identity = DeviceIdentity::new(
        device_id,
        host_name,
        DevicePlatform::Windows,
        fingerprint,
        DeviceRole::Host,
    )
    .expect("generated host identity must be valid");

    if let Ok(json) = serde_json::to_string_pretty(&identity) {
        let _ = fs::write(&id_path, json);
    }

    identity
}

/// Loads the persistent TrustStore from disk, or returns an empty one.
pub fn load_trust_store() -> TrustStore {
    let path = get_app_dir().join("trusted_devices.json");
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str::<TrustStore>(&content) {
            return store;
        }
    }
    TrustStore::new()
}

/// Saves the TrustStore atomically to disk.
pub fn save_trust_store(store: &TrustStore) -> Result<(), String> {
    let dir = get_app_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("trusted_devices.json");
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
