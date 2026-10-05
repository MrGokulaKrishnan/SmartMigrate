//! Shared thread-safe application state managed by Tauri.

use crate::stream::StreamSessionState;
use migroute::identity::DeviceIdentity;
use migroute::pairing::PairingSession;
use migroute::trust::TrustStore;
use std::sync::Mutex;

pub struct AppState {
    pub identity: DeviceIdentity,
    pub trust_store: Mutex<TrustStore>,
    pub active_pairing: Mutex<Option<PairingSession>>,
    pub active_stream: Mutex<StreamSessionState>,
}

impl AppState {
    pub fn new(identity: DeviceIdentity, trust_store: TrustStore) -> Self {
        Self {
            identity,
            trust_store: Mutex::new(trust_store),
            active_pairing: Mutex::new(None),
            active_stream: Mutex::new(StreamSessionState::default()),
        }
    }
}

