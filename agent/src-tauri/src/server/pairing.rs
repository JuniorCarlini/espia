//! The human side of pairing (protocol §4.2): once a connecting device's
//! `pair_request{code}` has been read off the wire, this asks the settings
//! UI to show that device's info and collect what the person typed off the
//! device's own screen — never the code itself, since seeing it is exactly
//! what proves they're standing in front of the device.

use std::collections::HashMap;
use std::sync::Mutex;

use espia_core::pairing::DevicePairing;
use serde::Serialize;
use tauri::{Emitter, State};
use tokio::sync::oneshot;

use super::SharedState;

pub type PendingPairings = Mutex<HashMap<String, oneshot::Sender<Option<String>>>>;

const PAIR_REQUEST_EVENT: &str = "device-pair-request";

#[derive(Clone, Serialize)]
struct PairRequestPayload {
    request_id: String,
    device_id: String,
    board: String,
    firmware: String,
}

/// Registers a pending confirmation and asks the frontend to show it.
/// Returns a receiver that resolves once [`respond_to_pairing`] is called
/// for this same request — `None` if the person cancelled.
pub fn request_confirmation(state: &SharedState, device_id: &str, board: &str, firmware: &str) -> oneshot::Receiver<Option<String>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();
    state
        .pending_pairings
        .lock()
        .expect("pending pairings lock poisoned")
        .insert(request_id.clone(), tx);

    let payload = PairRequestPayload {
        request_id,
        device_id: device_id.to_string(),
        board: board.to_string(),
        firmware: firmware.to_string(),
    };
    if let Err(error) = state.app_handle.emit(PAIR_REQUEST_EVENT, payload) {
        eprintln!("espia: could not notify the UI of a pairing request: {error}");
    }

    rx
}

/// Tauri command: the settings UI calls this once the person has typed (or
/// cancelled) the code shown on the device's screen. The actual code
/// comparison happens on the WebSocket connection's own task, which is the
/// only place that ever saw the device's real code.
#[tauri::command]
pub fn respond_to_pairing(state: State<std::sync::Arc<SharedState>>, request_id: String, typed_code: Option<String>) -> Result<(), String> {
    let mut pending = state.pending_pairings.lock().map_err(|_| "pending pairings lock poisoned".to_string())?;
    let sender = pending
        .remove(&request_id)
        .ok_or_else(|| "no such pairing request — it may have already expired".to_string())?;
    // The receiver may already be gone (the 2-minute deadline elapsed just
    // before this call landed) — that's not an error the UI needs to see.
    let _ = sender.send(typed_code);
    Ok(())
}

/// Every device this agent has paired with.
#[tauri::command]
pub fn list_paired_devices() -> Vec<DevicePairing> {
    espia_core::pairing::load().devices
}

/// Forgets a device's pairing — it will need to pair again to reconnect.
#[tauri::command]
pub fn unpair_device(device_id: String) -> Result<(), String> {
    espia_core::pairing::remove(&device_id)?;
    Ok(())
}
