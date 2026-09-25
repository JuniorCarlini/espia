//! The WebSocket server devices connect to (`protocol/README.md` §2-§5),
//! including pairing (§4.2): an unrecognized or missing `hello` token gets
//! `pair_required` instead of `welcome`, and the connection only continues
//! once the device's own `pair_request{code}` is confirmed by a human in
//! the settings UI (see `pairing.rs`).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use espia_core::pairing::{self, DevicePairing};
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::pairing as pairing_ui;
use super::protocol::{ErrorMessage, Hello, MetricsMessage, PairRequest, Paired, PairRequired, Welcome};
use super::SharedState;

const WS_PATH: &str = "/v1/ws";
// Protocol requires a ping at least every 10s; a couple of seconds of
// margin avoids racing the device's own 30s dead-connection timeout.
const PING_INTERVAL: Duration = Duration::from_secs(8);
// Protocol §4.2: "the pairing code MUST expire after 2 minutes."
const PAIRING_TIMEOUT: Duration = Duration::from_secs(120);

pub async fn serve(bind_addr: &str, state: Arc<SharedState>) {
    let listener = match TcpListener::bind(bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("espia: could not bind WebSocket server on {bind_addr}: {error}");
            return;
        }
    };
    eprintln!("espia: WebSocket server listening on {bind_addr}{WS_PATH}");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                eprintln!("espia: failed to accept a connection: {error}");
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, peer, state).await {
                eprintln!("espia: connection from {peer} ended: {error}");
            }
        });
    }
}

// The callback's `Err` variant type (`ErrorResponse`, a full HTTP response)
// is tokio-tungstenite's own signature, not something this function controls.
#[allow(clippy::result_large_err)]
async fn handle_connection(stream: TcpStream, peer: SocketAddr, state: Arc<SharedState>) -> Result<(), String> {
    let mut path_ok = false;
    let ws_stream = tokio_tungstenite::accept_hdr_async(stream, |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
        path_ok = request.uri().path() == WS_PATH;
        Ok(response)
    })
    .await
    .map_err(|e| format!("handshake failed: {e}"))?;

    if !path_ok {
        return Err("rejected: wrong WebSocket path".to_string());
    }

    let (mut write, mut read) = ws_stream.split();

    let hello_text = match read.next().await {
        Some(Ok(Message::Text(text))) => text,
        Some(Ok(_)) => return Err("first message was not text".to_string()),
        Some(Err(error)) => return Err(format!("error reading hello: {error}")),
        None => return Err("connection closed before hello".to_string()),
    };
    let hello: Hello = serde_json::from_str(&hello_text).map_err(|e| format!("bad hello: {e}"))?;
    eprintln!("espia: device {} ({}) said hello from {peer}", hello.device_id, hello.board);

    let already_paired = hello.token.as_deref().and_then(pairing::find_by_token);

    if let Some(existing) = already_paired {
        eprintln!("espia: device {} presented a known token, skipping pairing", existing.device_id);
    } else {
        let pair_required_json = serde_json::to_string(&PairRequired::new()).map_err(|e| e.to_string())?;
        write.send(Message::Text(pair_required_json.into())).await.map_err(|e| e.to_string())?;

        match tokio::time::timeout(PAIRING_TIMEOUT, run_pairing(&mut read, &state, &hello)).await {
            Ok(Ok(pairing)) => {
                let paired_json = serde_json::to_string(&Paired::new(state.agent_id.clone(), pairing.token)).map_err(|e| e.to_string())?;
                write.send(Message::Text(paired_json.into())).await.map_err(|e| e.to_string())?;
            }
            Ok(Err((code, message))) => {
                send_error_and_close(&mut write, code, &message).await;
                return Err(format!("pairing failed for {}: {message}", hello.device_id));
            }
            Err(_elapsed) => {
                send_error_and_close(&mut write, "pairing_expired", "Pairing code expired.").await;
                return Err(format!("pairing timed out for {}", hello.device_id));
            }
        }
    }

    let welcome = Welcome::new(state.agent_id.clone(), state.agent_name());
    let welcome_json = serde_json::to_string(&welcome).map_err(|e| e.to_string())?;
    write.send(Message::Text(welcome_json.into())).await.map_err(|e| e.to_string())?;

    let mut metrics_rx = state.metrics.subscribe();
    let mut ping_ticker = interval(PING_INTERVAL);

    loop {
        tokio::select! {
            changed = metrics_rx.changed() => {
                if changed.is_err() {
                    break; // sender (the collector task) is gone
                }
                let metrics = metrics_rx.borrow_and_update().clone();
                let message = MetricsMessage::new(metrics, now_ms());
                let json = serde_json::to_string(&message).map_err(|e| e.to_string())?;
                if write.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            _ = ping_ticker.tick() => {
                if write.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    // Pongs and anything else are ignored — no other
                    // inbound message types exist once a session is live.
                    _ => {}
                }
            }
        }
    }

    eprintln!("espia: device {} disconnected", hello.device_id);
    Ok(())
}

/// Runs one device's pairing exchange (protocol §4.2): reads its
/// `pair_request{code}`, asks a human to confirm it, and issues a token on
/// a match. The `Err` side is `(machine-readable code, human message)`,
/// matching protocol §4.3's `error` fields directly.
async fn run_pairing(
    read: &mut SplitStream<WebSocketStream<TcpStream>>,
    state: &Arc<SharedState>,
    hello: &Hello,
) -> Result<DevicePairing, (&'static str, String)> {
    let request_text = match read.next().await {
        Some(Ok(Message::Text(text))) => text,
        Some(Ok(_)) => return Err(("bad_message", "expected pair_request as text".to_string())),
        Some(Err(error)) => return Err(("bad_message", format!("error reading pair_request: {error}"))),
        None => return Err(("bad_message", "connection closed before pair_request".to_string())),
    };
    let pair_request: PairRequest = serde_json::from_str(&request_text).map_err(|e| ("bad_message", format!("could not parse pair_request: {e}")))?;
    if pair_request.kind != "pair_request" {
        return Err(("bad_message", format!("expected pair_request, got {}", pair_request.kind)));
    }

    let confirmation = pairing_ui::request_confirmation(state, &hello.device_id, &hello.board, &hello.firmware);
    let typed_code = confirmation.await.ok().flatten();

    let Some(typed_code) = typed_code else {
        return Err(("pairing_rejected", "Pairing was cancelled.".to_string()));
    };

    if !espia_core::security::constant_time_eq(typed_code.trim().as_bytes(), pair_request.code.trim().as_bytes()) {
        return Err(("pairing_rejected", "The pairing code did not match.".to_string()));
    }

    espia_core::pairing::add(&hello.device_id, &hello.board, &hello.firmware).map_err(|e| ("bad_message", e))
}

async fn send_error_and_close(
    write: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
    code: &'static str,
    message: &str,
) {
    let error = ErrorMessage::new(code, message.to_string());
    if let Ok(json) = serde_json::to_string(&error) {
        let _ = write.send(Message::Text(json.into())).await;
    }
    let _ = write.send(Message::Close(None)).await;
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
