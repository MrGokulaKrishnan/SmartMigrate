//! Shared thread-safe application state managed by Tauri.

use crate::clipboard::ClipboardState;
use crate::input::InputController;
use crate::resilience::ConnectionWatchdog;
use crate::stream::StreamSessionState;
use crate::transfer::TransferManager;
use migroute::identity::DeviceIdentity;
use migroute::pairing::PairingSession;
use migroute::trust::TrustStore;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub identity: DeviceIdentity,
    pub trust_store: Arc<Mutex<TrustStore>>,
    pub active_pairing: Arc<Mutex<Option<PairingSession>>>,
    pub active_stream: Arc<Mutex<StreamSessionState>>,
    pub input_controller: Arc<InputController>,
    pub watchdog: Arc<ConnectionWatchdog>,
    pub clipboard: Arc<ClipboardState>,
    pub transfer: Arc<TransferManager>,
}

impl AppState {
    pub fn new(identity: DeviceIdentity, trust_store: TrustStore) -> Self {
        Self {
            identity,
            trust_store: Arc::new(Mutex::new(trust_store)),
            active_pairing: Arc::new(Mutex::new(None)),
            active_stream: Arc::new(Mutex::new(StreamSessionState::default())),
            input_controller: Arc::new(InputController::new()),
            watchdog: Arc::new(ConnectionWatchdog::new()),
            clipboard: Arc::new(ClipboardState::new()),
            transfer: Arc::new(TransferManager::new()),
        }
    }
}


